// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::iter::{zip, Map};

use bitvec::field::BitField;
use bitvec::prelude::*;
use funty::Integral;

pub trait Pack: IntoIterator {
    fn pack_le<V: Integral + Into<isize>, U: IntoIterator<Item = V>>(self, unpacked: U);
    fn pack_be<V: Integral + Into<isize>, U: IntoIterator<Item = V>>(self, unpacked: U);
}

impl<'a, T: BitStore, O: BitOrder, I> Pack for I
where
    I: IntoIterator<Item = &'a mut BitSlice<T, O>>,
    BitSlice<T, O>: BitField,
{
    fn pack_le<V: Integral + Into<isize>, U: IntoIterator<Item = V>>(self, unpacked: U) {
        for (src, dst) in zip(unpacked, self) {
            assert!(
                src.into() < (1 << (dst.len()) - 1) as isize,
                "Input value {:X} cannot be packed into {} bits",
                src,
                dst.len()
            );
            dst.store_le(src);
        }
    }

    fn pack_be<V: Integral + Into<isize>, U: IntoIterator<Item = V>>(self, unpacked: U) {
        for (src, dst) in zip(unpacked, self) {
            assert!(
                src.into() < (1 << (dst.len()) - 1) as isize,
                "Input value {:X} cannot be packed into {} bits",
                src,
                dst.len()
            );
            dst.store_be(src);
        }
    }
}

pub trait Unpack: IntoIterator + Sized {
    fn unpack_le<V: Integral>(
        self,
    ) -> Map<<Self as IntoIterator>::IntoIter, fn(<Self as IntoIterator>::Item) -> V>;
    fn unpack_be<V: Integral>(
        self,
    ) -> Map<<Self as IntoIterator>::IntoIter, fn(<Self as IntoIterator>::Item) -> V>;
}

impl<'a, T: BitStore, O: BitOrder, I> Unpack for I
where
    I: IntoIterator<Item = &'a BitSlice<T, O>>,
    BitSlice<T, O>: BitField,
{
    fn unpack_le<V: Integral>(
        self,
    ) -> Map<<Self as IntoIterator>::IntoIter, fn(<Self as IntoIterator>::Item) -> V> {
        self.into_iter().map(|b| b.load_le())
    }

    fn unpack_be<V: Integral>(
        self,
    ) -> Map<<Self as IntoIterator>::IntoIter, fn(<Self as IntoIterator>::Item) -> V> {
        self.into_iter().map(|b| b.load_be())
    }
}
