// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::iter::zip;

use bitvec::prelude::*;
use vector_victor::Matrix;

use crate::blocktexture::Block;
use crate::color::{ColorImpl, RGBA};
use crate::pack::{Pack, Unpack};

mod decode;
mod encode;

#[derive(Copy, Clone)]
pub struct BC1Block {
    colors: [RGBA; 2],
    codes: Matrix<u8, 4, 4>,
}

impl Block for BC1Block {
    type Bytes = [u8; 8];
    const SIZE: usize = 8;

    fn to_bytes(&self) -> Self::Bytes {
        let mut bytes: Self::Bytes = [0; 8];
        let bits = bytes.view_bits_mut::<Msb0>();

        // store endpoints
        bits[0..16].store_le(self.colors[0].to_565());
        bits[16..32].store_le(self.colors[1].to_565());

        // store codes
        zip(self.codes.rows(), bits[32..].chunks_mut(8))
            // pack each row of 2-bit values into reversed chunks
            .for_each(|(src, dst)| dst.rchunks_mut(2).pack_be(src));

        bytes
    }

    fn from_bytes(bytes: &Self::Bytes) -> Self {
        let bits = bytes.as_bits::<Msb0>();

        // load endpoints
        let color0 = RGBA::from_565(bits[0..16].load_le());
        let color1 = RGBA::from_565(bits[16..32].load_le());

        // load codes
        let codes = Matrix::<u8, 4, 4>::from_rows(
            // reverse each row of 2-bit numbers and collect them to a Vector
            bits.chunks(8).map(|r| r.rchunks(2).unpack_le().collect()),
        );

        Self {
            colors: [color0, color1],
            codes,
        }
    }
}
