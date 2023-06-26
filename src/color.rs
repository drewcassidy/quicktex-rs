// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use bitvec::prelude::*;
use vector_victor::Vector;

pub type Channel = u8;
pub type Color = Vector<Channel, 4>;

pub trait ColorImpl {
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
