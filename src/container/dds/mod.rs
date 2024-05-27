// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;
use std::io::{BufRead, Read, Seek};

use binrw::prelude::*;
use enumflags2::{BitFlags, bitflags};
use image::io::Reader;
use lazycell::LazyCell;
use strum::VariantArray;
use thiserror::Error;

use crate::container::{ContainerError, ContainerHeader, IntoContainerError};
use crate::container::dds::DDSError::{HeaderError, ParseError};
use crate::container::dds::dx10_header::{Dimensionality, DX10Header};
use crate::container::dds::pixel_format::{FourCC, PixelFormat};
use crate::dimensions::Dimensions;
use crate::format::Format;
use crate::shape::{CubeFace, TextureShape};
use crate::texture::Texture;

mod pixel_format;
mod dx10_header;

#[derive(Debug, Error)]
pub enum DDSError {
    #[error("Error parsing DDS header: {0}")]
    ParseError(#[from] binrw::error::Error),

    #[error("Invalid DDS Header: {0}")]
    HeaderError(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("IO Error with file contents")]
    IOError(#[from] std::io::Error),
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


pub fn read_texture(reader: &mut (impl Read + Seek)) -> DDSResult<Texture> {
    let header = DDSHeader::read(reader)?;
    println!("{header:#?}");
    let texture = header.read_with(reader)?;
    Ok(texture)
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

impl IntoContainerError for DDSError {
    fn into(self, op: &'static str) -> ContainerError {
        ContainerError::DDSError(self, op)
    }
}

impl DDSHeader {
    fn read_mips<R: Read>(&self, reader: &mut R) -> DDSResult<Texture> {
        if let Some(mip_count) = self.mips()? {
            let textures = self.dimensions()?.mips().take(mip_count)
                .map(|d| -> DDSResult<_> {
                    Texture::read_surface(reader, d, self.format()?).map_err(DDSError::from)
                })
                .collect::<DDSResult<Vec<_>>>()?;
            Ok(Texture::try_from_mips(textures).expect("Shape error reading mip chain"))
        } else {
            Ok(Texture::read_surface(reader, self.dimensions()?, self.format()?)?)
        }
    }

    fn read_faces<R: Read>(&self, reader: &mut R) -> DDSResult<Texture> {
        if let Some(mut faces) = self.faces()? {
            faces.sort_by_key(cubemap_order);
            let textures = faces.into_iter()
                .map(|f| -> DDSResult<_> {
                    Ok((f, self.read_mips(reader)?))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Texture::try_from_faces(textures).expect("Shape error reading mip chain"))
        } else {
            self.read_mips(reader)
        }
    }

    fn read_all<R: Read>(&self, reader: &mut R) -> DDSResult<Texture> {
        if let Some(layers) = self.layers()? {
            let textures = (0..layers)
                .map(|l| self.read_faces(reader))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Texture::try_from_layers(textures).expect("Shape error reading mip chain"))
        } else {
            self.read_faces(reader)
        }
    }
}

impl ContainerHeader for DDSHeader {
    type Error = DDSError;

    fn read_with<R: Read + Seek>(&self, reader: &mut R) -> Result<Texture, Self::Error> {
        self.read_all(reader)
    }

    fn dimensions(&self) -> Result<Dimensions, DDSError> {
        Ok(match self.dx10header.clone() {
            None =>
                if self.flags.contains(DDSFlags::Depth) {
                    Dimensions::_3D {
                        width: self.width as usize,
                        height: self.height as usize,
                        depth: self.depth as usize,
                    }
                } else {
                    Dimensions::_2D {
                        width: self.width as usize,
                        height: self.height as usize,
                    }
                }

            Some(dx10header) =>
                match dx10header.dimensionality {
                    Dimensionality::Texture1D => Dimensions::_1D {
                        width: self.width as usize
                    },

                    Dimensionality::Texture2D => Dimensions::_2D {
                        width: self.width as usize,
                        height: self.height as usize,
                    },

                    Dimensionality::Texture3D => Dimensions::_3D {
                        width: self.width as usize,
                        height: self.height as usize,
                        depth: self.depth as usize,
                    },
                }
        })
    }

    fn layers(&self) -> Result<Option<usize>, DDSError> {
        Ok(match self.dx10header.map(|d| d.array_size) {
            None | Some(1 | 0) => None,
            Some(layers) => Some(layers as usize)
        })
    }

    fn faces(&self) -> DDSResult<Option<Vec<CubeFace>>> {
        Ok(match self.dx10header.clone() {
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
        })
    }

    fn mips(&self) -> DDSResult<Option<usize>> {
        match (self.flags.contains(DDSFlags::MipmapCount), self.mipmap_count) {
            (false, _) => Ok(None),
            (true, 0) => Err(HeaderError("MipmapCount flag is present, but MipmapCount is 0".into())),
            (true, mips) => Ok(Some(mips as usize))
        }
    }

    fn format(&self) -> DDSResult<Format> {
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
