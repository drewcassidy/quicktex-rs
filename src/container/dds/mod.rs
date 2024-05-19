// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod header;
mod pixel_format;
mod dx10_header;

use std::fmt::{Debug, Write};
use std::io::{BufRead, Read, Seek};
use binrw::prelude::*;
use itertools::Itertools;
use strum::VariantArray;
use thiserror::Error;

// use crate::container::{ContainerError, Result};
use crate::container::ContainerHeader;
use crate::texture::Texture;


#[derive(Debug, Error)]
pub enum DDSError {
    #[error("Format error parsing DDS header: {0}")]
    HeaderError(#[from] binrw::error::Error),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

type DDSResult<T = ()> = Result<T, DDSError>;

use header::DDSHeader;
use crate::shape::CubeFace;

fn cubemap_order(face: &CubeFace) -> u32 {
    match face {
        CubeFace::PositiveX => { 0 }
        CubeFace::NegativeX => { 1 }
        CubeFace::PositiveY => { 2 }
        CubeFace::NegativeY => { 3 }
        CubeFace::PositiveZ => { 4 }
        CubeFace::NegativeZ => { 5 }
    }
}

fn read_mips(reader: &mut (impl Read + Seek), header: &DDSHeader) {
    todo!();
}


pub fn read_texture(reader: &mut (impl Read + Seek)) -> DDSResult<Texture> {
    let header = DDSHeader::read(reader)?;

    println!("{header:#?}");

    let format = header.format()?;

    println!("{format:#?}");


    if let Some(mut faces) = header.faces() {
        faces.sort_by_key(|f| cubemap_order(f));

        for face in faces {}
    }
    todo!()
}
