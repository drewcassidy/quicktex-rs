// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;
use std::io::{Read, Seek};

use binrw::prelude::*;
use enumflags2::{BitFlags, bitflags};
use itertools::Itertools;
use strum::VariantArray;
use thiserror::Error;

use crate::container::{ContainerError, ContainerHeader, IntoContainerError};
use crate::container::dds::dx10_header::{Dimensionality, DX10HeaderIntermediate, DXGIFormat};
use crate::container::dds::pixel_format::PixelFormat;
use crate::dimensions::Dimensions;
use crate::format::Format;
use crate::shape::{CubeFace, ShapeError};
use crate::texture::{Texture, TextureError, TextureReader};

mod pixel_format;
mod dx10_header;

#[derive(Debug, Error)]
pub enum DDSError {
    #[error("Error parsing DDS header: {0}")]
    ParseError(#[from] binrw::error::Error),

    #[error("Invalid DDS Header: {0}")]
    HeaderError(String),

    #[error("Invalid Texture Shape: {0}")]
    ShapeError(#[from] ShapeError),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("IO Error with file contents")]
    IOError(#[from] std::io::Error),
}

impl IntoContainerError for DDSError {
    fn into(self, op: &'static str) -> ContainerError {
        ContainerError::DDSError(self, op)
    }
}

impl From<TextureError> for DDSError {
    fn from(value: TextureError) -> Self {
        match value {
            TextureError::IO(io) => { Self::IOError(io) }
            TextureError::Shape(shape) => { Self::ShapeError(shape) }
        }
    }
}

type DDSResult<T = ()> = Result<T, DDSError>;


pub fn read_texture(reader: &mut (impl Read + Seek)) -> DDSResult<Texture> {
    let header = DDSHeader::read(reader)?;
    println!("{header:#?}");
    let texture = header.read_with(reader)?;
    Ok(texture)
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DDSFlags {
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
enum Caps1 {
    Complex = 0x8,
    Mipmap = 0x400000,
    Texture = 0x1000,
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Caps2 {
    Cubemap = 0x200,
    CubemapPositiveX = 0x400,
    CubemapNegativeX = 0x800,
    CubemapPositiveY = 0x1000,
    CubemapNegativeY = 0x2000,
    CubemapPositiveZ = 0x4000,
    CubemapNegativeZ = 0x8000,
    Volume = 0x200000,
}

static CAPS_CUBEMAP_MAP: [(Caps2, CubeFace); 6] = [
    (Caps2::CubemapPositiveX, CubeFace::PositiveX),
    (Caps2::CubemapNegativeX, CubeFace::NegativeX),
    (Caps2::CubemapPositiveY, CubeFace::PositiveY),
    (Caps2::CubemapNegativeY, CubeFace::NegativeY),
    (Caps2::CubemapPositiveZ, CubeFace::PositiveZ),
    (Caps2::CubemapNegativeZ, CubeFace::NegativeZ),
];


impl Caps2 {
    fn to_cubemap_face(self) -> Option<CubeFace> {
        CAPS_CUBEMAP_MAP.iter().find_map(
            |(cap, face)| (*cap == self).then_some(*face)
        )
    }

    fn from_cubemap_face(face: CubeFace) -> Self {
        CAPS_CUBEMAP_MAP.iter().find_map(
            |(cap, rface)| (*rface == face).then_some(*cap)
        ).expect("Invalid cubemap face")
    }
}

fn cubemap_order(face: &CubeFace) -> usize {
    CAPS_CUBEMAP_MAP.iter().position(|(_, rface)| *rface == *face).expect("Invalid cubemap face")
}

#[binrw]
#[derive(Debug, Copy, Clone)]
#[brw(little, magic = b"DDS ")]
struct DDSHeaderIntermediate {
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
    #[br(try_map = BitFlags::from_bits)]
    #[bw(map = | bf | bf.bits())]
    pub caps1: BitFlags<Caps1>,
    #[br(try_map = BitFlags::from_bits)]
    #[bw(map = | bf | bf.bits())]
    #[brw(pad_after = 4)]
    pub caps2: BitFlags<Caps2>,
    pub caps3: u32,
    pub caps4: u32,
    #[br(if (pixel_format.is_dx10()))]
    pub dx10header: Option<DX10HeaderIntermediate>,
}


// #[bw(map = | h: & DDSHeader | DDSHeaderIntermediate::from( * h))]
#[derive(Debug, Clone, BinRead)]
#[br(map = DDSHeaderIntermediate::into)]
pub enum DDSHeader {
    Legacy {
        dimensions: Dimensions,
        mips: Option<u32>,
        faces: Option<Vec<CubeFace>>,
        format: PixelFormat,
    },
    DX10 {
        dimensions: Dimensions,
        mips: Option<u32>,
        layers: Option<u32>,
        is_cubemap: bool,
        dxgi_format: DXGIFormat,
    },
}

impl From<DDSHeaderIntermediate> for DDSHeader {
    fn from(value: DDSHeaderIntermediate) -> Self {
        let mips = value.flags.contains(DDSFlags::MipmapCount).then_some(value.mipmap_count);
        if let Some(dx10header) = value.dx10header {
            let dimensions = match dx10header.dimensionality {
                Dimensionality::Texture1D => Dimensions::_1D(value.width),
                Dimensionality::Texture2D => Dimensions::_2D([value.width, value.height]),
                Dimensionality::Texture3D => Dimensions::_3D([value.width, value.height, value.depth])
            };
            let layers = match dx10header.array_size {
                0 | 1 => None,
                l => Some(l)
            };

            DDSHeader::DX10 {
                dimensions,
                mips,
                layers,
                is_cubemap: dx10header.cube,
                dxgi_format: dx10header.dxgi_format,
            }
        } else {
            let dimensions = if value.flags.contains(DDSFlags::Depth) {
                Dimensions::_3D([value.width, value.height, value.depth])
            } else {
                Dimensions::_2D([value.width, value.height])
            };
            let faces = value.caps2.contains(Caps2::Cubemap).then_some(
                value.caps2.iter().filter_map(Caps2::to_cubemap_face).collect_vec()
            );

            DDSHeader::Legacy {
                dimensions,
                mips,
                faces,
                format: value.pixel_format,
            }
        }
    }
}

impl ContainerHeader for DDSHeader {
    type Error = DDSError;

    fn read_with<R: Read + Seek>(&self, reader: &mut R) -> Result<Texture, Self::Error> {
        let mut texture_reader = TextureReader { format: self.format()?, reader };
        let layers = self.layers()?;
        let faces = self.faces()?.map(|f| f.into_iter().sorted_by_key(cubemap_order).collect_vec());
        let mips = self.mips()?;

        // DDS files are ordered as Array(Cubemap(Mipmap(Surface)))
        // yes this is confusing I couldn't figure out how to abstract it
        let texture =
            texture_reader.read_layers(self.dimensions()?, layers, |r: &mut TextureReader<R>, d| {
                r.read_faces(d, faces.clone(), |r: &mut TextureReader<R>, d| {
                    r.read_mips(d, mips, TextureReader::<R>::read_surface)
                })
            })?;

        Ok(texture)
    }

    fn dimensions(&self) -> Result<Dimensions, DDSError> {
        Ok(match self {
            DDSHeader::Legacy { dimensions, .. } |
            DDSHeader::DX10 { dimensions, .. } => { *dimensions }
        })
    }

    fn layers(&self) -> Result<Option<usize>, DDSError> {
        Ok(match self {
            DDSHeader::DX10 { layers: Some(layers), .. } => { Some(*layers as usize) }
            _ => { None }
        })
    }

    fn faces(&self) -> DDSResult<Option<Vec<CubeFace>>> {
        Ok(match self {
            DDSHeader::Legacy { faces, .. } => { faces.clone() }
            DDSHeader::DX10 { is_cubemap, .. } => {
                is_cubemap.then_some(CubeFace::VARIANTS.into())
            }
        })
    }

    fn mips(&self) -> DDSResult<Option<usize>> {
        Ok(match self {
            DDSHeader::Legacy { mips: Some(mips), .. } |
            DDSHeader::DX10 { mips: Some(mips), .. } => Some(*mips as usize),
            _ => None
        })
    }

    fn format(&self) -> DDSResult<Format> {
        match *self {
            DDSHeader::Legacy { format, .. } => { format.try_into() }
            DDSHeader::DX10 { dxgi_format, .. } => { dxgi_format.try_into() }
        }
    }
}
