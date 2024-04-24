// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufRead, Seek};
use crate::texture::{Texture};
use thiserror::Error;

pub mod dds;


#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Unexpected signature: '{0}'")]
    Signature(String),

    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
}

pub(crate) type Result<T, E = ContainerError> = core::result::Result<T, E>;

trait Container: Sized + Clone {
    type Header;

    fn load<R: BufRead + Seek>(&self, reader: R) -> Result<Self>;

    fn header(&self) -> &Self::Header;
    fn texture(&self) -> &Texture;
}
