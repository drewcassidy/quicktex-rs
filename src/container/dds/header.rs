use binrw::{BinRead, binrw, BinWrite};
use enumflags2::{BitFlags, bitflags};
use strum::VariantArray;
use crate::container::dds::DDSResult;
use crate::container::dds::dx10_header::{Dimensionality, DX10Header};
use crate::container::dds::pixel_format::PixelFormat;
use crate::dimensions::Dimensions;
use crate::format::Format;
use crate::shape::CubeFace;

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
    #[br(if (pixel_format.four_cc == * b"DX10"))]
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
        use crate::container::dds::DDSError::UnsupportedFormat;
        use super::pixel_format::PixelFormatFlags;
        use crate::format::{AlphaFormat, ColorFormat};
        use crate::s3tc::S3TCFormat::*;
        use crate::format::Format::*;

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
                format!("Unknown FourCC code: '{0}'", String::from_utf8_lossy(&four_cc[..])))
            ),
            None => {
                let pixel_format = self.pixel_format.clone();
                let color_flags = pixel_format.flags | !PixelFormatFlags::AlphaPixels;
                let has_alpha = pixel_format.flags.intersects(PixelFormatFlags::Alpha | PixelFormatFlags::AlphaPixels);

                let pitch = pixel_format.bit_count as usize;

                let color_format = match color_flags.exactly_one() {
                    Some(PixelFormatFlags::RGB) => Ok(ColorFormat::RGB {
                        srgb: false,
                        bitmasks: pixel_format.color_bit_masks,
                    }),
                    Some(PixelFormatFlags::YUV) => Ok(ColorFormat::YUV {
                        bitmasks: pixel_format.color_bit_masks,
                    }),
                    Some(PixelFormatFlags::Luminance) => Ok(ColorFormat::L {
                        bitmask: pixel_format.color_bit_masks[0]
                    }),
                    Some(PixelFormatFlags::Alpha) | None => Ok(ColorFormat::None),
                    _ => {
                        Err(UnsupportedFormat(
                            format!("Invalid PixelFormat flags: {0:?}", pixel_format.flags)))
                    }
                }?;

                let alpha_format = match has_alpha {
                    true => { AlphaFormat::Custom { bitmask: pixel_format.alpha_bit_mask } }
                    false => { AlphaFormat::Opaque }
                };

                match (&color_format, &alpha_format) {
                    (ColorFormat::None, AlphaFormat::Opaque) =>
                        Err(UnsupportedFormat(
                            "PixelFormat has neither color nor alpha information".into()
                        )),

                    _ => Ok(Uncompressed { color_format, alpha_format, pitch })
                }
            }
        }
    }
}
