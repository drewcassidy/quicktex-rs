// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use miette::{bail, IntoDiagnostic, Result};

use crate::util::ReadExt;
use arrayvec::ArrayString;
use enumflags2::{bitflags, BitFlags};

use itertools::Itertools;
use std::fmt::Debug;
use std::io::SeekFrom::Current;
use std::io::{Read, Seek};

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

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DDSFlags {
    Caps = 0x1,
    Height = 0x2,
    Width = 0x4,
    Pitch = 0x8,
    PixelFormat = 0x1000,
    MipmapCount = 0x20000,
    LinearSize = 0x80000,
    Depth = 0x800000,
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DDSCaps {
    Complex = 0x8,
    Mipmap = 0x400000,
    Texture = 0x1000,
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DDSCaps2 {
    Cubemap = 0x200,
    CubemapPositiveX = 0x400,
    CubemapNegativeX = 0x800,
    CubemapPositiveY = 0x1000,
    CubemapNegativeY = 0x2000,
    CubemapPositiveZ = 0x4000,
    CubemapNegativeZ = 0x8000,
    Volume = 0x200000,
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DDSPixelFormatFlags {
    AlphaPixels = 0x1,
    Alpha = 0x2,
    FourCC = 0x4,
    RGB = 0x40,
    YUV = 0x200,
    Luminance = 0x20000,
}

#[derive(Debug)]
struct DDSPixelFormat {
    flags: BitFlags<DDSPixelFormatFlags>,
    four_cc: ArrayString<4>,
    rgb_bit_count: u32,
    r_bit_mask: u32,
    g_bit_mask: u32,
    b_bit_mask: u32,
    a_bit_mask: u32,
}

impl DDSPixelFormat {
    fn new<R: Read + Seek>(mut reader: R) -> Result<Self> {
        const SIZE: u32 = 32;

        let start = reader.stream_position().into_diagnostic()?;

        let size: u32 = reader.load_le()?;
        if size != SIZE {
            bail!(
                "Invalid pixel format size: found {0} should be {1}",
                size,
                SIZE
            )
        }

        let flags: u32 = reader.load_le()?;
        let flags = BitFlags::<DDSPixelFormatFlags>::from_bits(flags).into_diagnostic()?;

        let four_cc = ArrayString::<4>::from_byte_string(&reader.load_array_le::<u8, 4>()?)
            .into_diagnostic()?;

        let [rgb_bit_count, r_bit_mask, g_bit_mask, b_bit_mask, a_bit_mask] =
            reader.load_array_le::<u32, 5>()?;

        let end = reader.stream_position().into_diagnostic()?;
        assert_eq!(end - start, SIZE as u64, "Incorrect number of bytes read");

        return Ok(Self {
            flags,
            four_cc,
            rgb_bit_count,
            r_bit_mask,
            g_bit_mask,
            b_bit_mask,
            a_bit_mask,
        });
    }
}

#[derive(Debug)]
pub struct DDSHeader {
    flags: BitFlags<DDSFlags>,
    height: u32,
    width: u32,
    pitch_or_linear_size: u32,
    depth: u32,
    mipmap_count: u32,
    pixel_format: DDSPixelFormat,
    caps: (BitFlags<DDSCaps>, BitFlags<DDSCaps2>, u32, u32),
}

impl DDSHeader {
    fn new<R: Read + Seek>(mut reader: R) -> Result<Self> {
        const SIZE: u32 = 124;

        let start = reader.stream_position().into_diagnostic()?;

        let size: u32 = reader.load_le()?;
        if size != SIZE {
            bail!("Invalid header size: found {0} should be {1}", size, SIZE)
        }

        let flags: u32 = reader.load_le()?;
        let flags = BitFlags::<DDSFlags>::from_bits(flags).into_diagnostic()?;

        let [height, width, pitch_or_linear_size, depth, mipmap_count] =
            reader.load_array_le::<u32, 5>()?;

        // skip reserved bytes
        reader.seek(Current(11 * 4)).into_diagnostic()?;

        let pixel_format = DDSPixelFormat::new(&mut reader)?;

        let caps = reader.load_array_le::<u32, 4>()?;

        let caps = (
            BitFlags::<DDSCaps>::from_bits(caps[0]).into_diagnostic()?,
            BitFlags::<DDSCaps2>::from_bits(caps[1]).into_diagnostic()?,
            caps[2],
            caps[3],
        );

        // skip reserved bytes
        reader.seek(Current(4)).into_diagnostic()?;

        let end = reader.stream_position().into_diagnostic()?;
        assert_eq!(end - start, SIZE as u64, "Incorrect number of bytes read");

        Ok(Self {
            flags,
            height,
            width,
            pitch_or_linear_size,
            depth,
            mipmap_count,
            pixel_format,
            caps,
        })
    }
}

#[derive(Debug)]
pub struct DDSFile {
    pub header: DDSHeader,
    pub data: Vec<u8>,
}

impl DDSFile {
    pub fn new<R: Read + Seek>(mut reader: R) -> Result<Self> {
        let magic = reader.load_array_le::<u8, 4>()?;

        if &magic != b"DDS " {
            bail!("Invalid magic numbers: {magic:02X?}. Expected b\"DDS \" ([44, 44, 53, 20])",)
        }

        let header = DDSHeader::new(&mut reader)?;

        let data: Result<Vec<u8>, std::io::Error> = reader.bytes().collect();
        let data = data.into_diagnostic()?;

        return Ok(Self { header, data });
    }
}
