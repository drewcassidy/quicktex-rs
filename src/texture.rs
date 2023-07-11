// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::format::{Format, ImageFormat};

use arrayvec::ArrayVec;

use crate::dimensions::Dimensions;
use std::rc::Rc;
use std::slice;
use std::slice::SliceIndex;

pub trait Block: Sized {
    type Bytes: AsRef<[u8]>; // = [u8; 8], etc. Many thanks to @kornel@mastodon.social
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

pub trait TextureList {
    fn len(&self) -> usize;
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
struct Texture {
    metadata: Format,
    dimensions: Dimensions,
    shape: TextureShape,
}

#[derive(Copy, Clone)]
enum TextureIndex {
    Face(CubemapFace),
    Mip(usize),
    Layer(usize),
}

#[derive(Copy, Clone)]
enum CubemapFace {
    PositiveX = 0,
    NegativeX = 1,
    PositiveY = 2,
    NegativeY = 3,
    PositiveZ = 4,
    NegativeZ = 5,
}

#[derive(Clone)]
enum TextureShape {
    Array(Vec<TextureShape>),
    Cube(Box<[TextureShape; 6]>),
    MipChain(Vec<TextureShape>),
    Image(Rc<[u8]>),
}

impl TextureShape {
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut TextureShape> {
        match self {
            TextureShape::Array(v) => v.iter_mut(),
            TextureShape::Cube(c) => c.iter_mut(),
            TextureShape::MipChain(m) => m.iter_mut(),
            TextureShape::Image(_) => std::slice::from_mut(self).iter_mut(),
        }
    }

    fn iter(&self) -> impl Iterator<Item = &TextureShape> {
        match self {
            TextureShape::Array(v) => v.iter(),
            TextureShape::Cube(c) => c.iter(),
            TextureShape::MipChain(m) => m.iter(),
            TextureShape::Image(_) => std::slice::from_ref(self).iter(),
        }
    }

    fn mip<I>(&self, index: I) -> TextureShape
    where
        I: SliceIndex<[TextureShape]> + Copy,
        <I as SliceIndex<[TextureShape]>>::Output: AsSlice<TextureShape>,
    {
        return match self {
            TextureShape::MipChain(mips) => TextureShape::MipChain(mips[index].as_slice().into()),
            TextureShape::Image(_) => std::slice::from_ref(self)[index].as_slice()[0].clone(),
            _ => {
                let mut res = self.clone();
                res.iter_mut().for_each(|t| *t = t.mip(index));
                res
            }
        };
    }

    fn layer<I>(&self, index: I) -> TextureShape
    where
        I: SliceIndex<[TextureShape]> + Copy,
        <I as SliceIndex<[TextureShape]>>::Output: AsSlice<TextureShape>,
    {
        return match self {
            TextureShape::Array(layers) => TextureShape::Array(layers[index].as_slice().into()),
            TextureShape::Image(_) => std::slice::from_ref(self)[index].as_slice()[0].clone(),
            _ => {
                let mut res = self.clone();
                res.iter_mut().for_each(|t| *t = t.mip(index));
                res
            }
        };
    }

    fn face(&self, index: CubemapFace) -> TextureShape {
        // maybe check shape here?
        return match self {
            TextureShape::Cube(layers) => layers[index as usize].clone(),
            TextureShape::Image(_) => self.clone(),
            _ => {
                let mut res = self.clone();
                res.iter_mut().for_each(|t| *t = t.face(index));
                res
            }
        };
    }

    fn slice(&self, index: TextureIndex) -> TextureShape {
        match index {
            TextureIndex::Face(f) => self.face(f),
            TextureIndex::Mip(m) => self.mip(m),
            TextureIndex::Layer(l) => self.layer(l),
        }
    }
}
