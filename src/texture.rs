// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::slice;
use std::slice::SliceIndex;
use strum::VariantArray;

use crate::dimensions::Dimensions;

pub trait Block: Sized {
    type Bytes: AsRef<[u8]>;
    // = [u8; 8], etc. Many thanks to @kornel@mastodon.social
    const SIZE: usize;
    const WIDTH: usize = 4;
    const HEIGHT: usize = 4;

    fn to_bytes(&self) -> Self::Bytes;
    fn from_bytes(bytes: &Self::Bytes) -> Self;
}

struct BlockTexture<B>
    where
        B: Block,
{
    width: usize,
    height: usize,
    blocks: Vec<B>,
}

pub trait AsSlice<T> {
    fn as_slice(&self) -> &[T];
}

impl<T> AsSlice<T> for T {
    fn as_slice(&self) -> &[T] {
        slice::from_ref(self)
    }
}

impl<T> AsSlice<T> for &[T] {
    fn as_slice(&self) -> &[T] {
        self
    }
}

#[derive(Clone)]
pub struct Texture {
    buffers: TextureShapeNode<Rc<[u8]>>,
}

/// The face index of one face of a cubemap
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, VariantArray)]
#[repr(usize)]
pub enum CubemapFace {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

#[derive(Copy, Clone, Debug)]
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


/// An iterator for a TextureShape
/// Can iterate over faces, layers, or mips
pub struct TextureIterator<'a, T: TextureShape> {
    texture: &'a T,
    current: TextureIndex<usize>,
    len: usize,
}


impl<'a, T: TextureShape> Iterator for TextureIterator<'a, T> {
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

impl<'a, T: TextureShape> ExactSizeIterator for TextureIterator<'a, T> {}

/// A trait for a shaped texture, allowing slicing by face, layer, or mip.
pub trait TextureShape: Sized {
    fn get<I>(&self, index: TextureIndex<I>) -> Option<Self>
        where I: SliceIndex<[Self], Output: AsSlice<Self>> + Copy + Debug;

    fn get_face(&self, index: CubemapFace) -> Option<Self> {
        self.get::<usize>(TextureIndex::Face(index))
    }

    fn get_layer<I>(&self, index: I) -> Option<Self>
        where I: SliceIndex<[Self], Output: AsSlice<Self>> + Copy + Debug
    {
        self.get(TextureIndex::Layer(index))
    }

    fn get_mip<I>(&self, index: I) -> Option<Self>
        where I: SliceIndex<[Self], Output: AsSlice<Self>> + Copy + Debug,
    {
        self.get(TextureIndex::Mip(index))
    }
}

/// One node of a texture shape data structure
#[derive(Clone, Debug)]
pub enum TextureShapeNode<B: Sized + Clone> {
    Array(Vec<Self>),
    Cube(HashMap<CubemapFace, Self>),
    MipChain(Vec<Self>),
    Surface {
        dimensions: Dimensions,
        buffer: B,
    },
}


impl<'a, B: Clone> TextureShapeNode<B> {
    fn iter(&'a self) -> Box<dyn Iterator<Item=&'a Self> + 'a> {
        match self {
            TextureShapeNode::Array(v) => Box::new(v.iter()),
            TextureShapeNode::Cube(c) => Box::new(c.values()),
            TextureShapeNode::MipChain(m) => Box::new(m.iter()),
            _ => Box::new(slice::from_ref(self).iter()),
        }
    }

    fn mips(&self) -> Option<usize> {
        match self {
            TextureShapeNode::Surface { .. } => { None }
            TextureShapeNode::MipChain(v) => { Some(v.len()) }
            _ => self.iter().next().and_then(TextureShapeNode::mips)
        }
    }

    fn layers(&self) -> Option<usize> {
        match self {
            TextureShapeNode::Surface { .. } => { None }
            TextureShapeNode::Array(v) => { Some(v.len()) }
            _ => self.iter().next().and_then(TextureShapeNode::layers)
        }
    }


    fn faces(&self) -> Option<Vec<CubemapFace>> {
        match self {
            TextureShapeNode::Surface { .. } => { None }
            TextureShapeNode::Cube(faces) => { Some(faces.keys().cloned().collect()) }
            _ => self.iter().next().and_then(TextureShapeNode::faces)
        }
    }
}

impl<B: Clone> TextureShape for TextureShapeNode<B> {
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
}
