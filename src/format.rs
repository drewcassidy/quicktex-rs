// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;
use crate::dimensions::Dimensions;


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlphaFormat {
    /// Any alpha channel content is being used as a 4th channel
    /// and is not intended to represent transparency (straight or premultiplied).
    /// This is the default for unknown alpha channel types.
    Custom { alpha_mask: u32 },

    /// Any alpha channel content is presumed to use straight alpha.
    Straight { alpha_mask: u32 },

    /// Any alpha channel content is using premultiplied alpha.
    Premultiplied { alpha_mask: u32 },

    /// Any alpha channel content is all set to fully opaque.
    Opaque,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorFormat {
    /// RGB color channels
    RGB {
        r_mask: u32,
        g_mask: u32,
        b_mask: u32,
        srgb: bool,
    },

    /// YUV color channels
    YUV {
        y_mask: u32,
        u_mask: u32,
        v_mask: u32,
    },

    /// Luminance-only color channels
    L {
        l_mask: u32,
    },

    /// No color information, e.g. alpha only
    None,
}


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    BC1 { srgb: bool },
    BC2 { srgb: bool },
    BC3 { srgb: bool },
    BC4 { signed: bool },
    BC5 { signed: bool },
    Uncompressed {
        pitch: usize,
        color_format: ColorFormat,
        alpha_format: AlphaFormat,
    },
    // Not yet supported, but might be in the future:
    // * ASTC, ETC, BC7
    // * Basis and other super compression schemes (would contain a boxed format for the inner)
    // * Video formats like YUV 4:2:2, but I don't think anyone actually uses these.
    // UNORM/UINT/SNORM/SINT/FLOAT? even if its just for round trip
}

impl Format {
    pub fn size_for(&self, dimensions: Dimensions) -> usize {
        use Format::*;
        match self {
            BC1 { .. } | BC4 { .. } => {
                8 * dimensions.blocks(Dimensions::try_from([4, 4]).unwrap()).product() as usize
            }
            BC2 { .. } | BC3 { .. } | BC5 { .. } => {
                16 * dimensions.blocks(Dimensions::try_from([4, 4]).unwrap()).product() as usize
            }
            Uncompressed { pitch, .. } => {
                *pitch * dimensions.product() as usize
            }
        }
    }
}
