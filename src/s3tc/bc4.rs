use crate::pack::{Pack, Unpack};
use crate::texture::Block;
use bitvec::prelude::*;
use std::iter::zip;
use vector_victor::Matrix;

pub struct BC4Block {
    endpoints: [u8; 2],
    codes: Matrix<u8, 4, 4>,
}

impl Block for BC4Block {
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
