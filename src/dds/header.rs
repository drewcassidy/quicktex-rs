// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use binrw::binrw;
use enumflags2::{bitflags, BitFlags};

use crate::shape::CubeFace;

use super::dx10_header::DX10HeaderIntermediate;
use super::pixel_format::PixelFormat;

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DDSFlags {
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
pub(super) enum Caps1 {
    Complex = 0x8,
    Mipmap = 0x400000,
    Texture = 0x1000,
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Caps2 {
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
    pub(super) fn to_cubemap_face(self) -> Option<CubeFace> {
        CAPS_CUBEMAP_MAP
            .iter()
            .find_map(|(cap, face)| (*cap == self).then_some(*face))
    }

    pub(super) fn from_cubemap_face(face: CubeFace) -> Self {
        CAPS_CUBEMAP_MAP
            .iter()
            .find_map(|(cap, rface)| (*rface == face).then_some(*cap))
            .expect("Invalid cubemap face")
    }
}

pub(super) fn cubemap_order(face: &CubeFace) -> usize {
    CAPS_CUBEMAP_MAP
        .iter()
        .position(|(_, rface)| *rface == *face)
        .expect("Invalid cubemap face")
}

#[binrw]
#[derive(Debug, Copy, Clone)]
#[brw(little, magic = b"DDS ")]
pub(super) struct DDSHeaderIntermediate {
    #[br(temp)]
    #[bw(calc = 124u32)]
    _size: u32,
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
    pub caps2: BitFlags<Caps2>,
    pub caps3: u32,
    #[brw(pad_after = 4)]
    pub caps4: u32,
    #[br(if (pixel_format.is_dx10()))]
    pub dx10_header: Option<DX10HeaderIntermediate>,
}
