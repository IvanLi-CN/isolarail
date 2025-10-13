use embedded_graphics::mono_font::ascii::{FONT_4X6, FONT_7X13_BOLD};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};

struct Canvas {
    w: u32,
    h: u32,
    buf: Vec<Rgb565>,
}

impl Canvas {
    fn new(w: u32, h: u32, bg: Rgb565) -> Self {
        Self { w, h, buf: vec![bg; (w * h) as usize] }
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

    fn to_ppm(&self, path: &str) {
        use std::fs::File; use std::io::Write;
        let mut f = File::create(path).unwrap();
        // P6 binary PPM, 8-bit per channel
        writeln!(f, "P6").unwrap();
        writeln!(f, "{} {}", self.w, self.h).unwrap();
        writeln!(f, "255").unwrap();
        for y in 0..self.h {
            for x in 0..self.w {
                let px = self.buf[(y * self.w + x) as usize];
                let r = (px.r() as u32 * 255 / 31) as u8;
                let g = (px.g() as u32 * 255 / 63) as u8;
                let b = (px.b() as u32 * 255 / 31) as u8;
                let _ = f.write_all(&[r, g, b]);
            }
        }
    }
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

fn draw_dashboard(samples: &[(bool, &str, &str, &str); 4], svg_path: &str, ppm_path: &str) {
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
    // rows: y=2, 17, 32 (2 px interline spacing; no header)
    for (col, cx) in centers.iter().enumerate() {
        let (conn, v, i, w) = samples[col];
        if conn {
            // outline pass
            for (dx,dy) in [(-1,0),(1,0),(0,-1),(0,1)] {
                draw_centered_text(&mut c, *cx+dx, 2+dy,  v, MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);
                draw_centered_text(&mut c, *cx+dx, 17+dy, i, MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);
                draw_centered_text(&mut c, *cx+dx, 32+dy, w, MonoTextStyle::new(&FONT_7X13_BOLD, Rgb565::new(0,0,0)), 7);
            }
            draw_centered_text(&mut c, *cx, 2,  v, v_style, 7);
            draw_centered_text(&mut c, *cx, 17, i, i_style, 7);
            draw_centered_text(&mut c, *cx, 32, w, w_style, 7);
        } else {
            let header_style = MonoTextStyle::new(&FONT_4X6, Rgb565::new(0,0,0));
            draw_centered_text(&mut c, *cx, 2,  "--", header_style, 4);
            draw_centered_text(&mut c, *cx, 17, "--", header_style, 4);
            draw_centered_text(&mut c, *cx, 32, "--", header_style, 4);
        }
        // power bar 2px high at y=48..49
        Rectangle::new(Point::new([3,43,83,123][col], 48), Size::new(34,2)).into_styled(border).draw(&mut c).ok();
        if conn && col<3 {
            Rectangle::new(Point::new([4,44,84,124][col], 49), Size::new([20,30,15,0][col] as u32,1))
                .into_styled(PrimitiveStyle::with_fill(Rgb565::new(0,63,0))).draw(&mut c).ok();
        }
    }
    c.to_svg(svg_path);
    c.to_ppm(ppm_path);
}

fn main() {
    let normal = [
        (true,  "5.12V", "980mA", "5.0W"),
        (true,  "9.00V", "2.50A", "22.5W"),
        (true,  "20.0V", "1.50A", "30.0W"),
        (false, "--",    "--",    "--"),
    ];
    draw_dashboard(
        &normal,
        "../../docs/assets/dashboard_wireframe_160x50_color_bold.svg",
        "../../docs/assets/dashboard_wireframe_160x50_color_bold.ppm",
    );
    let disconnected = [
        (false, "--", "--", "--"),
        (false, "--", "--", "--"),
        (false, "--", "--", "--"),
        (false, "--", "--", "--"),
    ];
    draw_dashboard(
        &disconnected,
        "../../docs/assets/dashboard_wireframe_160x50_disconnected_color_bold.svg",
        "../../docs/assets/dashboard_wireframe_160x50_disconnected_color_bold.ppm",
    );
}
