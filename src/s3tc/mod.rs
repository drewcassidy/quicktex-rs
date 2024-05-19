// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub mod bc1;
pub mod bc3;
pub mod bc4;
pub mod bc5;

#[derive(Clone, Debug)]
pub enum S3TCFormat {
    BC1 { srgb: bool },
    BC2 { srgb: bool },
    BC3 { srgb: bool },
    BC4 { signed: bool },
    BC5 { signed: bool },
} 