// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Index;
use std::rc::Rc;
use std::slice;
use std::slice::SliceIndex;

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

trait AsSlice<T> {
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
    shape: TextureShape,
}

#[derive(Copy, Clone, Debug)]
pub enum TextureIndex<I: Sized + Clone + Debug = usize> {
    Face(CubemapFace),
    Mip(I),
    Layer(I),
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, strum::VariantArray)]
pub enum CubemapFace {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}


#[derive(Clone, Debug)]
pub enum TextureShape {
    Array(Vec<TextureShape>),
    Cube(HashMap<CubemapFace, TextureShape>),
    MipChain(Vec<TextureShape>),
    Image {
        dimensions: Dimensions,
        buffer: Rc<[u8]>,
    },
}

impl<'a> TextureShape {
    fn iter(&'a self) -> Box<dyn Iterator<Item=&'a TextureShape> + 'a> {
        match self {
            TextureShape::Array(v) => Box::new(v.iter()),
            TextureShape::Cube(c) => Box::new(c.values()),
            TextureShape::MipChain(m) => Box::new(m.iter()),
            _ => Box::new(slice::from_ref(self).iter()),
        }
    }

    fn mips(&self) -> Option<usize> {
        match self {
            TextureShape::Image { .. } => { None }
            TextureShape::MipChain(v) => { Some(v.len()) }
            _ => self.iter().next().and_then(TextureShape::mips)
        }
    }

    fn mip<I>(&self, index: I) -> TextureShape
        where I: SliceIndex<[TextureShape], Output: AsSlice<TextureShape>> + Copy + Debug,
    {
        self.index(TextureIndex::Mip(index))
    }

    fn layers(&self) -> Option<usize> {
        match self {
            TextureShape::Image { .. } => { None }
            TextureShape::Array(v) => { Some(v.len()) }
            _ => self.iter().next().and_then(TextureShape::layers)
        }
    }

    fn layer<I>(&self, index: I) -> TextureShape
        where I: SliceIndex<[TextureShape], Output: AsSlice<TextureShape>> + Copy + Debug,
    {
        self.index(TextureIndex::Layer(index))
    }

    fn faces(&self) -> Option<Vec<CubemapFace>> {
        match self {
            TextureShape::Image { .. } => { None }
            TextureShape::Cube(faces) => { Some(faces.keys().cloned().collect()) }
            _ => self.iter().next().and_then(TextureShape::faces)
        }
    }

    fn face(&self, index: CubemapFace) -> TextureShape {
        self.index::<usize>(TextureIndex::Face(index))
    }

    fn index<I>(&self, index: TextureIndex<I>) -> TextureShape
        where I: SliceIndex<[TextureShape], Output: AsSlice<TextureShape>> + Copy + Debug
    {
        return match (self, index) {
            (TextureShape::Image { .. }, _) => self.clone(),

            (TextureShape::Cube(faces), TextureIndex::Face(f)) => { faces[&f].clone() }
            (TextureShape::Cube(faces), index) => {
                TextureShape::Cube(faces.iter().map(|(i, f)| (*i, f.index(index))).collect())
            }

            (TextureShape::MipChain(mips), TextureIndex::Mip(m)) => {
                let mips: Vec<TextureShape> = mips[m].as_slice().into();
                match &mips[..] {
                    [single] => {
                        assert_eq!(single.mips(), None);
                        single.clone()
                    }
                    [..] => {
                        TextureShape::MipChain(mips)
                    }
                }
            }
            (TextureShape::MipChain(mips), _) => {
                TextureShape::MipChain(mips.iter().map(|t| t.index(index)).collect())
            }

            (TextureShape::Array(layers), TextureIndex::Layer(l)) => {
                let layers: Vec<TextureShape> = layers[l].as_slice().into();
                match &layers[..] {
                    [single] => {
                        assert_eq!(single.layers(), None);
                        single.clone()
                    }
                    [..] => TextureShape::Array(layers)
                }
            }
            (TextureShape::Array(layers), _) => {
                TextureShape::Array(layers.iter().map(|t| t.index(index)).collect())
            }
        };
    }
}
