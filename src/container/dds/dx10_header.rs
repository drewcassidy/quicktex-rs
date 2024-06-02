use binrw::{BinRead, BinWrite};
use crate::error::TextureError;
use crate::format::Format;

#[derive(BinRead, BinWrite)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[brw(little, repr = u32)]
pub enum DXGIFormat {
    Unknown = 0,
    R32G32B32A32 = 1,
    R32G32B32A32Float = 2,
    R32G32B32A32UInt = 3,
    R32G32B32A32SInt = 4,
    R32G32B32 = 5,
    R32G32B32Float = 6,
    R32G32B32UInt = 7,
    R32G32B32SInt = 8,
    R16G16B16A16 = 9,
    R16G16B16A16Float = 10,
    R16G16B16A16UNorm = 11,
    R16G16B16A16UInt = 12,
    R16G16B16A16SNorm = 13,
    R16G16B16A16SInt = 14,
    R32G32 = 15,
    R32G32Float = 16,
    R32G32UInt = 17,
    R32G32SInt = 18,
    R32G8X24 = 19,
    D32FloatS8X24UInt = 20,
    R32FloatX8X24 = 21,
    X32G8X24UInt = 22,
    R10G10B10A2 = 23,
    R10G10B10A2UNorm = 24,
    R10G10B10A2UInt = 25,
    R11G11B10Float = 26,
    R8G8B8A8 = 27,
    R8G8B8A8UNorm = 28,
    R8G8B8A8UNormSRGB = 29,
    R8G8B8A8UInt = 30,
    R8G8B8A8SNorm = 31,
    R8G8B8A8SInt = 32,
    R16G16 = 33,
    R16G16Float = 34,
    R16G16UNorm = 35,
    R16G16UInt = 36,
    R16G16SNorm = 37,
    R16G16SInt = 38,
    R32 = 39,
    D32Float = 40,
    R32Float = 41,
    R32UInt = 42,
    R32SInt = 43,
    R24G8 = 44,
    D24UNormS8UInt = 45,
    R24UNormX8 = 46,
    X24G8UInt = 47,
    R8G8 = 48,
    R8G8UNorm = 49,
    R8G8UInt = 50,
    R8G8SNorm = 51,
    R8G8SInt = 52,
    R16 = 53,
    R16Float = 54,
    D16UNorm = 55,
    R16UNorm = 56,
    R16UInt = 57,
    R16SNorm = 58,
    R16SInt = 59,
    R8 = 60,
    R8UNorm = 61,
    R8UInt = 62,
    R8SNorm = 63,
    R8SInt = 64,
    A8UNorm = 65,
    R1UNorm = 66,
    R9G9B9E5SharedExp = 67,
    R8G8B8G8UNorm = 68,
    G8R8G8B8UNorm = 69,
    BC1 = 70,
    BC1UNorm = 71,
    BC1UNormSRGB = 72,
    BC2 = 73,
    BC2UNorm = 74,
    BC2UNormSRGB = 75,
    BC3 = 76,
    BC3UNorm = 77,
    BC3UNormSRGB = 78,
    BC4 = 79,
    BC4UNorm = 80,
    BC4SNorm = 81,
    BC5 = 82,
    BC5UNorm = 83,
    BC5SNorm = 84,
    B5G6R5UNorm = 85,
    B5G5R5A1UNorm = 86,
    B8G8R8A8UNorm = 87,
    B8G8R8X8UNorm = 88,
    R10G10B10XRBiasA2UNorm = 89,
    B8G8R8A8 = 90,
    B8G8R8A8UNormSRGB = 91,
    B8G8R8X8 = 92,
    B8G8R8X8UNormSRGB = 93,
    BC6H = 94,
    BC6HUF16 = 95,
    BC6HSF16 = 96,
    BC7 = 97,
    BC7UNorm = 98,
    BC7UNormSRGB = 99,
    AYUV = 100,
    Y410 = 101,
    Y416 = 102,
    NV12 = 103,
    P010 = 104,
    P016 = 105,
    YUV420Opaque = 106,
    YUY2 = 107,
    Y210 = 108,
    Y216 = 109,
    NV11 = 110,
    AI44 = 111,
    IA44 = 112,
    P8 = 113,
    A8P8 = 114,
    B4G4R4A4UNorm = 115,
    P208 = 130,
    V208 = 131,
    V408 = 132,
}

impl TryInto<Format> for DXGIFormat {
    type Error = TextureError;

    fn try_into(self) -> Result<Format, Self::Error> {
        // todo!("DX10 header formats are not currently supported");
        Err(TextureError::Format("DX10 header formats are not currently supported".into()))
    }
}


#[derive(BinRead, BinWrite)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[brw(little, repr = u32)]
pub(crate) enum Dimensionality {
    Texture1D = 2,
    Texture2D = 3,
    Texture3D = 4,
}

#[derive(BinRead, BinWrite)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[brw(little, repr = u32)]
pub(crate) enum AlphaMode {
    Unknown = 0,
    Straight = 1,
    Premultiplied = 2,
    Opaque = 3,
    Custom = 4,
}

#[derive(BinRead, BinWrite)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct DX10HeaderIntermediate {
    pub dxgi_format: DXGIFormat,
    pub dimensionality: Dimensionality,
    #[br(map = | b: u32 | b == 0x4)]
    #[bw(map = | cube | if * cube {0x4u32} else {0x0u32})]
    pub cube: bool,
    pub array_size: u32,
    pub alpha_mode: AlphaMode,
}
