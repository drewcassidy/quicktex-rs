// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;
use std::io::{Read, Seek};
use std::rc::Rc;
use std::slice::SliceIndex;
use image::codecs::png::CompressionType::Default;
use itertools::Itertools;
use crate::dimensions::{Dimensioned, Dimensions};
use crate::format::Format;
use crate::shape::{CubeFace, ShapeError, ShapeResult, TextureIndex, TextureShape, TextureShapeNode};
use crate::util::AsSlice;

#[derive(Clone, Debug)]
pub struct Surface {
    dimensions: Dimensions,
    buffer: Rc<[u8]>,
}


impl Dimensioned for Surface {
    fn dimensions(&self) -> Dimensions { self.dimensions }
}

#[derive(Clone, Debug)]
pub struct Texture {
    format: Format,
    surfaces: TextureShapeNode<Surface>,
}

impl Texture {
    pub fn read_surface<T>(reader: &mut T, dimensions: Dimensions, format: Format) -> Result<Self, std::io::Error>
        where T: Read + Sized
    {
        let size = format.size_for(dimensions);
        let mut buffer: Vec<u8> = vec![0; size];
        reader.read_exact(&mut buffer[..])?; // read into the vec buffer
        let buffer = Rc::<[u8]>::from(buffer); // move buffer contents into an RC without copying

        let surfaces = TextureShapeNode::Surface(Surface { dimensions, buffer });

        return Ok(Texture { format, surfaces });
    }
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