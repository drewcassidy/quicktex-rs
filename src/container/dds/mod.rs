// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;
use std::io::{Read, Seek};

use binrw::prelude::*;
use thiserror::Error;
use enumflags2::{BitFlags, bitflags};

use strum::VariantArray;
use crate::container::dds::dx10_header::{Dimensionality, DX10Header};
use crate::container::dds::pixel_format::{FourCC, PixelFormat};
use crate::dimensions::Dimensions;
use crate::format::Format;

use crate::shape::CubeFace;
use crate::texture::Texture;

mod pixel_format;
mod dx10_header;

#[derive(Debug, Error)]
pub enum DDSError {
    #[error("Format error parsing DDS header: {0}")]
    HeaderError(#[from] binrw::error::Error),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

type DDSResult<T = ()> = Result<T, DDSError>;

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

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DDSFlags {
    Caps = 0x1,
    Height = 0x2,
    Width = 0x4,
    pub(crate) Pitch = 0x8,
    PixelFormat = 0x1000,
    pub(crate) MipmapCount = 0x20000,
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


impl Caps2 {
    fn to_cubemap_face(self) -> Option<CubeFace> {
        use Caps2::*;
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
        use Caps2::*;
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
#[derive(Debug, Copy, Clone)]
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


#[binrw]
#[derive(Debug, Copy, Clone)]
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
    #[br(if (pixel_format.four_cc == FourCC(* b"DX10")))]
    pub dx10header: Option<DX10Header>,
}

impl DDSHeader {
    pub fn faces(&self) -> Option<Vec<CubeFace>> {
        return match self.dx10header.clone() {
            None => {
                // if there's no DX10 header, we read from the caps flags
                let caps2 = self.caps.1;
                if caps2.contains(Caps2::Cubemap) {
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
                match dx10header.dimensionality {
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
        use DDSError::UnsupportedFormat;
        use crate::container::dds::pixel_format::FourCC;

        if let Some(format) = self.pixel_format.as_format()? {
            Ok(format)
        } else {
            // DirectX 10 header format with DXGI

            // fourCC must be "DX10"
            assert!(self.pixel_format.four_cc.eq(&FourCC(*b"DX10")),
                    "No format found in PixelFormat yet FourCC is not 'DX10'");

            let dx10_header = self.dx10header
                .ok_or(UnsupportedFormat("FourCC is 'DX10' but no DX10 header found".into()))?;

            dx10_header.dxgi_format.as_format()
        }
    }
}
