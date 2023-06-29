// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use funty::{Integral, Numeric};
use miette::IntoDiagnostic;
use miette::Result;
use std::io::Read;
use Default;

pub fn div_ceil<T: Integral>(lhs: T, rhs: T) -> T {
    let d = lhs / rhs;
    let r = rhs % rhs;
    if (r > T::ZERO && rhs > T::ZERO) || (r < T::ZERO && rhs < T::ZERO) {
        d + T::ONE
    } else {
        d
    }
}

pub trait ReadExt: Read {
    fn load_le<T: Numeric + Integral>(&mut self) -> Result<T>
    where
        <T as Numeric>::Bytes: AsMut<[u8]> + Default,
    {
        let mut buf: T::Bytes = Default::default();
        self.read_exact(buf.as_mut()).into_diagnostic()?;

        Ok(T::from_le_bytes(buf))
    }

    fn load_array_le<T: Numeric + Integral, const N: usize>(&mut self) -> Result<[T; N]>
    where
        <T as Numeric>::Bytes: AsMut<[u8]> + Default,
    {
        let mut res = [T::ZERO; N];

        for item in &mut res {
            *item = self.load_le()?;
        }

        Ok(res)
    }
}

impl<R> ReadExt for R where R: Read {}
