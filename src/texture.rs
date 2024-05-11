// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;
use std::rc::Rc;
use crate::dimensions::{Dimensioned, Dimensions};
use crate::shape::TextureShapeNode;

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

#[derive(Clone)]
pub struct Surface {
    dimensions: Dimensions,
    buffer: Rc<[u8]>,
}

impl Dimensioned for Surface {
    fn dimensions(&self) -> Dimensions { self.dimensions }
}

#[derive(Clone)]
pub struct Texture {
    surfaces: TextureShapeNode<Surface>,
}
