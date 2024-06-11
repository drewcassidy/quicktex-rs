// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::slice;

pub trait AsSlice<T> {
    fn as_slice(&self) -> &[T];
}

impl<T> AsSlice<T> for T {
    fn as_slice(&self) -> &[T] {
        slice::from_ref(self)
    }
}

impl<T> AsSlice<T> for &[T] {
    fn as_slice(&self) -> &[T] {
        self
    }
}
