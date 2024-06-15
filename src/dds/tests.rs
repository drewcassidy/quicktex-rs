// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fs::File;
use std::io::Read;

use anyhow::Result;

use crate::container::ContainerHeader;
use crate::dimensions::{Dimensioned, Dimensions};
use crate::format::{AlphaFormat, ColorFormat, Format};
use crate::shape::{CubeFace, TextureShape};

use super::DDSHeader;

const DDS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/images/dds");

#[test]
/// Read a cubemap made using nvassemble.
fn read_cubemap() -> Result<()> {
    let cubepath = format!("{DDS_DIR}/cubemap.dds");

    let mut reader = File::open(cubepath)?;
    let texture = DDSHeader::read_texture(&mut reader)?;

    let format = texture.format;
    assert_eq!(
        format,
        Format::Uncompressed {
            pitch: 3,
            color_format: ColorFormat::RGB {
                r_mask: 0xFF,
                g_mask: 0xFF00,
                b_mask: 0xFF0000,
                srgb: false
            },
            alpha_format: AlphaFormat::Opaque
        }
    );

    assert_eq!(texture.mips(), None, "nvassemble never generates mipmaps");
    assert_eq!(texture.layers(), None, "cubemap should not have layers");
    let faces = texture.faces().expect("missing faces");
    assert_eq!(faces.len(), 6, "incomplete cubemap");

    for (face, surface) in texture.iter_faces() {
        let surface = surface
            .try_into_surface()
            .expect("Cubemap faces should be surface primitives");
        let face = face.unwrap();
        let buffer = &surface.buffer;
        assert_eq!(
            surface.dimensions(),
            Dimensions::try_from([128, 128])?,
            "Incorrect image dimensions"
        );
        assert_eq!(buffer.len(), 128 * 128 * 3, "Incorrect buffer size");

        let magenta = [0xFF, 0x00, 0xFF];
        assert_eq!(buffer[..3], magenta, "First pixel is not magenta");
        assert_eq!(
            buffer[buffer.len() - 3..],
            magenta,
            "Last pixel is not magenta"
        );

        let (expected_bg, bg_index) = match face {
            CubeFace::PositiveX => ([0xFF, 0xBC, 0xBC], 32), //red
            CubeFace::NegativeX => ([0xBC, 0xFF, 0xFF], 32), //cyan
            CubeFace::PositiveY => ([0xBC, 0xFF, 0xBC], 32), //green
            CubeFace::NegativeY => ([0xFF, 0xBC, 0xFF], 32), //magenta
            CubeFace::PositiveZ => ([0xBC, 0xBC, 0xFF], 31), //blue
            CubeFace::NegativeZ => ([0xFF, 0xFF, 0xBC], 31), //yellow
        };
        let bg_index = bg_index * 3;
        let bg = &buffer[bg_index..bg_index + 3];
        assert_eq!(*bg, expected_bg, "Background color incorrect for {face:?}");
    }

    // make sure there's no more data to read
    let mut remainder = Vec::new();
    reader.read_to_end(&mut remainder)?;
    assert_eq!(remainder.len(), 0, "Data left unread in file");
    Ok(())
}
