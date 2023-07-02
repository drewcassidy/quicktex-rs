use std::fs::File;
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
use clap::{arg, command, value_parser, ArgAction, Command};
use miette::IntoDiagnostic;
use quicktex::dds::DDSFile;

fn main() -> miette::Result<()> {
    let matches = command!().arg(arg!([path] "file path")).get_matches();

    if let Some(name) = matches.get_one::<String>("path") {
        let reader = File::open(name).into_diagnostic()?;
        let dds = DDSFile::new(reader)?;
        println!("{:#?}", dds.header)
    }

    Ok(())
}
