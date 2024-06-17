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

/// A header for a texture container. Contains information about dimensions, shape, and texture format,
/// but does not contain any actual texture data.
pub trait ContainerHeader: Sized + Clone + Debug + BinRead + BinWrite
where
    for<'a> <Self as BinRead>::Args<'a>: Default,
    for<'a> <Self as BinWrite>::Args<'a>: Default,
{
    type Args: Default;

    /// Read a texture in this container type using the provided reader. The header object is not exposed
    fn read_texture<R: Read + Seek>(reader: &mut R) -> TextureResult<Texture> {
        let header: Self = reader.read_le()?;
        header.to_texture(reader)
    }

    /// Write a texture in this container type using the provided writer and default arguments.
    /// The header object is not exposed
    fn write_texture<W: Write + Seek>(writer: &mut W, texture: &Texture) -> TextureResult<()> {
        Self::write_texture_args(writer, texture, &Default::default())
    }

    /// Write a texture in this container type using the provided writer and [`Self::Args`].
    /// The header object is not exposed
    fn write_texture_args<W>(
        writer: &mut W,
        texture: &Texture,
        args: &<Self as ContainerHeader>::Args,
    ) -> TextureResult<()>
    where
        W: Write + Seek,
    {
        let header: Self = Self::from_texture_args(texture, args)?;
        writer.write_le(&header)?;
        header.write_surfaces(writer, texture.clone().surfaces)
    }

    /// read the surfaces associated with this header using the provided reader
    fn read_surfaces<R: Read + Seek>(&self, reader: &mut R) -> TextureResult<Surfaces>;

    /// Write surfaces associated with this header using the provided writer
    fn write_surfaces<W: Write + Seek>(
        &self,
        writer: &mut W,
        surfaces: Surfaces,
    ) -> TextureResult<()>;

    /// Convert this header into a texture using the provided reader
    fn to_texture<R: Read + Seek>(&self, reader: &mut R) -> TextureResult<Texture> {
        let format = self.format()?;
        let surfaces = self.read_surfaces(reader)?;
        Ok(Texture { format, surfaces })
    }

    /// Create a new header for a texture using default arguments
    fn from_texture(texture: &Texture) -> TextureResult<Self> {
        Self::from_texture_args(texture, &Default::default())
    }

    /// Create a new header for a texture using [`Self::Args`]
    fn from_texture_args(
        texture: &Texture,
        args: &<Self as ContainerHeader>::Args,
    ) -> TextureResult<Self>;

    /// Get the dimensions indicated by this container header
    fn dimensions(&self) -> TextureResult<Dimensions>;

    /// Get the layer count indicated by this container header
    fn layers(&self) -> TextureResult<Option<usize>>;

    /// Get the cubemap faces indicated by this container header
    fn faces(&self) -> TextureResult<Option<Vec<CubeFace>>>;

    /// Get the mipmap count indicated by this container header
    fn mips(&self) -> TextureResult<Option<usize>>;

    /// Get the texture format indicated by this container header
    fn format(&self) -> TextureResult<Format>;
}
