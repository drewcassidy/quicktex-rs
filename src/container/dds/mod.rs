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
use crate::container::dds::DDSError::HeaderError;
use crate::container::dds::dx10_header::{Dimensionality, DX10Header};
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

impl From<TextureError> for DDSError {
    fn from(value: TextureError) -> Self {
        match value {
            TextureError::IO(io) => { Self::IOError(io) }
            TextureError::Shape(shape) => { Self::ShapeError(shape) }
        }
    }
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
    #[br(if (pixel_format.is_dx10()))]
    pub dx10header: Option<DX10Header>,
}

impl IntoContainerError for DDSError {
    fn into(self, op: &'static str) -> ContainerError {
        ContainerError::DDSError(self, op)
    }
}

impl DDSHeader {
    // fn read_mips<R: Read>(&self, mut reader: TextureReader<R>) -> DDSResult<Texture> {
    //     if let Some(mip_count) = self.mips()? {
    //         let textures = self.dimensions()?.mips().take(mip_count)
    //             .map(|d| -> DDSResult<_> {
    //                 reader.read_surface(d).map_err(DDSError::from)
    //             })
    //             .collect::<DDSResult<Vec<_>>>()?;
    //         Ok(Texture::try_from_mips(textures).expect("Shape error reading mip chain"))
    //     } else {
    //         Ok(reader.read_surface(self.dimensions()?)?)
    //     }
    // }
    //
    // fn read_faces<R: Read>(&self, mut reader: TextureReader<R>) -> DDSResult<Texture> {
    //     if let Some(mut faces) = self.faces()? {
    //         faces.sort_by_key(cubemap_order);
    //         let textures = faces.into_iter()
    //             .map(|f| -> DDSResult<_> {
    //                 Ok((f, self.read_mips(reader)?))
    //             })
    //             .collect::<Result<Vec<_>, _>>()?;
    //         Ok(Texture::try_from_faces(textures).expect("Shape error reading mip chain"))
    //     } else {
    //         self.read_mips(reader)
    //     }
    // }
    //
    // fn read_all<R: Read>(&self, mut reader: TextureReader<R>) -> DDSResult<Texture> {
    //     if let Some(layers) = self.layers()? {
    //         let textures = (0..layers)
    //             .map(|l| self.read_faces(reader))
    //             .collect::<Result<Vec<_>, _>>()?;
    //         Ok(Texture::try_from_layers(textures).expect("Shape error reading mip chain"))
    //     } else {
    //         self.read_faces(reader)
    //     }
    // }
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
        Ok(match self.dx10header.clone() {
            None =>
                if self.flags.contains(DDSFlags::Depth) {
                    Dimensions::_3D([self.width, self.height, self.depth])
                } else {
                    Dimensions::_2D([self.width, self.height])
                }

            Some(dx10header) =>
                match dx10header.dimensionality {
                    Dimensionality::Texture1D => Dimensions::_1D(self.width),

                    Dimensionality::Texture2D => Dimensions::_2D([self.width, self.height]),

                    Dimensionality::Texture3D => Dimensions::_3D([self.width, self.height, self.depth])
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

        if let Some(format) = self.pixel_format.as_format()? {
            Ok(format)
        } else {
            // DirectX 10 header format with DXGI

            // fourCC must be "DX10"
            assert!(self.pixel_format.is_dx10(),
                    "No format found in PixelFormat yet FourCC is not 'DX10'");

            let dx10_header = self.dx10header
                .ok_or(UnsupportedFormat("FourCC is 'DX10' but no DX10 header found".into()))?;

            dx10_header.dxgi_format.as_format()
        }
    }
}
