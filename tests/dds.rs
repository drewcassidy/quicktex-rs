// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::env::set_current_dir;
use std::fs::File;
use std::io::Read;
use std::process::Command;

use anyhow::Result;
use tempfile::tempdir;

use quicktex::container::ContainerHeader;
use quicktex::dimensions::{Dimensioned, Dimensions};
use quicktex::format::{AlphaFormat, ColorFormat, Format};
use quicktex::shape::TextureShape;
use quicktex::*;

const IMAGE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/images");
const CUBEMAP_FACES: [&str; 6] = ["+X", "-X", "+Y", "-Y", "+Z", "-Z"];

#[test]
fn load_cubemap() -> Result<()> {
    let d = tempdir()?;
    set_current_dir(d.path())?;
    // println!("{:?}", current_dir());

    // assemble a cubemap from PNGs
    Command::new("nvassemble")
        .args(["-cube", "-noalpha", "-bgra"])
        .args(CUBEMAP_FACES.map(|suffix| format!("{IMAGE_DIR}/cubemap{suffix}.png")))
        .args(["-o", "cubemap.dds"])
        .output()?;

    let mut reader = File::open(d.path().join("cubemap.dds"))?;
    let texture = DDSHeader::read_texture(&mut reader)?;
    // println!("{header:#?}");
    // println!("{texture:#?}");

    let format = texture.format;
    let (pitch, _color_format, _alpha_format) = match format {
        Format::Uncompressed {
            pitch,
            color_format,
            alpha_format,
        } => {
            // surprise! not all nvtt binaries use the same pitch with nvassemble
            // assert_eq!(pitch, 3, "Format should be 3-byte pitch");
            assert_eq!(
                color_format,
                ColorFormat::RGB {
                    r_mask: 0xFF0000,
                    g_mask: 0xFF00,
                    b_mask: 0xFF,
                    srgb: false,
                },
                "Incorrect color format"
            );
            assert_eq!(alpha_format, AlphaFormat::Opaque, "Incorrect alpha format");
            (pitch, color_format, alpha_format)
        }
        _ => {
            panic!("Format was not `Uncompressed`");
        }
    };

    assert_eq!(texture.mips(), None, "nvassemble never generates mipmaps");
    assert_eq!(texture.layers(), None, "cubemap should not have layers");
    let faces = texture.faces().expect("missing faces");
    assert_eq!(faces.len(), 6, "incomplete cubemap");

    for (_face, surface) in texture.iter_faces() {
        let surface = surface
            .try_into_surface()
            .expect("Cubemap faces should be surface primitives");
        let buffer = &surface.buffer;
        assert_eq!(
            surface.dimensions(),
            Dimensions::try_from([128, 128])?,
            "Incorrect image dimensions"
        );
        assert_eq!(buffer.len(), 128 * 128 * pitch, "Incorrect buffer size");

        // test that the images are all loaded on the right boundaries
        // the test image has magenta pixels at the top left and bottom right corners for this reason
        assert_eq!(
            buffer[..3],
            [0xFF, 0x00, 0xFF],
            "First pixel is not magenta"
        );
        assert_eq!(
            buffer[buffer.len() - pitch..][0..3],
            [0xFF, 0x00, 0xFF],
            "Last pixel is not magenta"
        );
    }

    // make sure there's no more data to read
    let mut remainder = Vec::new();
    reader.read_to_end(&mut remainder)?;
    assert_eq!(remainder.len(), 0, "Data left unread in file");
    Ok(())
}
