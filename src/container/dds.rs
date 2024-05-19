// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::{Debug, Write};
use std::io::{BufRead, Read, Seek};
use binrw::Endian;

use binrw::prelude::*;
use enumflags2::{bitflags, BitFlags};
use itertools::Itertools;
use strum::VariantArray;
use thiserror::Error;

// use crate::container::{ContainerError, Result};
use crate::container::Container;
use crate::dimensions::Dimensions;
use crate::texture::Texture;


#[derive(Debug, Error)]
pub enum DDSError {
    #[error("Format error parsing DDS header: {0}")]
    HeaderError(#[from] binrw::error::Error),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

type DDSResult<T = ()> = Result<T, DDSError>;

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
pub enum Caps1 {
    Complex = 0x8,
    Mipmap = 0x400000,
    Texture = 0x1000,
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Caps2 {
    Cubemap = 0x200,
    CubemapPositiveX = 0x400,
    CubemapNegativeX = 0x800,
    CubemapPositiveY = 0x1000,
    CubemapNegativeY = 0x2000,
    CubemapPositiveZ = 0x4000,
    CubemapNegativeZ = 0x8000,
    Volume = 0x200000,
}

use crate::container::dds::Caps2::*;
use crate::container::dds::DDSError::UnsupportedFormat;
use crate::format::Format;
use crate::s3tc::S3TCFormat;
use crate::shape::{CubeFace, TextureShapeNode};

impl Caps2 {
    fn to_cubemap_face(self) -> Option<CubeFace> {
        match self {
            CubemapPositiveX => Some(CubeFace::PositiveX),
            CubemapNegativeX => Some(CubeFace::NegativeX),
            CubemapPositiveY => Some(CubeFace::PositiveY),
            CubemapNegativeY => Some(CubeFace::NegativeY),
            CubemapPositiveZ => Some(CubeFace::PositiveZ),
            CubemapNegativeZ => Some(CubeFace::NegativeZ),
            _ => None
        }
    }

    fn from_cubemap_face(face: CubeFace) -> Self {
        match face {
            CubeFace::PositiveX => CubemapPositiveX,
            CubeFace::NegativeX => CubemapNegativeX,
            CubeFace::PositiveY => CubemapPositiveY,
            CubeFace::NegativeY => CubemapNegativeY,
            CubeFace::PositiveZ => CubemapPositiveZ,
            CubeFace::NegativeZ => CubemapNegativeZ
        }
    }
}

/// Named tuple containing all "Caps" bitflags
#[derive(BinRead, BinWrite)]
#[derive(Debug, Clone)]
#[brw(little)]
pub struct Caps(
    #[br(try_map = BitFlags::from_bits)]
    #[bw(map = | bf | bf.bits())]
    BitFlags<Caps1>,
    #[br(try_map = BitFlags::from_bits)]
    #[bw(map = | bf | bf.bits())]
    BitFlags<Caps2>,
    u32,
    u32,
);

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormatFlags {
    AlphaPixels = 0x1,
    Alpha = 0x2,
    FourCC = 0x4,
    RGB = 0x40,
    YUV = 0x200,
    Luminance = 0x20000,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct PixelFormat {
    #[brw(magic = 32u32)] // Size constant
    #[br(try_map = BitFlags::from_bits)]
    #[bw(map = | bf | bf.bits())]
    flags: BitFlags<PixelFormatFlags>,
    four_cc: [u8; 4],
    rgb_bit_count: u32,
    r_bit_mask: u32,
    g_bit_mask: u32,
    b_bit_mask: u32,
    a_bit_mask: u32,
}

#[derive(BinRead, BinWrite)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[brw(little, repr = u32)]
pub enum DXGIFormat {
    Unknown = 0,
    R32G32B32A32 = 1,
    R32G32B32A32Float = 2,
    R32G32B32A32UInt = 3,
    R32G32B32A32SInt = 4,
    R32G32B32 = 5,
    R32G32B32Float = 6,
    R32G32B32UInt = 7,
    R32G32B32SInt = 8,
    R16G16B16A16 = 9,
    R16G16B16A16Float = 10,
    R16G16B16A16UNorm = 11,
    R16G16B16A16UInt = 12,
    R16G16B16A16SNorm = 13,
    R16G16B16A16SInt = 14,
    R32G32 = 15,
    R32G32Float = 16,
    R32G32UInt = 17,
    R32G32SInt = 18,
    R32G8X24 = 19,
    D32FloatS8X24UInt = 20,
    R32FloatX8X24 = 21,
    X32G8X24UInt = 22,
    R10G10B10A2 = 23,
    R10G10B10A2UNorm = 24,
    R10G10B10A2UInt = 25,
    R11G11B10Float = 26,
    R8G8B8A8 = 27,
    R8G8B8A8UNorm = 28,
    R8G8B8A8UNormSRGB = 29,
    R8G8B8A8UInt = 30,
    R8G8B8A8SNorm = 31,
    R8G8B8A8SInt = 32,
    R16G16 = 33,
    R16G16Float = 34,
    R16G16UNorm = 35,
    R16G16UInt = 36,
    R16G16SNorm = 37,
    R16G16SInt = 38,
    R32 = 39,
    D32Float = 40,
    R32Float = 41,
    R32UInt = 42,
    R32SInt = 43,
    R24G8 = 44,
    D24UNormS8UInt = 45,
    R24UNormX8 = 46,
    X24G8UInt = 47,
    R8G8 = 48,
    R8G8UNorm = 49,
    R8G8UInt = 50,
    R8G8SNorm = 51,
    R8G8SInt = 52,
    R16 = 53,
    R16Float = 54,
    D16UNorm = 55,
    R16UNorm = 56,
    R16UInt = 57,
    R16SNorm = 58,
    R16SInt = 59,
    R8 = 60,
    R8UNorm = 61,
    R8UInt = 62,
    R8SNorm = 63,
    R8SInt = 64,
    A8UNorm = 65,
    R1UNorm = 66,
    R9G9B9E5SharedExp = 67,
    R8G8B8G8UNorm = 68,
    G8R8G8B8UNorm = 69,
    BC1 = 70,
    BC1UNorm = 71,
    BC1UNormSRGB = 72,
    BC2 = 73,
    BC2UNorm = 74,
    BC2UNormSRGB = 75,
    BC3 = 76,
    BC3UNorm = 77,
    BC3UNormSRGB = 78,
    BC4 = 79,
    BC4UNorm = 80,
    BC4SNorm = 81,
    BC5 = 82,
    BC5UNorm = 83,
    BC5SNorm = 84,
    B5G6R5UNorm = 85,
    B5G5R5A1UNorm = 86,
    B8G8R8A8UNorm = 87,
    B8G8R8X8UNorm = 88,
    R10G10B10XRBiasA2UNorm = 89,
    B8G8R8A8 = 90,
    B8G8R8A8UNormSRGB = 91,
    B8G8R8X8 = 92,
    B8G8R8X8UNormSRGB = 93,
    BC6H = 94,
    BC6HUF16 = 95,
    BC6HSF16 = 96,
    BC7 = 97,
    BC7UNorm = 98,
    BC7UNormSRGB = 99,
    AYUV = 100,
    Y410 = 101,
    Y416 = 102,
    NV12 = 103,
    P010 = 104,
    P016 = 105,
    YUV420Opaque = 106,
    YUY2 = 107,
    Y210 = 108,
    Y216 = 109,
    NV11 = 110,
    AI44 = 111,
    IA44 = 112,
    P8 = 113,
    A8P8 = 114,
    B4G4R4A4UNorm = 115,
    P208 = 130,
    V208 = 131,
    V408 = 132,
}

#[derive(BinRead, BinWrite)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[brw(little, repr = u32)]
pub enum Dimensionality {
    Texture1D = 2,
    Texture2D = 3,
    Texture3D = 4,
}

#[derive(BinRead, BinWrite)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[brw(little, repr = u32)]
pub enum AlphaMode {
    Unknown = 0,
    Straight = 1,
    Premultiplied = 2,
    Opaque = 3,
    Custom = 4,
}

#[derive(BinRead, BinWrite)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DX10Header {
    dxgi_format: DXGIFormat,
    dimension: Dimensionality,
    #[br(map = | b: u32 | b == 0x4)]
    #[bw(map = | cube | if * cube {0x4u32} else {0x0u32})]
    cube: bool,
    array_size: u32,
    alpha_mode: AlphaMode,
}

impl DXGIFormat {
    pub fn as_format(&self) -> DDSResult<Format> {
        Err(UnsupportedFormat("DX10 header formats are not currently supported".into()))
    }
}


#[binrw]
#[derive(Debug, Clone)]
#[brw(little, magic = b"DDS ")]
pub struct DDSHeader {
    #[brw(magic = 124u32)] // Size constant
    #[br(try_map = BitFlags::from_bits)]
    #[bw(map = | bf | bf.bits())]
    pub flags: BitFlags<DDSFlags>,
    pub height: u32,
    pub width: u32,
    pub pitch_or_linear_size: u32,
    pub depth: u32,
    pub mipmap_count: u32,
    #[brw(pad_before = 44)]
    pub pixel_format: PixelFormat,
    #[brw(pad_after = 4)]
    pub caps: Caps,
    #[br(if (pixel_format.four_cc == * b"DX10"))]
    pub dx10header: Option<DX10Header>,
}

impl DDSHeader {
    pub fn faces(&self) -> Option<Vec<CubeFace>> {
        return match self.dx10header.clone() {
            None => {
                // if there's no DX10 header, we read from the caps flags
                let caps2 = self.caps.1;
                if caps2.contains(Cubemap) {
                    Some(caps2.iter().filter_map(Caps2::to_cubemap_face).collect())
                } else { None }
            }
            Some(dx10header) => {
                // if there is a DX10 header, we check the cube flag.
                // DX10 DDS files do not support partial cubemaps
                if dx10header.cube {
                    Some(CubeFace::VARIANTS.into())
                } else { None }
            }
        };
    }

    pub fn layers(&self) -> Option<u32> {
        match self.dx10header.clone()?.array_size {
            1 | 0 => None,
            layers => Some(layers)
        }
    }

    pub fn mips(&self) -> Option<u32> {
        return match (self.flags.contains(DDSFlags::MipmapCount), self.mipmap_count) {
            (false, _) => None,
            (true, 0) => None,
            (true, mips) => Some(mips)
        };
    }

    pub fn dimensions(&self) -> Dimensions {
        return match self.dx10header.clone() {
            None =>
                if self.flags.contains(DDSFlags::Depth) {
                    Dimensions::_3D {
                        width: self.width,
                        height: self.height,
                        depth: self.depth,
                    }
                } else {
                    Dimensions::_2D {
                        width: self.width,
                        height: self.height,
                    }
                }

            Some(dx10header) =>
                match dx10header.dimension {
                    Dimensionality::Texture1D => Dimensions::_1D {
                        width: self.width
                    },

                    Dimensionality::Texture2D => Dimensions::_2D {
                        width: self.width,
                        height: self.height,
                    },

                    Dimensionality::Texture3D => Dimensions::_3D {
                        width: self.width,
                        height: self.height,
                        depth: self.depth,
                    },
                }
        };
    }

    pub fn format(&self) -> DDSResult<Format> {
        use S3TCFormat::*;
        use Format::*;

        // get the fourCC. if the flag for fourCC exists, it will be a Some([u8;4]). otherwise None
        let four_cc = self.pixel_format.flags.contains(PixelFormatFlags::FourCC)
            .then_some(&self.pixel_format.four_cc);

        match four_cc {
            Some(b"DXT1") => Ok(S3TC(BC1 { srgb: false })),
            Some(b"DXT3") => Ok(S3TC(BC2 { srgb: false })),
            Some(b"DXT5") => Ok(S3TC(BC3 { srgb: false })),
            Some(b"BC4U") => Ok(S3TC(BC4 { signed: false })),
            Some(b"BC4S") => Ok(S3TC(BC4 { signed: true })),
            Some(b"ATI2") => Ok(S3TC(BC5 { signed: false })),
            Some(b"BC5S") => Ok(S3TC(BC5 { signed: true })),
            Some(b"DX10") => {
                self.dx10header
                    .ok_or(UnsupportedFormat("FourCC is 'DX10' but no DX10 header was found".into()))?
                    .dxgi_format.as_format()
            }
            Some(four_cc) => Err(UnsupportedFormat(
                format!("Unknown FourCC code '{0}'", String::from_utf8_lossy(&four_cc[..])))
            ),
            None => {
                Err(UnsupportedFormat(
                    format!("Unknown pixel Format \n{0:?}", self.pixel_format)))
            }
        }
    }
}

fn cubemap_order(face: &CubeFace) -> u32 {
    match face {
        CubeFace::PositiveX => { 0 }
        CubeFace::NegativeX => { 1 }
        CubeFace::PositiveY => { 2 }
        CubeFace::NegativeY => { 3 }
        CubeFace::PositiveZ => { 4 }
        CubeFace::NegativeZ => { 5 }
    }
}

fn read_mips(reader: &mut (impl Read + Seek), header: &DDSHeader) {
    todo!();
}


pub fn read_texture(reader: &mut (impl Read + Seek)) -> DDSResult<Texture> {
    let header = DDSHeader::read(reader)?;

    println!("{header:#?}");

    let format = header.format()?;

    println!("{format:#?}");


    if let Some(mut faces) = header.faces() {
        faces.sort_by_key(|f| cubemap_order(f));

        for face in faces {}
    }
    todo!()
}
