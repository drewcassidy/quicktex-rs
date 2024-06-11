// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt::Debug;
use std::io::{Read, Seek, Write};

use binrw::{BinRead, BinReaderExt, BinWrite, BinWriterExt};

use crate::dimensions::Dimensions;
use crate::error::TextureResult;
use crate::format::Format;
use crate::shape::CubeFace;
use crate::texture::{Surfaces, Texture};

pub mod dds;

pub trait ContainerHeader: Sized + Clone + Debug + BinRead + BinWrite
where
    for<'a> <Self as BinRead>::Args<'a>: Default,
    for<'a> <Self as BinWrite>::Args<'a>: Default,
{
    type Args: Default;

    fn read_surfaces<R: Read + Seek>(&self, reader: &mut R) -> TextureResult<Surfaces>;
    fn write_surfaces<W: Write + Seek>(
        &self,
        writer: &mut W,
        surfaces: Surfaces,
    ) -> TextureResult<()>;

    fn for_texture(texture: &Texture) -> TextureResult<Self> {
        Self::for_texture_args(texture, &Default::default())
    }
    fn for_texture_args(
        texture: &Texture,
        args: &<Self as ContainerHeader>::Args,
    ) -> TextureResult<Self>;

    fn read_texture<R: Read + Seek>(reader: &mut R) -> TextureResult<Texture> {
        let header: Self = reader.read_le()?;
        let format = header.format()?;
        let surfaces = header.read_surfaces(reader)?;
        Ok(Texture { format, surfaces })
    }

    fn write_texture<W: Write + Seek>(writer: &mut W, texture: &Texture) -> TextureResult<()> {
        Self::write_texture_args(writer, texture, &Default::default())
    }

    fn write_texture_args<W>(
        writer: &mut W,
        texture: &Texture,
        args: &<Self as ContainerHeader>::Args,
    ) -> TextureResult<()>
    where
        W: Write + Seek,
    {
        let header: Self = Self::for_texture_args(texture, args)?;
        writer.write_le(&header)?;
        header.write_surfaces(writer, texture.clone().surfaces)
    }

    fn dimensions(&self) -> TextureResult<Dimensions>;
    fn layers(&self) -> TextureResult<Option<usize>>;
    fn faces(&self) -> TextureResult<Option<Vec<CubeFace>>>;
    fn mips(&self) -> TextureResult<Option<usize>>;
    fn format(&self) -> TextureResult<Format>;
}
