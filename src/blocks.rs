use bitvec::prelude::*;
use std::iter::zip;

use bitvec::view::BitView;
use vector_victor::{Matrix, Vector};

use crate::pack::{Pack, Unpack};

type Channel = u8;
type Color = Vector<Channel, 4>;

trait ColorImpl {
    fn r(&self) -> &Channel;
    fn g(&self) -> &Channel;
    fn b(&self) -> &Channel;
    fn a(&self) -> &Channel;

    fn from_565(packed: u16) -> Self;
    fn to_565(&self) -> u16;
}

impl ColorImpl for Color {
    fn r(&self) -> &Channel {
        &self[0]
    }
    fn g(&self) -> &Channel {
        &self[1]
    }

    fn b(&self) -> &Channel {
        &self[2]
    }

    fn a(&self) -> &Channel {
        &self[3]
    }

    fn from_565(packed: u16) -> Self {
        let bits = packed.view_bits::<Msb0>();
        // TODO: Fix rounding for 565
        let r: Channel = bits[0..5].load_le::<u8>() << 3;
        let g: Channel = bits[5..11].load_le::<u8>() << 2;
        let b: Channel = bits[11..16].load_le::<u8>() << 3;
        let a: Channel = u8::MAX;

        Color::vec([r, g, b, a])
    }

    fn to_565(&self) -> u16 {
        let mut packed = 0u16;
        let bits = packed.view_bits_mut::<Msb0>();
        bits[0..5].store_le(*self.r() >> 3);
        bits[5..11].store_le(*self.g() >> 2);
        bits[11..16].store_le(*self.b() >> 3);

        return packed;
    }
}

trait TextureBlock: Sized {
    type Bytes: AsRef<[u8]>; // = [u8; 8], etc. Many thanks to @kornel@mastodon.social
    const SIZE: usize;
    const WIDTH: usize = 4;
    const HEIGHT: usize = 4;

    fn to_bytes(&self) -> Self::Bytes;
    fn from_bytes(bytes: &Self::Bytes) -> Self;
}

struct Texture<B>
where
    B: TextureBlock,
{
    blocks: Vec<B>,
}

#[derive(Copy, Clone)]
struct BC1Block {
    colors: [Color; 2],
    codes: Matrix<u8, 4, 4>,
}

impl TextureBlock for BC1Block {
    type Bytes = [u8; 8];
    const SIZE: usize = 8;

    fn to_bytes(&self) -> Self::Bytes {
        let mut bytes: Self::Bytes = [0; 8];
        let bits = bytes.view_bits_mut::<Msb0>();

        // store endpoints
        bits[0..16].store_le(self.colors[0].to_565());
        bits[16..32].store_le(self.colors[1].to_565());

        // store codes
        zip(self.codes.rows(), bits[32..].chunks_mut(8))
            // pack each row of 2-bit values into reversed chunks
            .for_each(|(src, dst)| dst.rchunks_mut(2).pack_be(src));

        bytes
    }

    fn from_bytes(bytes: &Self::Bytes) -> Self {
        let bits = bytes.as_bits::<Msb0>();

        // load endpoints
        let color0 = Color::from_565(bits[0..16].load_le());
        let color1 = Color::from_565(bits[16..32].load_le());

        // load codes
        let codes = Matrix::<u8, 4, 4>::from_rows(
            // reverse each row of 2-bit numbers and collect them to a Vector
            bits.chunks(8).map(|r| r.rchunks(2).unpack_le().collect()),
        );

        Self {
            colors: [color0, color1],
            codes,
        }
    }
}

struct BC4Block {
    endpoints: [u8; 2],
    codes: Matrix<u8, 4, 4>,
}

impl TextureBlock for BC4Block {
    type Bytes = [u8; 8];
    const SIZE: usize = 8;

    fn to_bytes(&self) -> Self::Bytes {
        let mut bytes: Self::Bytes = [0; 8];
        let bits = bytes.view_bits_mut::<Msb0>();

        // store endpoints
        bits[0..8].store_le(self.endpoints[0]);
        bits[8..16].store_le(self.endpoints[1]);

        // store codes into packed number
        // BC4 code packing is annoying because some of the 3-bit values cross byte-boundaries,
        // but those bytes are stored little-endian 🙄
        let mut codes_packed: u64 = 0;
        let codes_bits = codes_packed.view_bits_mut::<Msb0>();
        zip(self.codes.rows(), codes_bits.chunks_mut(12))
            // pack each row of 3-bit values into reversed chunks
            .for_each(|(src, dst)| dst.rchunks_mut(3).pack_be(src));

        // store packed codes
        bits[16..].store_le(codes_packed);

        bytes
    }

    fn from_bytes(bytes: &Self::Bytes) -> Self {
        let bits = bytes.as_bits::<Msb0>();

        // load endpoints
        let endpoint0: u8 = bits[0..8].load_le();
        let endpoint1: u8 = bits[8..16].load_le();

        // load codes
        let codes_packed: u64 = bits[16..].load_le();
        let codes_bits = codes_packed.view_bits::<Msb0>();
        let codes = Matrix::<u8, 4, 4>::from_rows(
            // reverse each row of 3-bit numbers and collect them to a Vector
            codes_bits
                .chunks(12)
                .map(|r| r.rchunks(3).unpack_le().collect()),
        );

        Self {
            endpoints: [endpoint0, endpoint1],
            codes,
        }
    }
}

struct BC3Block {
    rgb: BC1Block,
    a: BC4Block,
}

impl TextureBlock for BC3Block {
    type Bytes = [u8; 16];
    const SIZE: usize = 16;

    fn to_bytes(&self) -> Self::Bytes {
        let mut bytes: Self::Bytes = [0; 16];
        bytes[0..8].copy_from_slice(&self.rgb.to_bytes()[..]);
        bytes[8..16].copy_from_slice(&self.a.to_bytes()[..]);

        bytes
    }

    fn from_bytes(bytes: &Self::Bytes) -> Self {
        Self {
            rgb: BC1Block::from_bytes(&<[u8; 8]>::try_from(&bytes[0..8]).unwrap()),
            a: BC4Block::from_bytes(&<[u8; 8]>::try_from(&bytes[8..16]).unwrap()),
        }
    }
}

struct BC5Block {
    r: BC4Block,
    g: BC4Block,
}

impl TextureBlock for BC5Block {
    type Bytes = [u8; 16];
    const SIZE: usize = 16;

    fn to_bytes(&self) -> Self::Bytes {
        let mut bytes: Self::Bytes = [0; 16];
        bytes[0..8].copy_from_slice(&self.r.to_bytes()[..]);
        bytes[8..16].copy_from_slice(&self.g.to_bytes()[..]);

        bytes
    }

    fn from_bytes(bytes: &Self::Bytes) -> Self {
        Self {
            r: BC4Block::from_bytes(&<[u8; 8]>::try_from(&bytes[0..8]).unwrap()),
            g: BC4Block::from_bytes(&<[u8; 8]>::try_from(&bytes[8..16]).unwrap()),
        }
    }
}
