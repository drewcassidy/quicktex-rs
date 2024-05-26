use std::fmt::{Display, Formatter};
use std::fs::File;
use quicktex::container::dds;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

struct Foo2 {}

fn main() {
    let mut the_file = File::open("/Users/drewcassidy/Downloads/bluedog_Apollo_APAS75.dds").unwrap();
    let the_dds = dds::read_texture(&mut the_file);
}
