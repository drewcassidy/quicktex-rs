use std::fmt::{Debug, Display, Formatter, Write};
use binrw::binrw;
use enumflags2::{BitFlags, bitflags};
use crate::container::dds::DDSError::UnsupportedFormat;
use crate::container::dds::DDSResult;
use crate::format::{AlphaFormat, ColorFormat, Format};
use crate::format::Format::S3TC;
use crate::s3tc::S3TCFormat::{BC1, BC2, BC3, BC4, BC5};

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
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FourCC(pub [u8; 4]);

impl FourCC {
    /// Convert this FourCC to a Format. Returns None if the four_cc is "DX10", meaning the
    /// actual format is stored elsewhere in the DDS header
    fn as_format(&self) -> DDSResult<Option<Format>> {
        match &self.0 {
            b"DX10" => { Ok(None) }
            b"DXT1" => Ok(Some(S3TC(BC1 { srgb: false }))),
            b"DXT3" => Ok(Some(S3TC(BC2 { srgb: false }))),
            b"DXT5" => Ok(Some(S3TC(BC3 { srgb: false }))),
            b"BC4U" => Ok(Some(S3TC(BC4 { signed: false }))),
            b"BC4S" => Ok(Some(S3TC(BC4 { signed: true }))),
            b"ATI2" => Ok(Some(S3TC(BC5 { signed: false }))),
            b"BC5S" => Ok(Some(S3TC(BC5 { signed: true }))),
            four_cc => Err(UnsupportedFormat(
                format!("Unknown FourCC code: '{four_cc:?}'", )
            )),
        }
    }
}

impl AsRef<[u8]> for FourCC {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}

impl Into<[u8; 4]> for FourCC {
    fn into(self) -> [u8; 4] { self.0 }
}

impl Display for FourCC {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(self.as_ref())[..])
    }
}

impl Debug for FourCC {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Ok(as_str) = String::from_utf8(Vec::from(self.as_ref())) {
            f.write_str(as_str.as_str())
        } else {
            let as_u32 = u32::from_le_bytes(self.0);
            f.write_str(format!("{as_u32}").as_str())
        }
    }
}

#[binrw]
#[derive(Debug, Copy, Clone)]
pub struct PixelFormat {
    #[brw(magic = 32u32)] // Size constant
    #[br(try_map = BitFlags::from_bits)]
    #[bw(map = | bf | bf.bits())]
    pub flags: BitFlags<PixelFormatFlags>,
    pub four_cc: FourCC,
    pub bit_count: u32,
    pub color_bit_masks: [u32; 3],
    pub alpha_bit_mask: u32,
}

impl PixelFormat {
    /// Convert this PixelFormat to a Format. Returns None if the four_cc is "DX10", meaning the
    /// actual format is stored elsewhere in the DDS header
    pub fn as_format(&self) -> DDSResult<Option<Format>> {
        match self.flags.contains(PixelFormatFlags::FourCC) {
            true => { self.four_cc.as_format() }
            false => { Ok(Some(self.as_format_uncompressed()?)) }
        }
    }

    fn as_format_uncompressed(self) -> DDSResult<Format> {
        let color_flags = self.flags & !PixelFormatFlags::AlphaPixels;
        let has_alpha = self.flags.intersects(PixelFormatFlags::Alpha | PixelFormatFlags::AlphaPixels);

        let pitch = (self.bit_count / 8) as usize;

        let color_format = match color_flags.exactly_one() {
            Some(PixelFormatFlags::RGB) => Ok(ColorFormat::RGB {
                srgb: false,
                bitmasks: self.color_bit_masks,
            }),
            Some(PixelFormatFlags::YUV) => Ok(ColorFormat::YUV {
                bitmasks: self.color_bit_masks,
            }),
            Some(PixelFormatFlags::Luminance) => Ok(ColorFormat::L {
                bitmask: self.color_bit_masks[0]
            }),
            Some(PixelFormatFlags::Alpha) | None => Ok(ColorFormat::None),
            _ => {
                Err(UnsupportedFormat(
                    format!("Invalid PixelFormat flags: {0:?}", self.flags)))
            }
        }?;

        let alpha_format = match has_alpha {
            true => { AlphaFormat::Custom { bitmask: self.alpha_bit_mask } }
            false => { AlphaFormat::Opaque }
        };

        match (&color_format, &alpha_format) {
            (ColorFormat::None, AlphaFormat::Opaque) =>
                Err(UnsupportedFormat(
                    "PixelFormat has neither color nor alpha information".into()
                )),

            _ => Ok(Format::Uncompressed { color_format, alpha_format, pitch })
        }
    }
}
