// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/. 

use thiserror::Error;
use crate::dimensions::DimensionError;

use crate::shape::ShapeError;
use crate::texture::Texture;

#[derive(Error, Debug)]
pub enum TextureError {
    #[error("Error in file header: {0}")]
    Header(#[from] binrw::error::Error),

    #[error("IO error in file contents: {0}")]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Dimensions(#[from] DimensionError),

    #[error(transparent)]
    Shape(#[from] ShapeError),

    #[error("Unsupported format: {0}")]
    Format(String),

    #[error("Texture exceeds container's capabilities: {0}")]
    Capability(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type TextureResult<T = Texture> = Result<T, TextureError>;