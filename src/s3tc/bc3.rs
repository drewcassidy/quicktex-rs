// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::blocktexture::Block;
use crate::s3tc::bc1::BC1Block;
use crate::s3tc::bc4::BC4Block;

pub struct BC3Block(BC1Block, BC4Block);

impl Block for BC3Block {
    type Bytes = [u8; 16];
    const SIZE: usize = 16;

    //noinspection DuplicatedCode
    fn to_bytes(&self) -> Self::Bytes {
        let mut bytes: Self::Bytes = [0; 16];
        bytes[0..8].copy_from_slice(&self.0.to_bytes()[..]); // BC1 RGB
        bytes[8..16].copy_from_slice(&self.1.to_bytes()[..]); // BC4 Alpha

        bytes
    }

    fn from_bytes(bytes: &Self::Bytes) -> Self {
        Self(
            BC1Block::from_bytes(&<[u8; 8]>::try_from(&bytes[0..8]).unwrap()), // BC1 RGB
            BC4Block::from_bytes(&<[u8; 8]>::try_from(&bytes[8..16]).unwrap()), // BC4 Alpha
        )
    }
}
