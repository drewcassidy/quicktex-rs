// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use miette::{bail, diagnostic, IntoDiagnostic, Result};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

use crate::texture::TextureList;
use crate::util::ReadExt;
use arrayvec::ArrayString;
use bitflags::{bitflags, Flags};
use bitvec::order::Msb0;
use bitvec::view::BitView;
use miette::Diagnostic;
use std::fmt::{Debug, Display, Formatter};
use std::io::SeekFrom::Current;
use std::io::{Read, Seek};
use thiserror::Error;

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

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct DDSFlags : u32 {
        const CAPS = 0x1;
        const HEIGHT = 0x2;
        const WIDTH = 0x4;
        const PITCH = 0x8;
        const PIXELFORMAT = 0x1000;
        const MIPMAPCOUNT = 0x20000;
        const LINEARSIZE = 0x80000;
        const DEPTH = 0x800000;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct DDSCaps : u32 {
        const COMPLEX = 0x8;
        const MIPMAP = 0x400000;
        const TEXTURE = 0x1000;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct DDSCaps2 : u32 {
        const CUBEMAP = 0x200;
        const CUBEMAP_POSITIVEX = 0x400;
        const CUBEMAP_NEGATIVEX = 0x800;
        const CUBEMAP_POSITIVEY = 0x1000;
        const CUBEMAP_NEGATIVEY = 0x2000;
        const CUBEMAP_POSITIVEZ = 0x4000;
        const CUBEMAP_NEGATIVEZ = 0x8000;
        const VOLUME = 0x200000;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct DDSPixelFormatFlags : u32 {
        const ALPHAPIXELS = 0x1;
        const ALPHA = 0x2;
        const FOURCC = 0x4;
        const RGB = 0x40;
        const YUV = 0x200;
        const LUMINANCE = 0x20000;
    }
}

struct DDSPixelFormat {
    flags: DDSPixelFormatFlags,
    four_cc: ArrayString<4>,
    rgb_bit_count: u32,
    r_bit_mask: u32,
    g_bit_mask: u32,
    b_bit_mask: u32,
    a_bit_mask: u32,
}

impl DDSPixelFormat {
    fn read<R: Read + Seek>(mut reader: R) -> Result<Self> {
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

        let four_cc = ArrayString::<4>::from_byte_string(&reader.load_array_le::<u8, 4>()?)
            .into_diagnostic()?;

        let flags: u32 = reader.load_le()?;
        let flags = DDSPixelFormatFlags::from_bits(flags)
            .ok_or(diagnostic!("Invalid PixelFormat Flags: `{:X}`", flags))?;

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

struct DDSHeader {
    flags: DDSFlags,
    height: u32,
    width: u32,
    pitch_or_linear_size: u32,
    depth: u32,
    mipmap_count: u32,
    pixel_format: DDSPixelFormat,
    caps: (DDSCaps, DDSCaps2, u32, u32),
}

impl DDSHeader {
    fn read<R: Read + Seek>(mut reader: R) -> Result<Self> {
        const SIZE: u32 = 124;

        let start = reader.stream_position().into_diagnostic()?;

        let size: u32 = reader.load_le()?;
        if size != SIZE {
            bail!("Invalid header size: found {0} should be {1}", size, SIZE)
        }

        let flags: u32 = reader.load_le()?;
        let flags =
            DDSFlags::from_bits(flags).ok_or(diagnostic!("Invalid DDS Flags: `{:X}`", flags))?;

        let [height, width, pitch_or_linear_size, depth, mipmap_count] =
            reader.load_array_le::<u32, 5>()?;

        // skip reserved bytes
        reader.seek(Current(11 * 4)).into_diagnostic()?;

        let pixel_format = DDSPixelFormat::read(&mut reader)?;

        let caps = reader.load_array_le::<u32, 4>()?;

        let caps = (
            DDSCaps::from_bits(caps[0])
                .ok_or(diagnostic!("Invalid Caps Flags: `{:X}`", caps[0]))?,
            DDSCaps2::from_bits(caps[1])
                .ok_or(diagnostic!("Invalid Caps2 Flags: `{:X}`", caps[1]))?,
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

struct DDSFile {
    header: DDSHeader,
    data: Vec<u8>,
}

impl DDSFile {
    fn read<R: Read + Seek>(mut reader: R) -> Result<Self> {
        let magic = reader.load_array_le::<u8, 4>()?;

        if &magic != b"DDS " {
            bail!("Invalid magic numbers: {magic:02X?}. Expected b\"DDS \" ([44, 44, 53, 20])",)
        }

        let header = DDSHeader::read(&mut reader)?;

        let mut data = Vec::<u8>::new();
        reader.read_to_end(&mut data).into_diagnostic()?;

        return Ok(Self { header, data });
    }
}
