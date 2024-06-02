// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;
use std::io::{Read, Seek};

use crate::dimensions::Dimensions;
use crate::error::{TextureError, TextureResult};
use crate::format::Format;
use crate::shape::CubeFace;

pub mod dds;


trait ContainerHeader: Sized + Clone + Debug {
    fn read_with<R: Read + Seek>(&self, reader: &mut R) -> TextureResult;

    fn dimensions(&self) -> TextureResult<Dimensions>;
    fn layers(&self) -> TextureResult<Option<usize>>;
    fn faces(&self) -> TextureResult<Option<Vec<CubeFace>>>;
    fn mips(&self) -> TextureResult<Option<usize>>;
    fn format(&self) -> TextureResult<Format>;
}
