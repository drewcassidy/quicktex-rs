// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::dimensions::Dimensions;

pub mod bc1;
pub mod bc3;
pub mod bc4;
pub mod bc5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum S3TCFormat {
    BC1 { srgb: bool },
    BC2 { srgb: bool },
    BC3 { srgb: bool },
    BC4 { signed: bool },
    BC5 { signed: bool },
}

impl S3TCFormat {
    pub fn size_for(&self, dimensions: Dimensions) -> usize {
        use S3TCFormat::*;
        let blocks_width = (dimensions.width() + 3) / 4;
        let blocks_height = (dimensions.height() + 3) / 4;
        let blocks = blocks_height * blocks_width;
        return blocks * match self {
            BC1 { .. } | BC4 { .. } => { 8 }
            BC2 { .. } | BC3 { .. } | BC5 { .. } => { 16 }
        };
    }
}