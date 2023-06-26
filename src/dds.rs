// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::texture::{Texture, TextureList};
use arrayvec::ArrayString;
use itertools::Itertools;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::{Cursor, Read};

enum ColorFormat {
    RGB { bitcount: u32, bitmasks: [u32; 3] },
    YUV { bitcount: u32, bitmasks: [u32; 3] },
    L { bitcount: u32, bitmask: u32 },
    A { bitcount: u32, bitmask: u32 },
}

enum PixelFormat {
    Compressed {
        size: u32,
        four_cc: ArrayString<4>,
    },
    Uncompressed {
        pitch: u32,
        alpha_bitmask: Option<u32>,
        color_format: ColorFormat,
    },
    // todo: DX10 header option
}

struct DDSFile {
    width: u32,
    height: u32,
    depth: Option<u32>,
    mipmap_count: Option<u32>,
    pixel_format: PixelFormat,
    // todo: Find a way to store textures here
}

