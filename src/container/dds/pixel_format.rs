use binrw::binrw;
use enumflags2::{BitFlags, bitflags};

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
#[derive(Debug, Copy, Clone)]
pub struct PixelFormat {
    #[brw(magic = 32u32)] // Size constant
    #[br(try_map = BitFlags::from_bits)]
    #[bw(map = | bf | bf.bits())]
    pub flags: BitFlags<PixelFormatFlags>,
    pub four_cc: [u8; 4],
    pub bit_count: u32,
    pub color_bit_masks: [u32; 3],
    pub alpha_bit_mask: u32,
}
