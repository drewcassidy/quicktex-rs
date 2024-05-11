// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::fmt::Debug;
use std::slice::SliceIndex;

use itertools::Itertools;
use strum::{Display, VariantArray};
use thiserror::Error;

use crate::dimensions::{Dimensioned, Dimensions};
use crate::shape::ShapeError::{DuplicateFaces, Empty, NonUniformDimensions};
use crate::util::AsSlice;

#[derive(Debug, Error)]
pub enum ShapeError {
    #[error("Nonuniform {0} counts in provided textures")]
    NonUniformType(&'static str),

    #[error("Tried to form {0} out of textures that already have {0}s")]
    Nested(&'static str),

    #[error("Nonuniform dimensions in provided textures")]
    NonUniformDimensions,

    #[error("Textures do not have dimensions that form a valid mipchain")]
    InvalidMipChain,

    #[error("Multiple textures provided for the same cubemap face")]
    DuplicateFaces,

    #[error("{0} cannot be empty")]
    Empty(&'static str),
}

type ShapeResult<T = ()> = Result<T, ShapeError>;

/// The face index of one face of a cubemap
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default, VariantArray)]
#[repr(usize)]
pub enum CubemapFace {
    #[default]
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

#[derive(Copy, Clone, Debug, Display)]
pub enum TextureIndex<I: Sized + Clone + Debug = usize> {
    Face(CubemapFace),
    Mip(I),
    Layer(I),
}

impl TextureIndex<usize> {
    /// Return the next index
    ///
    /// For mips and faces, this is just the index + 1.
    ///
    /// For faces, this is the next face in the `CubemapFace` enum, wrapping around to `PositiveX`
    /// when it reaches the end
    fn next(&self) -> TextureIndex {
        match self {
            TextureIndex::Face(f) => {
                let mut face_index = *f as usize;
                face_index += 1;
                face_index %= CubemapFace::VARIANTS.len();

                TextureIndex::Face(
                    CubemapFace::VARIANTS[face_index]
                )
            }
            TextureIndex::Mip(m) => { TextureIndex::Mip(m + 1) }
            TextureIndex::Layer(l) => { TextureIndex::Layer(l + 1) }
        }
    }
}


impl<T: TextureShape> ExactSizeIterator for TextureIterator<T> {}

/// A trait for a shaped texture, allowing slicing by face, layer, or mip.
/// A TextureShape is made up of multiple surfaces,
/// and can contain any combination of mipmaps, cubemaps, or array structures.
///
/// A TextureShape has several guarantees:
/// * A TextureShape contains at least one surface
/// * a TextureShape only contains exactly zero or one of a mipmap structure, a cubemap structure,
/// or an array structure. It is impossible to have an array of arrays or a cube of cubes.
/// * all the surfaces with the same mip have matching dimensions
/// * all the surfaces with mip\[i+1] have dimensions half that of mip\[i]
///
/// TextureShape is implemented with `TextureShapeNode`,  made up of a tree structure.
/// All other types implementing this trait within this crate wrap that type. 
pub trait TextureShape: Clone + Dimensioned {
    type Surface;

    fn get<I>(&self, index: TextureIndex<I>) -> Option<Self>
        where I: SliceIndex<[Self], Output: AsSlice<Self>> + Copy + Debug;

    /// Get all cubemap faces matching the given face index.
    /// If `self` does not contain a cube structure, this will return a clone of `self`
    fn get_face(&self, index: CubemapFace) -> Option<Self> {
        self.get::<usize>(TextureIndex::Face(index))
    }

    /// Get all array layers matching the given index or range.
    /// If `self` does not contain an array structure, this will return a clone of `self`
    fn get_layer<I>(&self, index: I) -> Option<Self>
        where I: SliceIndex<[Self], Output: AsSlice<Self>> + Copy + Debug
    {
        self.get(TextureIndex::Layer(index))
    }

    /// Get all mips matching the given index or range.
    /// If `self` does not contain a mip structure, this will return a clone of `self`
    fn get_mip<I>(&self, index: I) -> Option<Self>
        where I: SliceIndex<[Self], Output: AsSlice<Self>> + Copy + Debug,
    {
        self.get(TextureIndex::Mip(index))
    }


    /// Try to create a new TextureShape from an iterator of surfaces that represents a mip chain.
    /// Returns an error if any of the following are true:
    /// * iter contains no textures
    /// * any of the provided textures already has a mipmap
    /// * the provided textures do not all have uniform faces, layers, or dimensions
    /// * the provided textures do not have dimensions matching a mipchain, 
    /// where each mip has half the dimensions of the last. See `Dimensions::
    fn try_from_mips<I: IntoIterator<Item=Self>>(iter: I) -> ShapeResult<Self>;

    fn try_from_faces<I: IntoIterator<Item=(CubemapFace, Self)>>(iter: I) -> ShapeResult<Self>;

    fn try_from_layers<I: IntoIterator<Item=Self>>(iter: I) -> ShapeResult<Self>;

    fn from_surface(surface: Self::Surface) -> Self;
    fn mips(&self) -> Option<usize>;
    fn layers(&self) -> Option<usize>;
    fn faces(&self) -> Option<Vec<CubemapFace>>;
}

/// An iterator for a TextureShape
/// Can iterate over faces, layers, or mips
pub struct TextureIterator<T: TextureShape> {
    texture: T,
    current: TextureIndex<usize>,
    len: usize,
}


impl<T: TextureShape> Iterator for TextureIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        while self.len > 0 {
            let next = self.texture.get(self.current);
            if next.is_some() { return next; }

            self.current = self.current.next();
            self.len -= 1;
        }
        return None;
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}


/// One node of a texture shape data structure
#[derive(Clone, Debug)]
pub(crate) enum TextureShapeNode<S: Sized + Clone + Dimensioned> {
    Array(Vec<Self>),
    Cube(HashMap<CubemapFace, Self>),
    MipChain(Vec<Self>),
    Surface(S),
}


impl<'a, S> TextureShapeNode<S> where S: Clone + Dimensioned + 'a {
    fn first_inner(&self) -> Self {
        match self {
            TextureShapeNode::Array(l) => { l[0].clone() }
            TextureShapeNode::Cube(f) => { f.values().next().expect("Cubemap has no faces").clone() }
            TextureShapeNode::MipChain(m) => { m[0].clone() }
            TextureShapeNode::Surface(_) => { self.clone() }
        }
    }

    fn uniformity_check<I, F, T>(iter: I, f: F, s: &'static str) -> ShapeResult
        where I: Iterator<Item=&'a Self>,
              F: FnMut(&Self) -> Option<T>,
              T: PartialEq {
        if iter.map(f).all_equal() {
            Err(ShapeError::NonUniformType(s))
        } else {
            Ok(())
        }
    }

    fn nesting_check<I, F, T>(iter: I, f: F, s: &'static str) -> ShapeResult
        where I: Iterator<Item=&'a Self>,
              F: FnMut(&Self) -> Option<T>,
              T: PartialEq {
        if iter.map(f).flatten().next().is_some() {
            Err(ShapeError::Nested(s))
        } else {
            Ok(())
        }
    }
}


impl<S> Dimensioned for TextureShapeNode<S> where S: Clone + Dimensioned {
    fn dimensions(&self) -> Dimensions {
        match self {
            TextureShapeNode::Surface(s) => { s.dimensions() }
            _ => self.first_inner().dimensions()
        }
    }
}

impl<S> TextureShape for TextureShapeNode<S> where S: Clone + Dimensioned {
    type Surface = S;

    fn get<I>(&self, index: TextureIndex<I>) -> Option<Self>
        where I: SliceIndex<[Self], Output: AsSlice<Self>> + Copy + Debug
    {
        return match (self, index) {
            (TextureShapeNode::Surface { .. }, _) => Some(self.clone()),

            (TextureShapeNode::Cube(faces), TextureIndex::Face(f)) => { Some(faces.get(&f)?.clone()) }
            (TextureShapeNode::Cube(faces), index) => {
                Some(TextureShapeNode::Cube(
                    faces.iter()
                        .map(|(i, f)| Some((*i, f.get(index)?)))
                        .collect::<Option<_>>()?
                ))
            }

            (TextureShapeNode::MipChain(mips), TextureIndex::Mip(m)) => {
                let mips = mips.get(m)?.as_slice();
                match &mips[..] {
                    [single] => {
                        assert_eq!(single.mips(), None);
                        Some(single.clone())
                    }
                    [..] => {
                        Some(TextureShapeNode::MipChain(mips.into()))
                    }
                }
            }
            (TextureShapeNode::MipChain(mips), _) => {
                Some(TextureShapeNode::MipChain(
                    mips.iter()
                        .map(|t| t.get(index))
                        .collect::<Option<_>>()?
                ))
            }

            (TextureShapeNode::Array(layers), TextureIndex::Layer(l)) => {
                let layers = layers.get(l)?.as_slice();
                match &layers[..] {
                    [single] => {
                        assert_eq!(single.layers(), None);
                        Some(single.clone())
                    }
                    [..] => {
                        Some(TextureShapeNode::Array(layers.into()))
                    }
                }
            }
            (TextureShapeNode::Array(layers), _) => {
                Some(TextureShapeNode::Array(
                    layers.iter()
                        .map(|t| t.get(index))
                        .collect::<Option<_>>()?
                ))
            }
        };
    }

    fn mips(&self) -> Option<usize> {
        match self {
            TextureShapeNode::Surface { .. } => { None }
            TextureShapeNode::MipChain(v) => { Some(v.len()) }
            _ => self.first_inner().mips()
        }
    }

    fn layers(&self) -> Option<usize> {
        match self {
            TextureShapeNode::Surface { .. } => { None }
            TextureShapeNode::Array(v) => { Some(v.len()) }
            _ => self.first_inner().layers()
        }
    }


    fn faces(&self) -> Option<Vec<CubemapFace>> {
        match self {
            TextureShapeNode::Surface { .. } => { None }
            TextureShapeNode::Cube(faces) => { Some(faces.keys().cloned().collect()) }
            _ => self.first_inner().faces()
        }
    }

    fn try_from_mips<I: IntoIterator<Item=Self>>(iter: I) -> ShapeResult<Self> {
        let mips = iter.into_iter().collect_vec();

        // get dimensions of first mip, while also making sure len > 0
        let dimensions = mips.get(0)
            .ok_or(ShapeError::Empty("mipchain"))?
            .dimensions();

        // check that dimensions follow the mip chain
        if !mips.iter().map(Self::dimensions).eq(dimensions.mips()) {
            return Err(ShapeError::InvalidMipChain);
        }

        // check for non-uniformity and nesting
        Self::uniformity_check(mips.iter(), Self::layers, "layers")?;
        Self::uniformity_check(mips.iter(), Self::faces, "faces")?;
        Self::nesting_check(mips.iter(), Self::mips, "mipchain")?;

        Ok(Self::MipChain(mips))
    }

    fn try_from_faces<I: IntoIterator<Item=(CubemapFace, Self)>>(iter: I) -> ShapeResult<Self> {
        let mut faces = HashMap::new();

        // add faces and check for duplicates
        for (face, t) in iter {
            if let Some(_) = faces.insert(face, t) {
                return Err(DuplicateFaces);
            }
        }

        // Check for emptiness
        if faces.len() == 0 {
            return Err(Empty("cube"));
        }

        // Check all faces have the same dimensions
        faces.values()
            .map(Self::dimensions)
            .all_equal_value()
            .or(Err(NonUniformDimensions))?;

        // check for non-uniformity and nesting
        Self::uniformity_check(faces.values(), Self::mips, "mips")?;
        Self::uniformity_check(faces.values(), Self::layers, "layers")?;
        Self::nesting_check(faces.values(), Self::faces, "cube")?;

        Ok(Self::Cube(faces))
    }

    fn try_from_layers<I: IntoIterator<Item=Self>>(iter: I) -> ShapeResult<Self> {
        let layers = iter.into_iter().collect_vec();

        // Check for emptiness
        if layers.len() == 0 {
            return Err(Empty("array"));
        }

        // Check all faces have the same dimensions
        layers.iter()
            .map(Self::dimensions)
            .all_equal_value()
            .or(Err(NonUniformDimensions))?;

        // check for non-uniformity and nesting
        Self::uniformity_check(layers.iter(), Self::mips, "mips")?;
        Self::uniformity_check(layers.iter(), Self::faces, "faces")?;
        Self::nesting_check(layers.iter(), Self::layers, "array")?;

        Ok(Self::Array(layers))
    }

    fn from_surface(surface: S) -> Self {
        Self::Surface(surface)
    }
}

