use std::default::Default;
use std::fmt::{Debug, Formatter};

use binrw::prelude::*;
use enumflags2::{BitFlags, bitflags};
use crate::error::TextureError;

use crate::format::{AlphaFormat, ColorFormat, Format};

/// Bit flags for identifying various information in a [`PixelFormatIntermediate`] object. Not exposed to the API.
#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PixelFormatFlags {
    AlphaPixels = 0x1,
    Alpha = 0x2,
    FourCC = 0x4,
    RGB = 0x40,
    YUV = 0x200,
    Luminance = 0x20000,
}

/// Intermediary literal representation of PixelFormat to leverage BinRW.
/// This gets converted to/from PixelFormat which is an easier to use data structure
#[binrw]
#[brw(magic = 32u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct PixelFormatIntermediate {
    #[br(map = BitFlags::from_bits_truncate)]
    #[bw(map = | bf | bf.bits())]
    pub flags: BitFlags<PixelFormatFlags>,
    pub four_cc: FourCC,
    pub bit_count: u32,
    pub bitmasks: [u32; 4],
}

/// A four byte format code. Usually an ASCII-like string but sometimes a u32.
/// For maximum compatibility it's just stored as a byte string, but printed as text in `Debug` if
/// it's valid UTF-8
#[binrw]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct FourCC(pub [u8; 4]);

impl Debug for FourCC {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Ok(as_str) = String::from_utf8(Vec::from(&self.0[..])) {
            f.write_str(as_str.as_str())
        } else {
            let as_u32 = u32::from_le_bytes(self.0);
            f.write_str(format!("{as_u32}").as_str())
        }
    }
}

impl From<&[u8; 4]> for FourCC {
    fn from(value: &[u8; 4]) -> Self { Self(*value) }
}

/// Representation of the DDS PixelFormat data structure as an enum.
/// Either a FourCC or a descriptor of a simple Uncompressed format.
#[binrw]
#[br(map = PixelFormatIntermediate::into)]
#[bw(map = | pf: & PixelFormat | PixelFormatIntermediate::from( * pf))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    FourCC(FourCC),
    Uncompressed {
        bit_count: u32,
        color_format: ColorFormat,
        alpha_format: AlphaFormat,
    },
}


impl From<PixelFormatIntermediate> for PixelFormat {
    fn from(value: PixelFormatIntermediate) -> Self {
        // unpack intermediate struct converted with binrw
        let PixelFormatIntermediate {
            flags, four_cc, bit_count, bitmasks
        } = value;

        // If FourCC is set, just return immediately
        if flags.contains(PixelFormatFlags::FourCC) {
            return PixelFormat::FourCC(four_cc);
        }

        // Match Alpha flags. extra flags are ignored
        let alpha_format = if flags.contains(PixelFormatFlags::Alpha) {
            AlphaFormat::Custom { alpha_mask: bitmasks[3] }
        } else if flags.contains(PixelFormatFlags::AlphaPixels) {
            AlphaFormat::Custom { alpha_mask: bitmasks[3] }
        } else {
            AlphaFormat::Opaque
        };

        // Match Color flags. extra flags are ignored
        let color_format = if flags.contains(PixelFormatFlags::RGB) {
            ColorFormat::RGB { r_mask: bitmasks[0], g_mask: bitmasks[1], b_mask: bitmasks[2], srgb: false }
        } else if flags.contains(PixelFormatFlags::YUV) {
            ColorFormat::YUV { y_mask: bitmasks[0], u_mask: bitmasks[1], v_mask: bitmasks[2] }
        } else if flags.contains(PixelFormatFlags::Luminance) {
            ColorFormat::L { l_mask: bitmasks[0] }
        } else {
            ColorFormat::None
        };

        PixelFormat::Uncompressed { bit_count, color_format, alpha_format }
    }
}

impl From<PixelFormat> for PixelFormatIntermediate {
    fn from(value: PixelFormat) -> PixelFormatIntermediate {
        match value {
            PixelFormat::FourCC(four_cc) => {
                // If FourCC, set single flag and zero masks and bit_count
                PixelFormatIntermediate {
                    flags: PixelFormatFlags::FourCC.into(),
                    four_cc,
                    bit_count: 0u32,
                    bitmasks: [0u32; 4],
                }
            }
            PixelFormat::Uncompressed { bit_count: pitch, color_format, alpha_format } => {
                let mut bitmasks = [0u32; 4];
                let color_flag: BitFlags<PixelFormatFlags>;
                let alpha_flag: BitFlags<PixelFormatFlags>;

                // Color flag and masks
                (color_flag, bitmasks[0], bitmasks[1], bitmasks[2]) = match color_format {
                    ColorFormat::RGB { r_mask, g_mask, b_mask, .. } => {
                        (PixelFormatFlags::RGB.into(), r_mask, g_mask, b_mask)
                    }
                    ColorFormat::YUV { y_mask, u_mask, v_mask } => {
                        (PixelFormatFlags::YUV.into(), y_mask, u_mask, v_mask)
                    }
                    ColorFormat::L { l_mask } => {
                        (PixelFormatFlags::Luminance.into(), l_mask, 0u32, 0u32)
                    }
                    ColorFormat::None => {
                        (BitFlags::default(), 0u32, 0u32, 0u32)
                    }
                };

                // Alpha flag and mask
                (alpha_flag, bitmasks[3]) = match alpha_format {
                    AlphaFormat::Custom { alpha_mask } |
                    AlphaFormat::Straight { alpha_mask } |
                    AlphaFormat::Premultiplied { alpha_mask } => { (PixelFormatFlags::AlphaPixels.into(), alpha_mask) }
                    AlphaFormat::Opaque => { (BitFlags::default(), 0u32) }
                };

                // Build intermediate object for conversion with binrw
                PixelFormatIntermediate {
                    flags: color_flag | alpha_flag,
                    four_cc: FourCC::default(),
                    bit_count: pitch * 8,
                    bitmasks,
                }
            }
        }
    }
}

impl TryFrom<PixelFormat> for Format {
    type Error = TextureError;

    fn try_from(pf: PixelFormat) -> Result<Format, Self::Error> {
        use crate::format::Format::*;
        match pf {
            PixelFormat::FourCC(four_cc) => {
                match &four_cc.0 {
                    b"DX10" => {
                        Err(TextureError::Format(
                            "Cannot convert DX10 PixelFormat".to_string()))
                    } // DX10 header must be stored elsewhere
                    b"DXT1" => Ok(BC1 { srgb: false }), // DXT1, AKA BC1
                    b"DXT3" => Ok(BC2 { srgb: false }), // DXT3, AKA BC2
                    b"DXT5" => Ok(BC3 { srgb: false }), // DXT5, AKA BC3
                    b"BC4U" => Ok(BC4 { signed: false }), // BC4 Unsigned
                    b"BC4S" => Ok(BC4 { signed: true }), // BC4 Signed
                    b"ATI2" | b"BC5U" => Ok(BC5 { signed: false }), // BC5 Unsigned
                    b"BC5S" => Ok(BC5 { signed: true }), // BC5 Signed
                    four_cc => Err(TextureError::Format(
                        format!("Unknown FourCC code: '{four_cc:?}'", )
                    )),
                }
            }
            PixelFormat::Uncompressed { bit_count, alpha_format, color_format } => {
                if bit_count % 8 != 0 {
                    return Err(TextureError::Format(format!("BitCount {bit_count} is not divisible by 8")));
                }

                Ok(Uncompressed {
                    pitch: (bit_count / 8) as usize,
                    alpha_format,
                    color_format,
                })
            }
        }
    }
}

impl TryFrom<Format> for PixelFormat {
    type Error = TextureError;

    fn try_from(format: Format) -> Result<Self, Self::Error> {
        #[allow(unreachable_patterns)]
        match format {
            Format::BC1 { .. } => { Ok(PixelFormat::FourCC(b"DXT1".into())) }
            Format::BC2 { .. } => { Ok(PixelFormat::FourCC(b"DXT3".into())) }
            Format::BC3 { .. } => { Ok(PixelFormat::FourCC(b"DXT5".into())) }
            Format::BC4 { signed: false } => { Ok(PixelFormat::FourCC(b"BC4U".into())) }
            Format::BC4 { signed: true } => { Ok(PixelFormat::FourCC(b"BC4S".into())) }
            Format::BC5 { signed: false } => { Ok(PixelFormat::FourCC(b"ATI2".into())) }
            Format::BC5 { signed: true } => { Ok(PixelFormat::FourCC(b"BC5S".into())) }
            Format::Uncompressed { pitch, color_format, alpha_format } => {
                Ok(PixelFormat::Uncompressed {
                    bit_count: pitch as u32 * 8,
                    color_format,
                    alpha_format,
                })
            }
            _f => Err(TextureError::Format(format!("PixelFormat does not support this format: {_f:?}")))
        }
    }
}

impl PixelFormat {
    pub fn is_dx10(&self) -> bool {
        match self {
            PixelFormat::FourCC(FourCC(four_cc)) if four_cc == b"DX10" => { true }
            _ => { false }
        }
    }

    pub fn dx10() -> Self {
        PixelFormat::FourCC(FourCC(*b"DX10"))
    }
}