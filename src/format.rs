// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::dimensions::Dimensions;
use crate::s3tc::S3TCFormat;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AlphaFormat {
    /// Any alpha channel content is being used as a 4th channel
    /// and is not intended to represent transparency (straight or premultiplied).
    /// This is the default for unknown alpha channel types.
    Custom { bitmask: u32 },

    /// Any alpha channel content is presumed to use straight alpha.
    Straight { bitmask: u32 },

    /// Any alpha channel content is using premultiplied alpha.
    Premultiplied { bitmask: u32 },

    /// Any alpha channel content is all set to fully opaque.
    Opaque,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorFormat {
    /// RGB color channels
    RGB {
        bitmasks: [u32; 3],
        srgb: bool,
    },

    /// YUV color channels
    YUV {
        bitmasks: [u32; 3],
    },

    /// Luminance-only color channels
    L {
        bitmask: u32,
    },

    /// No color information, e.g. alpha only
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    S3TC(S3TCFormat),
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
        match self {
            Format::S3TC(s) => { s.size_for(dimensions) }
            Format::Uncompressed { pitch, .. } => { *pitch * dimensions.pixels() }
        }
    }
}
