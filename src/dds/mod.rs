// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{Read, Seek, Write};

use binrw::binrw;
use enumflags2::{BitFlags, make_bitflags};
use itertools::Itertools;
use strum::VariantArray;

use dx10_header::{AlphaMode, DX10HeaderIntermediate, DXGIFormat};
use header::{Caps1, DDSHeaderIntermediate};
use header::DDSFlags;
use pixel_format::PixelFormat;

use crate::container::ContainerHeader;
use crate::dimensions::{Dimensioned, Dimensions};
use crate::error::{TextureError, TextureResult};
use crate::format::Format;
use crate::shape::{CubeFace, TextureShape};
use crate::texture::{SurfaceReader, Surfaces, Texture};

mod dx10_header;
mod header;
mod pixel_format;

#[cfg(test)]
mod tests;

#[binrw]
#[derive(Debug, Clone)]
#[br(try_map = DDSHeaderIntermediate::try_into)]
#[bw(try_map = | h: & DDSHeader | DDSHeaderIntermediate::try_from( h.clone() ))]
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
        alpha_mode: AlphaMode,
    },
}

impl TryFrom<DDSHeaderIntermediate> for DDSHeader {
    type Error = TextureError;

    fn try_from(raw: DDSHeaderIntermediate) -> TextureResult<Self> {
        // MipmapCount flag might not be set, so count a mipmapcount value greater than 1 as equivalent
        let mips = (raw.flags.contains(DDSFlags::MipmapCount) || raw.mipmap_count > 1)
            .then_some(raw.mipmap_count);

        if let Some(dx10header) = raw.dx10_header {
            let dimensions = dx10header
                .dimensionality
                .as_dimensions(raw.width, raw.height, raw.depth)?;
            let layers = match dx10header.array_size {
                0 | 1 => None,
                l => Some(l),
            };

            Ok(DDSHeader::DX10 {
                dimensions,
                mips,
                layers,
                is_cubemap: dx10header.cube,
                dxgi_format: dx10header.dxgi_format,
                alpha_mode: dx10header.alpha_mode,
            })
        } else {
            let dimensions = if raw.flags.contains(DDSFlags::Depth) {
                Dimensions::try_from([raw.width, raw.height, raw.depth])?
            } else {
                Dimensions::try_from([raw.width, raw.height])?
            };
            let faces = raw.caps2.contains(header::Caps2::Cubemap).then_some(
                raw.caps2
                    .iter()
                    .filter_map(header::Caps2::to_cubemap_face)
                    .collect_vec(),
            );

            Ok(DDSHeader::Legacy {
                dimensions,
                mips,
                faces,
                format: raw.pixel_format,
            })
        }
    }
}

impl TryFrom<DDSHeader> for DDSHeaderIntermediate {
    type Error = TextureError;

    fn try_from(header: DDSHeader) -> Result<Self, Self::Error> {
        let mut flags = make_bitflags!(DDSFlags::{Caps | Width | Height | PixelFormat });
        let mut caps1 = make_bitflags!(Caps1::{Texture});
        let mut caps2 = BitFlags::<header::Caps2>::default();

        let format = header.format();
        let (dimensions, mips, pixel_format, dx10_header) = match header {
            DDSHeader::Legacy {
                dimensions,
                mips,
                faces,
                format,
                ..
            } => {
                if let Some(faces) = faces {
                    caps1 |= Caps1::Complex;
                    caps2 |= header::Caps2::Cubemap;
                    for face in faces {
                        caps2 |= header::Caps2::from_cubemap_face(face)
                    }
                }
                (dimensions, mips, format, None)
            }

            DDSHeader::DX10 {
                dimensions,
                mips,
                layers,
                is_cubemap,
                dxgi_format,
                alpha_mode,
                ..
            } => {
                if is_cubemap {
                    caps1 |= Caps1::Complex;
                    caps2 |= header::Caps2::Cubemap;
                    for face in CubeFace::VARIANTS {
                        caps2 |= header::Caps2::from_cubemap_face(*face)
                    }
                }

                if layers.is_some() {
                    caps1 |= Caps1::Complex;
                }

                let dx10_header = Some(DX10HeaderIntermediate {
                    dxgi_format,
                    dimensionality: dimensions.into(),
                    cube: is_cubemap,
                    array_size: layers.unwrap_or(1),
                    alpha_mode,
                });
                (dimensions, mips, PixelFormat::dx10(), dx10_header)
            }
        };

        let pitch_or_linear_size = match format {
            // uncompressed format
            Ok(Format::Uncompressed { pitch, .. }) => {
                flags |= DDSFlags::Pitch;
                pitch as u32 * dimensions.width()
            }
            // compressed format
            Ok(format) => {
                flags |= DDSFlags::LinearSize;
                format.size_for(dimensions) as u32
            }
            // unknown format, just leave as 0 and hope the receiver doesn't mind.
            // this probably cant be encountered in normal use unless an API user
            // makes a DDS header from scratch
            Err(TextureError::Format(_)) => 0,
            // unexpected error: rethrow
            Err(err) => return Err(err),
        };

        let depth = match dimensions {
            Dimensions::_3D([_, _, depth]) => depth.into(),
            _ => 0,
        };

        let mipmap_count = match mips {
            None => 0u32,
            Some(m) => {
                flags |= DDSFlags::MipmapCount;
                caps1 |= Caps1::Complex;
                caps1 |= Caps1::Mipmap;
                m
            }
        };

        Ok(DDSHeaderIntermediate {
            flags,
            height: dimensions.height(),
            width: dimensions.width(),
            pitch_or_linear_size,
            depth,
            mipmap_count,
            pixel_format,
            caps1,
            caps2,
            caps3: 0,
            caps4: 0,
            dx10_header,
        })
    }
}

impl DDSHeader {
    fn for_texture_legacy(texture: &Texture) -> TextureResult<Self> {
        if texture.layers().is_some() {
            return Err(TextureError::Capability(
                "Texture arrays are not supported by legacy DDS headers".to_string(),
            ));
        }
        let dimensions = texture.dimensions();
        let mips: Option<u32> = texture.mips().map(|m| m as u32);
        let faces = texture.faces();
        let format: PixelFormat = texture.format.try_into()?;

        Ok(DDSHeader::Legacy {
            dimensions,
            mips,
            faces,
            format,
        })
    }

    fn for_texture_dx10(texture: &Texture) -> TextureResult<Self> {
        let dimensions = texture.dimensions();
        let mips: Option<u32> = texture.mips().map(|m| m as u32);
        let layers: Option<u32> = texture.layers().map(|m| m as u32);
        let is_cubemap = match texture.faces() {
            None => false,
            Some(faces) if faces.len() == 6 => true,
            Some(_) => {
                return Err(TextureError::Capability(
                    "Incomplete cubemaps are not supported by DX10 DDS headers".to_string(),
                ));
            }
        };
        let (dxgi_format, alpha_mode) = dx10_header::try_from_format(texture.format)?;

        Ok(DDSHeader::DX10 {
            dimensions,
            mips,
            layers,
            is_cubemap,
            dxgi_format,
            alpha_mode,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DDSHeaderMode {
    #[default]
    PreferLegacy,
    ForceLegacy,
    ForceDX10,
}

#[derive(Clone, Debug, Default)]
pub struct DDSHeaderArgs {
    pub mode: DDSHeaderMode,
}

impl ContainerHeader for DDSHeader {
    type Args = DDSHeaderArgs;

    fn read_surfaces<R: Read + Seek>(&self, reader: &mut R) -> TextureResult<Surfaces> {
        let mut surface_reader = SurfaceReader {
            format: self.format()?,
            reader,
        };
        let layers = self.layers()?;
        let faces = self.faces()?.map(|f| {
            f.into_iter()
                .sorted_by_key(header::cubemap_order)
                .collect_vec()
        });
        let mips = self.mips()?;

        // DDS files are ordered as Array(Cubemap(Mipmap(Surface)))
        // yes this is confusing I couldn't figure out how to abstract it
        surface_reader.read_layers(self.dimensions()?, layers, |r: &mut SurfaceReader<R>, d| {
            r.read_faces(d, faces.clone(), |r: &mut SurfaceReader<R>, d| {
                r.read_mips(d, mips, SurfaceReader::<R>::read_surface)
            })
        })
    }

    fn write_surfaces<W: Write + Seek>(
        &self,
        writer: &mut W,
        surfaces: Surfaces,
    ) -> TextureResult<()> {
        for (_, layer) in surfaces.iter_layers() {
            for (_, face) in layer
                .iter_faces()
                .sorted_by_key(|(c, _)| c.map_or(0, |c| header::cubemap_order(&c)))
            {
                for (_, mip) in face.iter_mips() {
                    writer.write(
                        &*mip
                            .try_into_surface()
                            .expect("Innermost shape is not a surface")
                            .buffer,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn from_texture_args(
        texture: &Texture,
        args: &<Self as ContainerHeader>::Args,
    ) -> TextureResult<Self> {
        if args.mode != DDSHeaderMode::ForceDX10 {
            // try to make a legacy header

            match Self::for_texture_legacy(texture) {
                Ok(header) => return Ok(header),

                // cant try again, return
                Err(e) if args.mode == DDSHeaderMode::ForceLegacy => return Err(e),

                // ignore capability  and format errors, will retry with DX10
                Err(TextureError::Capability(_) | TextureError::Format(_)) => {}

                // other errors should be rethrown
                Err(e) => return Err(e),
            }
        }

        // try to make a DX10 header
        Self::for_texture_dx10(texture)
    }

    fn dimensions(&self) -> TextureResult<Dimensions> {
        Ok(match self {
            DDSHeader::Legacy { dimensions, .. } | DDSHeader::DX10 { dimensions, .. } => {
                *dimensions
            }
        })
    }

    fn layers(&self) -> TextureResult<Option<usize>> {
        Ok(match self {
            DDSHeader::DX10 {
                layers: Some(layers),
                ..
            } => Some(*layers as usize),
            _ => None,
        })
    }

    fn faces(&self) -> TextureResult<Option<Vec<CubeFace>>> {
        Ok(match self {
            DDSHeader::Legacy { faces, .. } => faces.clone(),
            DDSHeader::DX10 { is_cubemap, .. } => is_cubemap.then_some(CubeFace::VARIANTS.into()),
        })
    }

    fn mips(&self) -> TextureResult<Option<usize>> {
        Ok(match self {
            DDSHeader::Legacy {
                mips: Some(mips), ..
            }
            | DDSHeader::DX10 {
                mips: Some(mips), ..
            } => Some(*mips as usize),
            _ => None,
        })
    }

    fn format(&self) -> TextureResult<Format> {
        match self {
            DDSHeader::Legacy { format, .. } => (*format).try_into(),
            DDSHeader::DX10 {
                dxgi_format,
                alpha_mode,
                ..
            } => dx10_header::try_into_format(dxgi_format, alpha_mode),
        }
    }
}
