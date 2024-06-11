// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::fmt::Debug;
use std::iter::{repeat, zip};

use itertools::Itertools;
use strum::{Display, VariantArray};
use thiserror::Error;

use crate::dimensions::{Dimensioned, Dimensions};
use crate::shape::ShapeError::*;
use crate::util::AsSlice;

#[derive(Debug, Error)]
pub enum ShapeError {
    #[error("Non-uniform {0} in provided textures")]
    NonUniform(&'static str),

    #[error("Tried to form {0} out of textures that already have {0}s")]
    Nested(&'static str),

    #[error("Textures do not have dimensions that form a valid mipchain")]
    InvalidMipChain,

    #[error("Multiple textures provided for the same cubemap face")]
    DuplicateFaces,

    #[error("{0} cannot be empty")]
    Empty(&'static str),
}

pub type ShapeResult<T = ()> = Result<T, ShapeError>;

/// The face index of one face of a cubemap
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default, PartialOrd, Ord, VariantArray)]
#[repr(usize)]
pub enum CubeFace {
    #[default]
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

#[derive(Copy, Clone, Debug, Display)]
pub enum TextureIndex {
    Face(CubeFace),
    Mip(usize),
    Layer(usize),
}

impl TextureIndex {
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
                face_index %= CubeFace::VARIANTS.len();

                TextureIndex::Face(CubeFace::VARIANTS[face_index])
            }
            TextureIndex::Mip(m) => TextureIndex::Mip(m + 1),
            TextureIndex::Layer(l) => TextureIndex::Layer(l + 1),
        }
    }
}

struct TextureIterResult<S> {
    layer: Option<usize>,
    face: Option<CubeFace>,
    mip: Option<usize>,
    surface: S,
}

/// A trait for a shaped texture, allowing slicing by face, layer, or mip.
/// A texture is made up of multiple surfaces,
/// and can contain any combination of mipmaps, cubemaps, or array structures.
///
/// A TextureShape has several guarantees:
/// * A TextureShape contains at least one surface
/// * a TextureShape only contains exactly zero or one of a mipmap structure, a cubemap structure,
/// or an array structure. It is impossible to have an array of arrays or a cube of cubes.
/// * all the surfaces with the same mip have matching dimensions
/// * all the surfaces with mip\[i+1] have dimensions half that of mip\[i]
///
/// TextureShape is implemented with [`TextureShapeNode`],  made up of a tree structure.
/// All other types implementing this trait within this crate wrap that type.
pub trait TextureShape: Clone + Dimensioned {
    type Surface;

    /// Get a texture made of all the surfaces that match the passed index. If there are no matching
    /// surfaces, or the indexed structure is not present in the texture, this will return [`None`]
    fn get(&self, index: TextureIndex) -> Option<Self>;

    /// Get all array layers matching the given layer index
    /// If `self` does not contain an array structure, or no layers match the index, this will return [`None`]
    fn get_layer(&self, index: usize) -> Option<Self> {
        self.get(TextureIndex::Layer(index))
    }

    /// Get the cubemap face matching the given face index.
    /// If `self` does not contain a cube structure, or now faces match the index, this will return [`None`]
    fn get_face(&self, index: CubeFace) -> Option<Self> {
        self.get(TextureIndex::Face(index))
    }

    /// Get all mips matching the given mip index
    /// If `self` does not contain a mip structure, or no mips match the index, this will return [`None`]
    fn get_mip(&self, index: usize) -> Option<Self> {
        self.get(TextureIndex::Mip(index))
    }

    /// Try to create a new texture from an iterator of textures that represents a mipmap.
    /// Returns an error if any of the following are true:
    /// * iter contains no textures
    /// * any of the provided textures already has a mipmap
    /// * the provided textures do not have uniform faces or layers
    /// * the provided textures do not have dimensions matching a mipchain,
    /// where each mip has half the dimensions of the last. See [`Dimensions::mips`]
    fn try_from_mips<I: IntoIterator<Item = Self>>(iter: I) -> ShapeResult<Self>;

    /// Try to create a new texture from an iterator of textures that represents a cubemap.
    /// Returns an error if any of the following are true:
    /// * iter contains no textures
    /// * any of the provided textures already has a cubemap
    /// * the provided textures do not have uniform mips, layers, or dimensions
    /// * multiple textures are provided for the same cube face
    fn try_from_faces<I: IntoIterator<Item = (CubeFace, Self)>>(iter: I) -> ShapeResult<Self>;

    /// Try to create a new texture from an iterator of textures that represents an array
    /// Returns an error if any of the following are true:
    /// * iter contains no textures
    /// * any of the provided textures already has an array
    /// * the provided textures do not have uniform faces, mips, or dimensions
    fn try_from_layers<I: IntoIterator<Item = Self>>(iter: I) -> ShapeResult<Self>;

    /// Get the number of mips in the texture
    fn mips(&self) -> Option<usize>;

    /// Get the number of layers in the texture
    fn layers(&self) -> Option<usize>;

    /// Get a Vec of the cubemap faces in the texture
    fn faces(&self) -> Option<Vec<CubeFace>>;

    /// Returns an optional iterator over this texture's layers. if `self` does not contain an
    /// array structrue this returns [`None`]
    fn try_iter_layers(&self) -> Option<impl Iterator<Item = Self>> {
        Some((0..self.layers()?).map(|l| self.get_layer(l).unwrap()))
    }

    /// Returns an optional iterator over this texture's faces. If `self` does not contain
    /// a cubemap structure this returns [`None`]
    fn try_iter_faces(&self) -> Option<impl Iterator<Item = (CubeFace, Self)>> {
        Some(
            self.faces()?
                .into_iter()
                .map(|f| (f, self.get_face(f).unwrap())),
        )
    }

    /// Returns an iterator over this texture's mips, if a mipmap structure exists
    fn try_iter_mips(&self) -> Option<impl Iterator<Item = Self>> {
        Some((0..self.mips()?).map(|m| self.get_mip(m).unwrap()))
    }

    /// Iterate over the layers of the texture.
    /// If this texture has an array structure, returns each layer along with its index.
    /// Otherwise, this iterator returns a single item `(None, self.clone())`
    fn iter_layers(&self) -> impl Iterator<Item = (Option<usize>, Self)> {
        self.try_iter_layers()
            .into_iter()
            .flatten() // either an iterator over layers, or zero length
            .enumerate() // with layer indices
            .map(|(l, t)| (Some(l), t)) // transform the layer index into a Some, if any exist
            .pad_using(1, |_| (None, self.clone())) // ensure at least one item is returned
    }

    /// Iterate over the cubemap faces of the texture.
    /// If this texture has a cubemap structure, returns each face along with its index.
    /// Otherwise, this iterator returns a single item `(None, self.clone())`
    fn iter_faces(&self) -> impl Iterator<Item = (Option<CubeFace>, Self)> {
        self.try_iter_faces()
            .into_iter()
            .flatten() // either an iterator over faces, or zero length
            .map(|(f, t)| (Some(f), t)) // transform the face into a Some, if any exist
            .pad_using(1, |_| (None, self.clone())) // ensure at least one item is returned, but with no CubeMapFace
    }

    /// Iterate over the mips of the texture.
    /// If this texture has a mipmap structure, returns each mip along with its index.
    /// Otherwise, this iterator returns a single item `(None, self.clone())`
    fn iter_mips(&self) -> impl Iterator<Item = (Option<usize>, Self)> {
        self.try_iter_mips()
            .into_iter()
            .flatten() // either an iterator over mips, or zero length
            .enumerate() // with mip indices
            .map(|(m, t)| (Some(m), t)) // transform the mip index into a Some, if any exist
            .pad_using(1, |_| (None, self.clone())) // ensure at least one item is returned
    }

    /// Iterate over all the surfaces in the texture, returning the layer, face, and mip index for
    /// each one if present
    fn iter(&self) -> impl Iterator<Item = TextureIterResult<Self::Surface>> {
        let iter = self.iter_mips();

        let iter = iter
            .map(|(m, t): (Option<usize>, Self)| zip(repeat(m), t.iter_faces().collect_vec()))
            .flatten(); // add faces

        let iter = iter
            .map(|(m, (f, t)): (Option<usize>, (Option<CubeFace>, Self))| {
                zip(repeat((m, f)), t.iter_layers().collect_vec())
            })
            .flatten(); // add layers

        iter.map(
            |((m, f), (l, t)): ((Option<usize>, Option<CubeFace>), (Option<usize>, Self))| {
                TextureIterResult {
                    mip: m,
                    face: f,
                    layer: l,
                    surface: t.try_into_surface().unwrap(),
                }
            },
        )
    }

    /// Returns this texture as a single surface, if it only has one. Otherwise returns [`None`]
    fn try_into_surface(self) -> Option<Self::Surface>;

    /// Returns the number of surfaces present in the texture
    fn len(&self) -> usize {
        let len = self.mips().unwrap_or(1)
            * self.layers().unwrap_or(1)
            * self.faces().map_or(1, |c| c.len());

        assert!(len > 0);
        len
    }

    /// Returns if this texture represents a single surface
    fn is_surface(&self) -> bool {
        self.len() == 1
    }

    /// Returns the primary surface of the texture
    /// This is defined as layer 0, mip 0, and the first cubemap face present,
    /// if any, in order of the definition of [`CubeFace`]
    fn primary(&self) -> Self::Surface {
        let mut ret = if let Some(mut faces) = self.faces() {
            faces.sort();
            match &faces[..] {
                [first, ..] => self.get_face(*first).unwrap(),
                [] => {
                    panic!("Texture has cubemap but no faces")
                }
            }
        } else {
            self.clone()
        };

        ret = ret.get_layer(0).unwrap_or(ret);
        ret = ret.get_mip(0).unwrap_or(ret);

        return ret.try_into_surface().unwrap();
    }
}

/// One node of a texture shape data structure
#[derive(Clone, Debug)]
pub enum TextureShapeNode<S: Sized + Clone + Dimensioned> {
    /// A node representing a texture array
    Array(Vec<Self>),

    /// A node representing a cubemap
    CubeMap(HashMap<CubeFace, Self>),

    /// A node representing a mipmap
    MipMap(Vec<Self>),

    /// A node representing a single surface
    Surface(S),
}

impl<'a, S> TextureShapeNode<S>
where
    S: Clone + Dimensioned + 'a,
{
    /// Create a new texture with a single surface
    fn from_surface(surface: S) -> Self {
        Self::Surface(surface)
    }

    fn first_inner(&self) -> Self {
        match self {
            TextureShapeNode::Array(l) => l[0].clone(),
            TextureShapeNode::CubeMap(f) => {
                f.values().next().expect("Cubemap has no faces").clone()
            }
            TextureShapeNode::MipMap(m) => m[0].clone(),
            TextureShapeNode::Surface(_) => self.clone(),
        }
    }

    /// Check for uniformity of a closure result across an iterator
    fn uniformity_check<I, F, T>(iter: I, f: F, s: &'static str) -> ShapeResult
    where
        I: Iterator<Item = &'a Self>,
        F: FnMut(&Self) -> T,
        T: PartialEq,
    {
        if iter.map(f).all_equal() {
            Ok(())
        } else {
            Err(NonUniform(s))
        }
    }

    /// Check for nesting by iterating over textures and ensuring a closure returns [None]
    fn nesting_check<I, F, T>(iter: I, f: F, s: &'static str) -> ShapeResult
    where
        I: Iterator<Item = &'a Self>,
        F: FnMut(&Self) -> Option<T>,
        T: PartialEq,
    {
        if iter.map(f).flatten().next().is_some() {
            Err(Nested(s))
        } else {
            Ok(())
        }
    }
}

impl<S> Dimensioned for TextureShapeNode<S>
where
    S: Clone + Dimensioned,
{
    fn dimensions(&self) -> Dimensions {
        match self {
            TextureShapeNode::Surface(s) => s.dimensions(),
            _ => self.first_inner().dimensions(),
        }
    }
}

impl<S> TextureShape for TextureShapeNode<S>
where
    S: Clone + Dimensioned,
{
    type Surface = S;

    fn get(&self, index: TextureIndex) -> Option<Self> {
        return match (self, index) {
            (TextureShapeNode::Surface { .. }, _) => None, // target index was never found :(

            (TextureShapeNode::CubeMap(faces), TextureIndex::Face(f)) => {
                Some(faces.get(&f)?.clone())
            }
            (TextureShapeNode::CubeMap(faces), index) => Some(TextureShapeNode::CubeMap(
                faces
                    .iter()
                    .map(|(i, f)| Some((*i, f.get(index)?)))
                    .collect::<Option<_>>()?,
            )),

            (TextureShapeNode::MipMap(mips), TextureIndex::Mip(m)) => {
                let mips = mips.get(m)?.as_slice();
                match &mips[..] {
                    [single] => {
                        assert_eq!(single.mips(), None);
                        Some(single.clone())
                    }
                    [..] => Some(TextureShapeNode::MipMap(mips.into())),
                }
            }
            (TextureShapeNode::MipMap(mips), _) => Some(TextureShapeNode::MipMap(
                mips.iter().map(|t| t.get(index)).collect::<Option<_>>()?,
            )),

            (TextureShapeNode::Array(layers), TextureIndex::Layer(l)) => {
                let layers = layers.get(l)?.as_slice();
                match &layers[..] {
                    [single] => {
                        assert_eq!(single.layers(), None);
                        Some(single.clone())
                    }
                    [..] => Some(TextureShapeNode::Array(layers.into())),
                }
            }
            (TextureShapeNode::Array(layers), _) => Some(TextureShapeNode::Array(
                layers.iter().map(|t| t.get(index)).collect::<Option<_>>()?,
            )),
        };
    }

    fn try_from_mips<I: IntoIterator<Item = Self>>(iter: I) -> ShapeResult<Self> {
        let mips = iter.into_iter().collect_vec();

        // get dimensions of first mip, while also making sure len > 0
        let dimensions = mips.get(0).ok_or(Empty("mipmap"))?.dimensions();

        // check that dimensions follow the mip chain
        if !mips.iter().map(Self::dimensions).eq(dimensions.mips()) {
            return Err(InvalidMipChain);
        }

        // check for non-uniformity and nesting
        Self::uniformity_check(mips.iter(), Self::layers, "layers")?;
        Self::uniformity_check(mips.iter(), Self::faces, "faces")?;
        Self::nesting_check(mips.iter(), Self::mips, "mipmap")?;

        Ok(Self::MipMap(mips))
    }

    fn try_from_faces<I: IntoIterator<Item = (CubeFace, Self)>>(iter: I) -> ShapeResult<Self> {
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

        // check for non-uniformity and nesting
        Self::uniformity_check(faces.values(), Self::dimensions, "dimensions")?;
        Self::uniformity_check(faces.values(), Self::mips, "mips")?;
        Self::uniformity_check(faces.values(), Self::layers, "layers")?;
        Self::nesting_check(faces.values(), Self::faces, "cube")?;

        Ok(Self::CubeMap(faces))
    }

    fn try_from_layers<I: IntoIterator<Item = Self>>(iter: I) -> ShapeResult<Self> {
        let layers = iter.into_iter().collect_vec();

        // Check for emptiness
        if layers.len() == 0 {
            return Err(Empty("array"));
        }

        // check for non-uniformity and nesting
        Self::uniformity_check(layers.iter(), Self::dimensions, "dimensions")?;
        Self::uniformity_check(layers.iter(), Self::mips, "mips")?;
        Self::uniformity_check(layers.iter(), Self::faces, "faces")?;
        Self::nesting_check(layers.iter(), Self::layers, "array")?;

        Ok(Self::Array(layers))
    }

    fn mips(&self) -> Option<usize> {
        match self {
            TextureShapeNode::Surface { .. } => None,
            TextureShapeNode::MipMap(v) => Some(v.len()),
            _ => self.first_inner().mips(),
        }
    }

    fn layers(&self) -> Option<usize> {
        match self {
            TextureShapeNode::Surface { .. } => None,
            TextureShapeNode::Array(v) => Some(v.len()),
            _ => self.first_inner().layers(),
        }
    }

    fn faces(&self) -> Option<Vec<CubeFace>> {
        match self {
            TextureShapeNode::Surface { .. } => None,
            TextureShapeNode::CubeMap(faces) => Some(faces.keys().cloned().collect()),
            _ => self.first_inner().faces(),
        }
    }

    fn try_into_surface(self) -> Option<S> {
        match self {
            TextureShapeNode::Surface(s) => Some(s),
            _ => None,
        }
    }

    fn is_surface(&self) -> bool {
        match self {
            TextureShapeNode::Surface(_) => true,
            _ => false,
        }
    }
}
