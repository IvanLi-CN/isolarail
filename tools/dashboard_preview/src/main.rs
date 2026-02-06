use embedded_graphics::mono_font::ascii::FONT_7X13_BOLD;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use std::path::PathBuf;
use std::fs;

struct Canvas {
    w: u32,
    h: u32,
    buf: Vec<Rgb565>,
}

impl Canvas {
    fn new(w: u32, h: u32, bg: Rgb565) -> Self {
        Self { w, h, buf: vec![bg; (w * h) as usize] }
    }
    fn set_px(&mut self, x: i32, y: i32, c: Rgb565) {
        if x >= 0 && y >= 0 && (x as u32) < self.w && (y as u32) < self.h {
            let idx = y as u32 * self.w + x as u32;
            self.buf[idx as usize] = c;
        }
    }
    fn to_svg(&self, path: &str) {
        use std::fs::File; use std::io::Write;
        let mut f = File::create(path).unwrap();
        writeln!(f, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
        writeln!(
            f,
            "<svg width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" xmlns=\"http://www.w3.org/2000/svg\" shape-rendering=\"crispEdges\">",
            self.w, self.h, self.w, self.h
        ).unwrap();
        // Emit every pixel as a 1x1 rect, including background, to meet per-pixel requirement
        for y in 0..self.h {
            for x in 0..self.w {
                let px = self.buf[(y * self.w + x) as usize];
                let hex = hex_color(px);
                writeln!(f, "<rect x=\"{}\" y=\"{}\" width=\"1\" height=\"1\" fill=\"#{}\"/>", x, y, hex).unwrap();
            }
        }
        writeln!(f, "</svg>").unwrap();
    }

    // PPM export removed per repository policy: keep only per-pixel SVG previews.
}

fn hex_color(c: Rgb565) -> String {
    let r = (c.r() as u32 * 255 / 31) as u8;
    let g = (c.g() as u32 * 255 / 63) as u8;
    let b = (c.b() as u32 * 255 / 31) as u8;
    format!("{:02X}{:02X}{:02X}", r, g, b)
}

impl OriginDimensions for Canvas {
    fn size(&self) -> Size { Size::new(self.w, self.h) }
}

impl DrawTarget for Canvas {
    type Color = Rgb565;
    type Error = core::convert::Infallible;
    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(Point { x, y }, color) in pixels {
            if x >= 0 && y >= 0 && (x as u32) < self.w && (y as u32) < self.h {
                let idx = y as u32 * self.w + x as u32;
                self.buf[idx as usize] = color;
            }
        }
        Ok(())
    }
}

fn draw_centered_text<D: DrawTarget<Color = Rgb565>>(
    disp: &mut D,
    col_cx: i32,
    y: i32,
    text: &str,
    style: MonoTextStyle<'_ , Rgb565>,
    adv_x: i32,
) {
    let w = (text.len() as i32) * adv_x;
    let x = col_cx - (w / 2);
    let _ = Text::with_baseline(text, Point::new(x, y), style, Baseline::Top).draw(disp);
}

fn draw_dashboard(samples: &[(bool, &str, &str, &str); 4], svg_path: &str) {
    let mut c = Canvas::new(160, 50, Rgb565::new(31, 63, 31));
    // borders and separators
    let border = PrimitiveStyle::with_stroke(Rgb565::new(0,0,0), 1);
    Rectangle::new(Point::new(0,0), Size::new(160,50)).into_styled(border).draw(&mut c).ok();
    for x in [40i32, 80, 120] {
        Rectangle::new(Point::new(x,0), Size::new(1,50)).into_styled(PrimitiveStyle::with_fill(Rgb565::new(0,0,0))).draw(&mut c).ok();
    }
    let centers = [20i32, 60, 100, 140];
    let v_style = MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(31,45,0));
    let i_style = MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(31,0,0));
    let w_style = MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,42,0));
    // rows: y=2, 16, 30 (1 px interline spacing; no header)
    for (col, cx) in centers.iter().enumerate() {
        let (conn, v, i, w) = samples[col];
        if conn {
            // outline pass
            for (dx,dy) in [(-1,0),(1,0),(0,-1),(0,1)] {
                draw_centered_text(&mut c, *cx+dx, 2+dy,  v, MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);
                draw_centered_text(&mut c, *cx+dx, 16+dy, i, MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);
                draw_centered_text(&mut c, *cx+dx, 30+dy, w, MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);
            }
            draw_centered_text(&mut c, *cx, 2,  v, v_style, 7);
            draw_centered_text(&mut c, *cx, 16, i, i_style, 7);
            draw_centered_text(&mut c, *cx, 30, w, w_style, 7);
        } else {
            // No data: black "--" without black outline (per requirement).
            let style = MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0));
            for &yy in &[2i32, 16, 30] {
                draw_centered_text(&mut c, *cx, yy, "--", style, 7);
            }
        }
        // power bar 4px high at y=45..48（清晰可见）
        Rectangle::new(Point::new([3,43,83,123][col], 45), Size::new(34,4)).into_styled(border).draw(&mut c).ok();
        if conn {
            // simple demo fill widths per column for preview
            let fills = [20u32, 30, 15, 10];
            Rectangle::new(Point::new([4,44,84,124][col], 46), Size::new(fills[col], 2))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::new(0,63,0))).draw(&mut c).ok();
        }
    }
    c.to_svg(svg_path);
}

fn draw_states_preview(svg_path: &str) {
    let mut c = Canvas::new(160, 50, Rgb565::new(31, 63, 31));
    // borders and separators
    let border = PrimitiveStyle::with_stroke(Rgb565::new(0,0,0), 1);
    Rectangle::new(Point::new(0,0), Size::new(160,50)).into_styled(border).draw(&mut c).ok();
    for x in [40i32, 80, 120] {
        Rectangle::new(Point::new(x,0), Size::new(1,50)).into_styled(PrimitiveStyle::with_fill(Rgb565::new(0,0,0))).draw(&mut c).ok();
    }
    let centers = [20i32, 60, 100, 140];
    let v_style = MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(31,45,0));
    let i_style = MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(31,0,0));
    let w_style = MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,42,0));

    // Helper: draw icon from 32x32 .raw mask (ASCII '0'/'1' x32 lines)
    fn draw_icon_raw(c: &mut Canvas, left: i32, top: i32, raw: &str, color: Rgb565) {
        for (y, line) in raw.lines().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                if ch == '1' {
                    c.set_px(left + x as i32, top + y as i32, color);
                }
            }
        }
    }
    // Helper: draw scaled (dw x dh) from 32x32 raw using nearest mapping
    fn draw_icon_raw_scaled(c: &mut Canvas, left: i32, top: i32, raw32: &str, dw: usize, dh: usize) {
        let lines: Vec<&str> = raw32.lines().collect();
        for oy in 0..dh {
            let src_y = (oy * 32) / dh;
            let line = lines.get(src_y).copied().unwrap_or("");
            let chars: Vec<char> = line.chars().collect();
            for ox in 0..dw {
                let src_x = (ox * 32) / dw;
                let ch = chars.get(src_x).copied().unwrap_or('0');
                if ch == '1' {
                    c.set_px(left + ox as i32, top + oy as i32, Rgb565::new(0,0,0));
                }
            }
        }
    }
    fn load_raw(path: &PathBuf) -> String { fs::read_to_string(path).expect("read raw") }

    // Column 0: Disconnected (32x32 icon + "DISC"; no power bar at all)
    let cx0 = centers[0];
    let icon0_left = cx0 - 16; let icon0_top  = 2;
    // Disconnected uses rivet-icons_close-circle-solid.raw
    let disc_raw = load_raw(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/rivet-icons_close-circle-solid.raw"));
    draw_icon_raw(&mut c, icon0_left, icon0_top, &disc_raw, Rgb565::new(0,0,0));
    // Label without outline (black text should not have black stroke)
    draw_centered_text(&mut c, cx0, 36, "DISC", MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);

    // Column 1: Overcurrent (show values + red CC badge on power row)
    let cx1 = centers[1];
    for (dx,dy) in [(-1,0),(1,0),(0,-1),(0,1)] {
        draw_centered_text(&mut c, cx1+dx, 2+dy,  "9.00V", MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);
        draw_centered_text(&mut c, cx1+dx, 16+dy, "2.50A", MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);
    }
    draw_centered_text(&mut c, cx1, 2,  "9.00V", v_style, 7);
    draw_centered_text(&mut c, cx1, 16, "2.50A", i_style, 7);
    // No power value when CC icon is present
    // CC icon: use canonical 32x32 mask and draw scaled to 24x24
    let cc_raw = load_raw(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fa7-solid_closed-captioning.raw"));
    // Place 24x24 centered over power row area (move down by ~5px but avoid clipping)
    // Max top without clipping is 26 (26..49). Using 26 for full visibility.
    draw_icon_raw_scaled(&mut c, cx1 - 12, 26, &cc_raw, 24, 24);

    // Column 2: Closed (32x32 plug-disconnected icon + "OFF"; no power bar at all)
    let cx2 = centers[2];
    let icon2_left = cx2 - 16; let icon2_top  = 2;
    // Closed uses fluent_plug-disconnected-16-filled.raw
    let off_raw = load_raw(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/fluent_plug-disconnected-16-filled.raw"));
    draw_icon_raw(&mut c, icon2_left, icon2_top, &off_raw, Rgb565::new(0,0,0));
    // Label without outline
    draw_centered_text(&mut c, cx2, 36, "OFF", MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);

    // Column 3: Initializing (three lines of "--"; power bar outline is ok)
    let cx3 = centers[3];
    let style = MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0));
    for &yy in &[2i32, 16, 30] {
        // no outline for black "--"
        draw_centered_text(&mut c, cx3, yy, "--", style, 7);
    }
    Rectangle::new(Point::new(123, 45), Size::new(34,4)).into_styled(border).draw(&mut c).ok();

    c.to_svg(svg_path);
}

fn main() {
    // Resolve output directory to repo-root/docs/assets using manifest dir
    let base: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/assets");
    let out_normal = base.join("dashboard_wireframe_160x50_color_bold.svg");
    let out_states = base.join("dashboard_wireframe_160x50_states_color_bold.svg");
    // All online for the normal preview
    let normal = [
        (true,  "5.12V", "980mA", "5.0W"),
        (true,  "9.00V", "2.50A", "22.5W"),
        (true,  "20.0V", "1.50A", "30.0W"),
        (true,  "12.0V", "1.00A", "12.0W"),
    ];
    draw_dashboard(&normal, out_normal.to_str().unwrap());
    draw_states_preview(out_states.to_str().unwrap());
}
