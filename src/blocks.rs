use arrayvec::ArrayVec;
use bitvec::prelude::*;
use std::io::Bytes;
use std::iter::zip;

use bitvec::view::BitView;
use itertools::Itertools;
use vector_victor::{Matrix, Vector};

use crate::pack::{Pack, Unpack};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
        let r: Channel = bits[0..5].load_le();
        let g: Channel = bits[5..11].load_le();
        let b: Channel = bits[11..16].load_le();
        let a: Channel = u8::MAX;

        Color::vec([r, g, b, a])
    }

    fn to_565(&self) -> u16 {
        let mut packed = 0u16;
        let bits = packed.view_bits_mut::<Msb0>();
        bits[0..5].store_le(*self.r());
        bits[5..11].store_le(*self.g());
        bits[11..16].store_le(*self.b());

        return packed;
    }
}

trait TextureBlock: Sized {
    type Bytes: AsRef<[u8]>; // = [u8; 8], etc. Many thanks to @kornel@mastodon.social
    const SIZE: usize;
    const WIDTH: usize = 4;
    const HEIGHT: usize = 4;

    fn to_bytes(&self, buf: &mut [u8]) -> Self::Bytes;
    fn from_bytes(buf: &Self::Bytes) -> Self;
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

    fn to_bytes(&self, buf: &mut [u8]) -> Self::Bytes {
        let mut bytes: Self::Bytes = [0; 8];
        let bits = bytes.view_bits_mut::<Msb0>();

        bits[0..16].store_be(self.colors[0].to_565());
        bits[16..32].store_be(self.colors[1].to_565());

        // store codes
        zip(self.codes.rows(), bits[32..].chunks_mut(8))
            .for_each(|(src, dst)| dst.rchunks_mut(2).pack_be(src));

        bytes
    }

    fn from_bytes(bytes: &Self::Bytes) -> Self {
        let bits = bytes.as_bits::<Msb0>();

        let color0 = Color::from_565(bits[0..16].load_be());
        let color1 = Color::from_565(bits[16..32].load_be());

        let codes = Matrix::<u8, 4, 4>::from_rows(
            // reverse each row of 2-bit numbers and collect them to a Vector
            bits.chunks(8).map(|r| r.rchunks(2).unpack_be().collect()),
        );

        Self {
            colors: [color0, color1],
            codes,
        }
    }
}
