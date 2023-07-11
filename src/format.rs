// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[derive(Clone, Copy)]
pub struct Foo {}

impl ImageFormat for Foo {
    fn name(&self) -> String {
        todo!()
    }
}

#[enum_dispatch::enum_dispatch(ImageFormat for Format)]
mod inner {
    use crate::format::Foo;

    pub trait ImageFormat {
        fn name(&self) -> String;
    }

    #[derive(Clone, Copy)]
    pub enum Format {
        Foo,
    }
}

pub use inner::*;
