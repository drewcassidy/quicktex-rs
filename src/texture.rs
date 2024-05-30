// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::{Debug, Formatter};
use std::io::Read;
use std::rc::Rc;

use itertools::Itertools;
use thiserror::Error;

use crate::dimensions::{Dimensioned, Dimensions};
use crate::format::Format;
use crate::shape::{CubeFace, ShapeError, TextureIndex, TextureShape, TextureShapeNode};

/// A single surface of a [`Texture`], consisting of dimensions and a buffer of bytes
#[derive(Clone)]
pub struct Surface {
    dimensions: Dimensions,
    buffer: Rc<[u8]>,
}

impl Debug for Surface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{:?} surface with {} bytes", self.dimensions, self.buffer.len()).as_str())
    }
}


impl Dimensioned for Surface {
    fn dimensions(&self) -> Dimensions { self.dimensions }
}


/// Struct to simplify reading a texture from a file
pub struct TextureReader<'a, R: Read> {
    pub format: Format,
    pub reader: &'a mut R,
}

#[derive(Error, Debug)]
pub enum TextureError {
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Shape Error: {0}")]
    Shape(#[from] ShapeError),
}

type TextureResult<T = Texture> = Result<T, TextureError>;

impl<'a, R: Read> TextureReader<'a, R> {
    /// Read a single surface from a binary reader using the given dimensions
    pub fn read_surface(&mut self, dimensions: Dimensions) -> TextureResult
    {
        let size = self.format.size_for(dimensions);
        let mut buffer: Vec<u8> = vec![0; size];
        self.reader.read_exact(&mut buffer[..])?; // read into the vec buffer
        let buffer = Rc::<[u8]>::from(buffer); // move buffer contents into an RC without copying

        let surfaces = TextureShapeNode::Surface(Surface { dimensions, buffer });

        return Ok(Texture { format: self.format, surfaces });
    }

    /// Construct a mipmap out of the textures produced by `inner`, or short circuit to `inner` if `mip_count` is [`None`]
    pub fn read_mips<F>(&mut self, dimensions: Dimensions, mip_count: Option<usize>, mut inner: F) -> TextureResult
        where F: FnMut(&mut Self, Dimensions) -> TextureResult
    {
        if let Some(mip_count) = mip_count {
            let textures = dimensions.mips().take(mip_count)
                .map(|d| inner(self, d))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Texture::try_from_mips(textures)?)
        } else {
            inner(self, dimensions)
        }
    }

    /// Construct a cubemap out of the textures produced by `inner`, or short circuit to `inner` if `faces` is [`None`]
    pub fn read_faces<F>(&mut self, dimensions: Dimensions, faces: Option<Vec<CubeFace>>, mut inner: F) -> TextureResult
        where F: FnMut(&mut Self, Dimensions) -> TextureResult
    {
        if let Some(faces) = faces {
            let textures = faces.into_iter()
                .map(|f| -> TextureResult<_> {
                    Ok((f, inner(self, dimensions)?))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Texture::try_from_faces(textures)?)
        } else {
            inner(self, dimensions)
        }
    }

    /// Construct an array out of the textures produced by `inner`, or short circuit to `inner` if `layer_count` is [`None`]
    pub fn read_layers<F>(&mut self, dimensions: Dimensions, layer_count: Option<usize>, mut inner: F) -> TextureResult
        where F: FnMut(&mut Self, Dimensions) -> TextureResult
    {
        if let Some(layer_count) = layer_count {
            let textures = (0..layer_count)
                .map(|_| inner(self, dimensions))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Texture::try_from_layers(textures)?)
        } else {
            inner(self, dimensions)
        }
    }
}

/// An encoded texture, consisting of a [`Format`] and one or more [`Surface`]s
#[derive(Clone, Debug)]
pub struct Texture {
    format: Format,
    surfaces: TextureShapeNode<Surface>,
}

impl Dimensioned for Texture {
    fn dimensions(&self) -> Dimensions { self.surfaces.dimensions() }
}

impl TextureShape for Texture {
    type Surface = Surface;

    fn get(&self, index: TextureIndex) -> Option<Self> {
        Some(Self {
            surfaces: self.surfaces.get(index)?,
            format: self.format,
        })
    }


    fn try_from_mips<I: IntoIterator<Item=Self>>(iter: I) -> crate::shape::ShapeResult<Self> {
        let (formats, nodes): (Vec<_>, Vec<_>) = iter.into_iter().map(|t| (t.format, t.surfaces)).unzip();
        let format = formats.iter().all_equal_value().or(Err(ShapeError::NonUniform("format")))?;
        Ok(Self {
            surfaces: TextureShapeNode::try_from_mips(nodes)?,
            format: *format,
        })
    }

    fn try_from_faces<I: IntoIterator<Item=(CubeFace, Self)>>(iter: I) -> crate::shape::ShapeResult<Self> {
        let (formats, nodes): (Vec<_>, Vec<_>) = iter.into_iter().map(|(f, t)| (t.format, (f, t.surfaces))).unzip();
        let format = formats.iter().all_equal_value().or(Err(ShapeError::NonUniform("format")))?;
        Ok(Self {
            surfaces: TextureShapeNode::try_from_faces(nodes)?,
            format: *format,
        })
    }

    fn try_from_layers<I: IntoIterator<Item=Self>>(iter: I) -> crate::shape::ShapeResult<Self> {
        let (formats, nodes): (Vec<_>, Vec<_>) = iter.into_iter().map(|t| (t.format, t.surfaces)).unzip();
        let format = formats.iter().all_equal_value().or(Err(ShapeError::NonUniform("format")))?;
        Ok(Self {
            surfaces: TextureShapeNode::try_from_layers(nodes)?,
            format: *format,
        })
    }

    fn mips(&self) -> Option<usize> { self.surfaces.mips() }

    fn layers(&self) -> Option<usize> { self.surfaces.layers() }

    fn faces(&self) -> Option<Vec<CubeFace>> { self.surfaces.faces() }

    fn try_into_surface(self) -> Option<Self::Surface> { self.surfaces.try_into_surface() }
}