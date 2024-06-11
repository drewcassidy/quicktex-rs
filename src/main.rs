// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use quicktex::container::{dds, ContainerHeader};
use std::fs::File;

fn main() {
    let mut the_file = File::open("/Users/drewcassidy/Downloads/cmft_cubemap.dds").unwrap();
    let the_dds = dds::DDSHeader::read_texture(&mut the_file).unwrap();
    println!("{the_dds:#?}");
    let new_header = dds::DDSHeader::for_texture(&the_dds);
    println!("{new_header:#?}");
}
