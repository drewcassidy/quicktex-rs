// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;
use std::io::{Read, Seek};

use thiserror::Error;

use crate::dimensions::Dimensions;
use crate::format::Format;
use crate::shape::CubeFace;
use crate::texture::Texture;

pub mod dds;

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Error {1} DDS file: {0}")]
    DDSError(dds::DDSError, &'static str)
}

pub trait IntoContainerError: Sized {
    fn into(self, op: &'static str) -> ContainerError;
    fn into_read(self) -> ContainerError { self.into("reading") }
    fn into_write(self) -> ContainerError { self.into("writing") }
}

trait ContainerHeader: Sized + Clone + Debug {
    type Error: IntoContainerError;

    fn read_with<R: Read + Seek>(&self, reader: &mut R) -> Result<Texture, Self::Error>;

    fn dimensions(&self) -> Result<Dimensions, Self::Error>;
    fn layers(&self) -> Result<Option<usize>, Self::Error>;
    fn faces(&self) -> Result<Option<Vec<CubeFace>>, Self::Error>;
    fn mips(&self) -> Result<Option<usize>, Self::Error>;
    fn format(&self) -> Result<Format, Self::Error>;
}
