use anyhow::{Context, Result};
use image::{io::Reader as ImageReader, GenericImageView};
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input = PathBuf::from(args.next().context("png path")?);
    let output = PathBuf::from(args.next().context("raw path")?);
    let img = ImageReader::open(&input)?.with_guessed_format()?.decode()?;
    if img.width() != 32 || img.height() != 32 {
        eprintln!("warning: input not 32x32, got {}x{}", img.width(), img.height());
    }
    let mut s = String::with_capacity(32 * (32 + 1));
    for y in 0..32u32 {
        for x in 0..32u32 {
            let p = img.get_pixel(x.min(img.width() - 1), y.min(img.height() - 1));
            // If alpha > 0 and luminance < 200 -> inked
            let a = p[3] as u32;
            let lum = (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000;
            let v = if a > 128 && lum < 200 { '1' } else { '0' };
            s.push(v);
        }
        s.push('\n');
    }
    fs::write(&output, s)?;
    eprintln!("wrote {}", output.display());
    Ok(())
}

