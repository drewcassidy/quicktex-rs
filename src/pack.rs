use bitvec::field::BitField;
use bitvec::macros::internal::funty::Integral;
use bitvec::order::BitOrder;
use bitvec::prelude::BitSlice;
use bitvec::store::BitStore;
use std::iter::{zip, Map};

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
        for (src, mut dst) in zip(unpacked, self) {
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
        for (src, mut dst) in zip(unpacked, self) {
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
