use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tiny_skia::Pixmap;

fn render_svg_to_pixmap(svg_path: &Path, size: u32) -> Result<Pixmap> {
    let svg_data = fs::read(svg_path).with_context(|| format!("read {:?}", svg_path))?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &opt).context("parse svg")?;

    let mut pixmap = Pixmap::new(size, size).context("pixmap")?;
    // Fit SVG viewbox into 32x32
    // Render fitted to 32x32
    let mut pm = pixmap.as_mut();
    resvg::render(&tree, resvg::FitTo::Size(size, size), &mut pm);
    Ok(pixmap)
}

fn write_raw_mask(pixmap: &Pixmap, out_path: &Path) -> Result<()> {
    let mut s = String::with_capacity((pixmap.width() * (pixmap.height() + 1)) as usize);
    for y in 0..pixmap.height() {
        for x in 0..pixmap.width() {
            let p = pixmap.pixel(x, y).unwrap();
            // treat non-transparent (alpha>0.5) as inked
            let a = p.alpha();
            let v = if a > 128 { 1 } else { 0 };
            s.push(if v == 1 { '1' } else { '0' });
        }
        s.push('\n');
    }
    fs::write(out_path, s).with_context(|| format!("write {:?}", out_path))?;
    Ok(())
}

fn main() -> Result<()> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../");
    let src_dir = repo_root.join("assets/icons_src");
    let out_dir = repo_root.join("assets");
    fs::create_dir_all(&src_dir).ok();
    fs::create_dir_all(&out_dir).ok();

    let jobs = [
        ("close-circle-solid.svg", "rivet-icons_close-circle-solid.raw"),
        ("closed-captioning-solid.svg", "fa7-solid_closed-captioning.raw"),
        ("plug-disconnected-16-filled.svg", "fluent_plug-disconnected-16-filled.raw"),
    ];
    for (svg, raw) in jobs {
        let svg_path = src_dir.join(svg);
        let raw_path = out_dir.join(raw);
        let pix = render_svg_to_pixmap(&svg_path, 32)?;
        write_raw_mask(&pix, &raw_path)?;
        eprintln!("wrote {}", raw_path.display());
    }
    Ok(())
}
