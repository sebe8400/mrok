//! Wszystkie tekstury i dzwieki sa generowane proceduralnie.
//! Zero zewnetrznych assetow = mala paczka i pelna kontrola nad stylem.

use macroquad::audio::{load_sound_from_bytes, Sound};
use macroquad::prelude::*;

use crate::rng::{fbm, hash2, Rng};

fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::new(r, g, b, 1.0)
}

fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color::new(r, g, b, a)
}

fn shade(c: Color, f: f32) -> Color {
    Color::new(
        (c.r * f).clamp(0.0, 1.0),
        (c.g * f).clamp(0.0, 1.0),
        (c.b * f).clamp(0.0, 1.0),
        c.a,
    )
}

struct Canvas {
    w: i32,
    h: i32,
    px: Vec<Color>,
}

impl Canvas {
    fn new(w: i32, h: i32) -> Self {
        Self {
            w,
            h,
            px: vec![BLANK; (w * h) as usize],
        }
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }

    #[inline]
    fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    fn put(&mut self, x: i32, y: i32, c: Color) {
        if !self.inside(x, y) || c.a <= 0.0 {
            return;
        }
        let i = self.idx(x, y);
        let dst = self.px[i];
        let sa = c.a.clamp(0.0, 1.0);
        let da = dst.a;
        let oa = sa + da * (1.0 - sa);
        if oa <= 0.0001 {
            return;
        }
        self.px[i] = Color::new(
            (c.r * sa + dst.r * da * (1.0 - sa)) / oa,
            (c.g * sa + dst.g * da * (1.0 - sa)) / oa,
            (c.b * sa + dst.b * da * (1.0 - sa)) / oa,
            oa,
        );
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        for yy in y..(y + h) {
            for xx in x..(x + w) {
                self.put(xx, yy, c);
            }
        }
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Color) {
        self.ellipse(cx, cy, r, r, c);
    }

    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: Color) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        let x0 = (cx - rx).floor() as i32;
        let x1 = (cx + rx).ceil() as i32;
        let y0 = (cy - ry).floor() as i32;
        let y1 = (cy + ry).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    self.put(x, y, c);
                }
            }
        }
    }

    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Color) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.put(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn noise_erode(&mut self, seed: u32, strength: f32) {
        let src = self.px.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                let i = self.idx(x, y);
                if src[i].a < 0.1 {
                    continue;
                }
                let mut edge = false;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if !self.inside(nx, ny) || src[self.idx(nx, ny)].a < 0.1 {
                        edge = true;
                        break;
                    }
                }
                if edge && hash2(x, y, seed) < strength {
                    self.px[i].a = 0.0;
                }
            }
        }
    }

    fn outline(&mut self, c: Color) {
        let src = self.px.clone();
        for y in 0..self.h {
            for x in 0..self.w {
                let i = self.idx(x, y);
                if src[i].a > 0.15 {
                    continue;
                }
                let mut near = false;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x + dx;
                        let ny = y + dy;
                        if self.inside(nx, ny) && src[self.idx(nx, ny)].a > 0.3 {
                            near = true;
                        }
                    }
                }
                if near {
                    self.px[i] = c;
                }
            }
        }
    }

    fn grain(&mut self, seed: u32, amount: f32) {
        for y in 0..self.h {
            for x in 0..self.w {
                let i = self.idx(x, y);
                if self.px[i].a < 0.05 {
                    continue;
                }
                let n = (hash2(x, y, seed) - 0.5) * amount;
                self.px[i] = Color::new(
                    (self.px[i].r + n).clamp(0.0, 1.0),
                    (self.px[i].g + n).clamp(0.0, 1.0),
                    (self.px[i].b + n).clamp(0.0, 1.0),
                    self.px[i].a,
                );
            }
        }
    }

    fn to_texture(&self, filter: FilterMode) -> Texture2D {
        let mut img = Image::gen_image_color(self.w as u16, self.h as u16, BLANK);
        for y in 0..self.h {
            for x in 0..self.w {
                img.set_pixel(x as u32, y as u32, self.px[self.idx(x, y)]);
            }
        }
        let tex = Texture2D::from_image(&img);
        tex.set_filter(filter);
        tex
    }
}

fn floor_texture(variant: usize) -> Texture2D {
    let seed = 4000 + variant as u32 * 71;
    let mut c = Canvas::new(48, 48);
    let mut rng = Rng::new(seed as u64 + 17);

    for y in 0..48 {
        for x in 0..48 {
            let fx = x as f32;
            let fy = y as f32;
            let n = fbm(fx * 0.09, fy * 0.09, seed, 4);
            let fine = hash2(x, y, seed + 9) * 0.035;
            let mut base = 0.145 + n * 0.10 + fine;
            let tint = match variant {
                0 => (1.02, 1.0, 0.93),
                1 => (0.94, 0.90, 0.79),
                2 => (0.82, 0.98, 0.87),
                _ => (1.08, 0.86, 0.80),
            };
            if x == 0 || y == 0 || x == 47 || y == 47 {
                base *= 0.52;
            }
            if (x == 1 || y == 1) && (x != 0 && y != 0) {
                base *= 0.82;
            }
            let col = rgb(base * tint.0, base * tint.1, base * tint.2);
            c.put(x, y, col);
        }
    }

    // zabrudzenia i plamy
    for _ in 0..5 {
        let cx = rng.range(4.0, 44.0);
        let cy = rng.range(4.0, 44.0);
        let rx = rng.range(3.0, 11.0);
        let ry = rng.range(2.0, 8.0);
        c.ellipse(cx, cy, rx, ry, rgba(0.06, 0.035, 0.025, 0.30));
    }

    // pekniecia
    for _ in 0..3 {
        let mut x = rng.range(3.0, 45.0) as i32;
        let mut y = rng.range(3.0, 45.0) as i32;
        for _ in 0..rng.usize(10) + 5 {
            c.put(x, y, rgba(0.02, 0.015, 0.015, 0.55));
            x += rng.usize(3) as i32 - 1;
            y += if rng.chance(0.5) { 1 } else { -1 };
            x = x.clamp(1, 46);
            y = y.clamp(1, 46);
        }
    }

    // krew / smary
    if variant != 0 || rng.chance(0.6) {
        let bx = rng.range(10.0, 38.0);
        let by = rng.range(10.0, 38.0);
        c.ellipse(
            bx,
            by,
            rng.range(5.0, 11.0),
            rng.range(3.0, 7.0),
            rgba(0.24, 0.015, 0.015, 0.55),
        );
        for _ in 0..7 {
            let a = rng.range(0.0, std::f32::consts::TAU);
            let d = rng.range(3.0, 13.0);
            c.circle(
                bx + a.cos() * d,
                by + a.sin() * d,
                rng.range(0.7, 2.4),
                rgba(0.20, 0.01, 0.01, 0.55),
            );
        }
    }

    match variant {
        1 => {
            // pojedyncze zeby wcisniete w beton
            for i in 0..3 {
                c.rect(
                    9 + i * 6,
                    31 + (i % 2) * 2,
                    3,
                    5,
                    rgba(0.62, 0.58, 0.46, 0.75),
                );
            }
        }
        2 => {
            // kratka sciekowa
            c.rect(16, 18, 16, 12, rgba(0.03, 0.03, 0.03, 0.8));
            for i in 0..5 {
                c.rect(17 + i * 3, 19, 1, 10, rgba(0.16, 0.15, 0.13, 0.9));
            }
        }
        3 => {
            // dwie stopy
            c.ellipse(17.0, 30.0, 3.2, 6.0, rgba(0.18, 0.02, 0.02, 0.5));
            c.ellipse(30.0, 26.0, 3.0, 5.5, rgba(0.18, 0.02, 0.02, 0.5));
            c.ellipse(30.0, 20.0, 2.5, 3.0, rgba(0.18, 0.02, 0.02, 0.35));
        }
        _ => {}
    }

    c.to_texture(FilterMode::Nearest)
}

fn draw_teeth(c: &mut Canvas, x0: i32, y0: i32, count: i32, step: i32, down: bool) {
    for i in 0..count {
        let x = x0 + i * step;
        let h = 3 + (i % 2);
        if down {
            c.rect(x, y0, 2, h, rgba(0.72, 0.68, 0.54, 0.95));
        } else {
            c.rect(x, y0 - h, 2, h, rgba(0.72, 0.68, 0.54, 0.95));
        }
    }
}

fn wall_texture(variant: usize) -> Texture2D {
    let seed = 7000 + variant as u32 * 131;
    let mut c = Canvas::new(48, 48);
    let mut rng = Rng::new(seed as u64 + 91);

    for y in 0..48 {
        for x in 0..48 {
            let fx = x as f32;
            let fy = y as f32;
            let n = fbm(fx * 0.12, fy * 0.12, seed, 4);
            let row = (fy / 12.0).floor() as i32;
            let offset = if row % 2 == 0 { 0.0 } else { 12.0 };
            let local_x = (fx + offset) % 24.0;
            let mortar = (fy % 12.0) < 1.2 || local_x < 1.2;
            let mut base = 0.16 + n * 0.09;
            if mortar {
                base *= 0.52;
            }
            let tint = if variant == 3 {
                (1.12, 0.93, 0.80)
            } else {
                (1.14, 0.82, 0.70)
            };
            let mut col = rgb(base * tint.0, base * tint.1, base * tint.2);
            // organiczne zyly
            let vein = fbm(fx * 0.22, fy * 0.18, seed + 33, 3);
            if vein > 0.63 {
                col = rgb(0.30, 0.075, 0.06);
            } else if vein > 0.58 {
                col = rgb(0.24, 0.14, 0.10);
            }
            // ciemniejszy dol
            let dark = 1.0 - (fy / 48.0) * 0.25;
            c.put(x, y, shade(col, dark));
        }
    }

    // sluz / zaciek
    for _ in 0..3 {
        let x = rng.range(4.0, 44.0) as i32;
        let y = rng.range(0.0, 12.0) as i32;
        let len = rng.usize(18) as i32 + 6;
        for i in 0..len {
            c.put(x + (i / 5), y + i, rgba(0.20, 0.02, 0.02, 0.35));
        }
    }

    match variant {
        1 => {
            // wielkie oko
            c.ellipse(24.0, 21.0, 14.0, 9.0, rgb(0.42, 0.10, 0.09));
            c.ellipse(24.0, 21.0, 11.0, 6.8, rgb(0.62, 0.56, 0.43));
            c.ellipse(24.0, 21.0, 6.0, 5.0, rgb(0.33, 0.06, 0.04));
            c.circle(24.0, 21.0, 2.4, rgb(0.01, 0.005, 0.005));
            c.circle(22.5, 19.5, 1.0, rgba(0.9, 0.85, 0.7, 0.8));
            for i in 0..5 {
                let a = i as f32 * 1.3;
                c.line(
                    (24.0 + a.cos() * 10.0) as i32,
                    (21.0 + a.sin() * 6.0) as i32,
                    (24.0 + a.cos() * 16.0) as i32,
                    (21.0 + a.sin() * 10.0) as i32,
                    rgba(0.28, 0.04, 0.03, 0.7),
                );
            }
        }
        2 => {
            // paszcza w scianie
            c.ellipse(24.0, 33.0, 17.0, 8.0, rgb(0.03, 0.005, 0.005));
            c.ellipse(24.0, 33.0, 13.0, 5.2, rgb(0.22, 0.02, 0.02));
            draw_teeth(&mut c, 11, 29, 10, 3, true);
            draw_teeth(&mut c, 12, 37, 9, 3, false);
            // slina
            c.line(17, 38, 18, 46, rgba(0.55, 0.5, 0.4, 0.35));
            c.line(31, 37, 30, 45, rgba(0.55, 0.5, 0.4, 0.35));
            c.ellipse(17.0, 12.0, 2.0, 2.6, rgb(0.55, 0.5, 0.4));
            c.circle(17.0, 12.0, 0.8, BLACK);
            c.ellipse(33.0, 9.0, 1.6, 2.0, rgb(0.5, 0.45, 0.36));
            c.circle(33.0, 9.0, 0.6, BLACK);
        }
        3 => {
            // twarz z bliznami
            c.ellipse(24.0, 20.0, 13.0, 16.0, rgba(0.54, 0.45, 0.35, 0.9));
            c.ellipse(17.0, 17.0, 3.0, 4.2, rgb(0.03, 0.01, 0.01));
            c.ellipse(30.0, 18.0, 2.5, 3.4, rgb(0.03, 0.01, 0.01));
            c.circle(17.0, 17.0, 0.9, rgb(0.6, 0.05, 0.04));
            c.circle(30.0, 18.0, 0.8, rgb(0.6, 0.05, 0.04));
            c.ellipse(24.0, 30.0, 7.0, 3.0, rgb(0.03, 0.005, 0.005));
            draw_teeth(&mut c, 19, 30, 5, 3, true);
            for i in 0..4 {
                c.line(12 + i * 2, 8, 18 + i * 4, 38, rgba(0.28, 0.02, 0.02, 0.45));
            }
        }
        _ => {
            // drobne oczy i znamiona
            for _ in 0..4 {
                let x = rng.range(6.0, 42.0);
                let y = rng.range(6.0, 42.0);
                c.circle(x, y, rng.range(0.7, 1.6), rgb(0.02, 0.01, 0.01));
                c.circle(x - 0.5, y - 0.5, 0.4, rgba(0.8, 0.1, 0.08, 0.8));
            }
        }
    }

    // ciemne obramowanie kafla
    for i in 0..48 {
        c.put(i, 0, rgba(0.01, 0.005, 0.005, 0.6));
        c.put(i, 47, rgba(0.01, 0.005, 0.005, 0.6));
        c.put(0, i, rgba(0.01, 0.005, 0.005, 0.6));
        c.put(47, i, rgba(0.01, 0.005, 0.005, 0.6));
    }

    c.grain(seed + 77, 0.055);
    c.to_texture(FilterMode::Nearest)
}

fn player_texture() -> Texture2D {
    let mut c = Canvas::new(40, 52);
    let cloak = rgb(0.095, 0.078, 0.105);
    let cloak_dark = rgb(0.030, 0.024, 0.036);
    let cloak_edge = rgba(0.20, 0.17, 0.21, 0.85);
    let bone = rgb(0.60, 0.56, 0.45);
    let bone_hi = rgb(0.72, 0.67, 0.54);
    let red = rgb(0.55, 0.025, 0.035);

    // płaszcz / workowata sylwetka
    c.ellipse(20.0, 38.0, 15.0, 15.0, cloak);
    c.ellipse(20.0, 29.0, 12.5, 10.0, cloak);
    for i in 0..11 {
        let x = 6 + i * 3;
        c.circle(x as f32, 51.0 + (i % 2) as f32, 1.7, cloak);
    }
    // slabu rim-light na lewej krawedzi
    for i in 0..13 {
        let t = i as f32 / 12.0;
        let x = 20.0 - 14.5 * (1.0 - t * t).sqrt();
        let y = 27.0 + t * 23.0;
        c.put(x as i32, y as i32, cloak_edge);
    }

    // kaptur + czerwona podszewka + maska
    c.circle(20.0, 16.0, 11.8, cloak_dark);
    c.ellipse(20.0, 16.0, 9.0, 9.8, rgb(0.18, 0.05, 0.055));
    c.ellipse(20.0, 16.0, 7.6, 8.6, bone);
    c.ellipse(22.5, 17.0, 5.2, 7.2, rgba(0.13, 0.10, 0.09, 0.35));
    c.ellipse(17.5, 14.0, 2.8, 3.4, rgba(0.78, 0.73, 0.60, 0.28));

    // puste oczodoly z czerwonymi punkcikami
    c.ellipse(16.2, 14.5, 2.7, 2.1, BLACK);
    c.ellipse(23.8, 14.5, 2.7, 2.1, BLACK);
    c.circle(16.2, 14.5, 1.0, rgb(0.90, 0.06, 0.04));
    c.circle(23.8, 14.5, 1.0, rgb(0.90, 0.06, 0.04));

    // zszyte usta
    for x in (16..25).step_by(2) {
        c.put(x, 20, rgba(0.08, 0.015, 0.015, 0.95));
    }
    c.line(16, 21, 24, 21, rgba(0.35, 0.02, 0.02, 0.7));

    // czerwony szal / nitka
    c.line(11, 24, 29, 24, red);
    c.circle(11.0, 24.0, 1.5, red);
    c.circle(29.0, 24.0, 1.5, red);

    // lewa reka
    c.line(9, 28, 4, 41, cloak_dark);
    c.circle(4.0, 41.0, 2.0, bone);
    // prawa reka z latarka, wysunieta do przodu i w bok
    c.line(28, 27, 32, 10, cloak_dark);
    c.circle(32.0, 10.0, 2.4, bone_hi);
    c.rect(29, 4, 6, 7, rgb(0.14, 0.14, 0.15));
    c.rect(28, 2, 8, 3, rgb(1.0, 0.84, 0.42));
    c.circle(32.0, 3.0, 5.0, rgba(1.0, 0.72, 0.25, 0.12));

    // szwy na płaszczu
    for i in 0..6 {
        c.put(13 + i * 3, 34, rgba(0.24, 0.20, 0.23, 0.6));
    }

    c.noise_erode(33, 0.14);
    c.outline(rgba(0.005, 0.003, 0.006, 1.0));
    c.grain(55, 0.055);
    c.to_texture(FilterMode::Nearest)
}

fn monster_texture(variant: usize) -> Texture2D {
    let mut c = Canvas::new(48, 56);
    let black = rgb(0.030, 0.020, 0.035);
    let bone = rgb(0.63, 0.57, 0.45);
    let blood = rgb(0.28, 0.015, 0.015);

    match variant {
        0 => {
            // "Grin"
            for i in 0..7 {
                let x = 10 + i * 4;
                c.line(x, 40, x - 4 + (i % 3) * 3, 55, rgba(0.10, 0.03, 0.05, 0.9));
            }
            c.ellipse(24.0, 38.0, 17.0, 14.0, black);
            c.line(12, 34, 2, 22, black);
            c.line(36, 34, 46, 24, black);
            c.line(2, 22, 0, 16, blood);
            c.line(46, 24, 48, 17, blood);
            c.ellipse(23.0, 18.0, 14.0, 17.0, bone);
            c.ellipse(26.0, 19.0, 11.0, 14.0, rgba(0.42, 0.35, 0.28, 0.35));
            // oczy
            c.ellipse(17.0, 13.0, 3.4, 4.5, rgb(0.015, 0.005, 0.005));
            c.ellipse(29.0, 15.0, 3.0, 4.0, rgb(0.015, 0.005, 0.005));
            c.circle(17.0, 13.0, 1.2, rgb(0.85, 0.05, 0.04));
            c.circle(29.0, 15.0, 1.0, rgb(0.85, 0.05, 0.04));
            c.ellipse(24.0, 8.0, 2.2, 2.8, rgb(0.02, 0.005, 0.005));
            c.circle(24.0, 8.0, 0.8, rgb(0.9, 0.1, 0.05));
            // paszcza
            c.ellipse(24.0, 27.0, 11.0, 6.0, rgb(0.01, 0.003, 0.003));
            draw_teeth(&mut c, 14, 25, 8, 3, true);
            draw_teeth(&mut c, 15, 31, 7, 3, false);
            c.line(20, 33, 18, 44, rgba(0.35, 0.02, 0.02, 0.8));
            c.line(29, 32, 31, 41, rgba(0.35, 0.02, 0.02, 0.8));
            c.circle(18.0, 44.0, 1.4, rgba(0.35, 0.02, 0.02, 0.9));
            // blizny
            for i in 0..4 {
                c.line(12 + i * 7, 5, 15 + i * 6, 20, rgba(0.28, 0.03, 0.03, 0.5));
            }
        }
        1 => {
            // "Matka"
            c.ellipse(24.0, 38.0, 20.0, 16.0, rgb(0.24, 0.07, 0.08));
            c.ellipse(24.0, 37.0, 17.0, 13.0, rgb(0.33, 0.10, 0.10));
            // macki
            for i in 0..8 {
                let a = -0.4 + i as f32 * 0.45;
                let x = 24.0 + a.cos() * 17.0;
                let y = 36.0 + a.sin() * 12.0;
                c.line(
                    24,
                    36,
                    x as i32,
                    (y + 16.0) as i32,
                    rgba(0.22, 0.04, 0.05, 0.9),
                );
                c.circle(x, y + 17.0, 1.7, rgba(0.5, 0.35, 0.28, 0.8));
            }
            // maly leb
            c.ellipse(24.0, 12.0, 8.0, 9.5, bone);
            c.ellipse(21.0, 11.0, 2.4, 3.0, rgb(0.02, 0.005, 0.005));
            c.ellipse(27.0, 11.0, 2.2, 2.8, rgb(0.02, 0.005, 0.005));
            c.circle(21.0, 11.0, 0.9, rgb(0.8, 0.05, 0.04));
            c.circle(27.0, 11.0, 0.9, rgb(0.8, 0.05, 0.04));
            c.ellipse(24.0, 17.0, 3.6, 2.4, rgb(0.03, 0.005, 0.005));
            // ropnie i oczy na ciele
            for i in 0..10 {
                let x = 10.0 + (i * 37 % 29) as f32;
                let y = 30.0 + (i * 23 % 19) as f32;
                c.circle(x, y, 1.8 + (i % 3) as f32, rgb(0.42, 0.14, 0.12));
                if i % 2 == 0 {
                    c.circle(x, y, 0.8, rgb(0.9, 0.1, 0.06));
                }
            }
            c.line(13, 45, 11, 55, rgba(0.35, 0.03, 0.03, 0.9));
            c.line(34, 45, 37, 55, rgba(0.35, 0.03, 0.03, 0.9));
        }
        _ => {
            // "Stilt"
            c.rect(22, 20, 5, 36, black);
            c.line(24, 24, 6, 10, black);
            c.line(24, 24, 43, 8, black);
            c.line(6, 10, 2, 3, black);
            c.line(43, 8, 47, 2, black);
            c.circle(2.0, 3.0, 2.2, rgb(0.45, 0.38, 0.28));
            c.circle(47.0, 2.0, 2.0, rgb(0.45, 0.38, 0.28));
            c.line(23, 32, 12, 48, black);
            c.line(26, 32, 37, 48, black);
            c.line(12, 48, 9, 55, black);
            c.line(37, 48, 41, 55, black);
            // zebra
            for i in 0..5 {
                c.line(20, 27 + i * 4, 29, 26 + i * 4, rgba(0.55, 0.48, 0.37, 0.55));
            }
            // glowa
            c.ellipse(24.0, 12.0, 8.0, 12.0, bone);
            c.ellipse(21.0, 10.0, 2.2, 3.6, rgb(0.01, 0.003, 0.003));
            c.ellipse(28.0, 11.0, 1.8, 3.0, rgb(0.01, 0.003, 0.003));
            c.circle(21.0, 10.0, 0.8, rgb(0.8, 0.05, 0.04));
            c.circle(28.0, 11.0, 0.7, rgb(0.8, 0.05, 0.04));
            // czerwony usmiech
            c.ellipse(24.0, 18.0, 5.5, 2.4, rgb(0.03, 0.003, 0.003));
            c.line(19, 18, 29, 18, rgb(0.55, 0.02, 0.02));
            for i in 0..4 {
                c.line(20 + i * 2, 17, 20 + i * 2, 20, rgba(0.7, 0.05, 0.04, 0.8));
            }
        }
    }

    c.noise_erode(900 + variant as u32, 0.16);
    c.outline(rgba(0.005, 0.002, 0.005, 1.0));
    c.grain(1300 + variant as u32, 0.08);
    c.to_texture(FilterMode::Nearest)
}

fn relic_texture() -> Texture2D {
    let mut c = Canvas::new(20, 32);
    let wax = rgb(0.72, 0.64, 0.48);
    let wax_dark = rgb(0.48, 0.40, 0.29);
    // swieca
    c.rect(6, 15, 8, 15, wax);
    c.rect(6, 15, 2, 15, wax_dark);
    for i in 0..5 {
        let x = 5 + i * 2;
        c.circle(x as f32, 30.0 - (i % 2) as f32, 1.4, wax);
    }
    // dziwna twarz w wosku
    c.circle(8.5, 21.0, 1.0, rgb(0.03, 0.01, 0.01));
    c.circle(12.0, 21.0, 1.0, rgb(0.03, 0.01, 0.01));
    c.line(8, 25, 12, 25, rgb(0.12, 0.03, 0.03));
    // knot i ogien
    c.rect(9, 11, 1, 5, rgb(0.08, 0.05, 0.03));
    c.ellipse(10.0, 8.0, 3.6, 5.0, rgba(0.95, 0.45, 0.08, 0.95));
    c.ellipse(10.0, 8.5, 2.2, 3.2, rgba(1.0, 0.82, 0.25, 1.0));
    c.ellipse(10.0, 9.0, 1.0, 1.7, rgba(1.0, 1.0, 0.9, 1.0));
    c.circle(10.0, 7.5, 5.0, rgba(1.0, 0.5, 0.1, 0.08));
    c.outline(rgba(0.01, 0.005, 0.005, 1.0));
    c.to_texture(FilterMode::Nearest)
}

fn door_texture() -> Texture2D {
    let mut c = Canvas::new(48, 64);
    let metal = rgb(0.16, 0.15, 0.15);
    let rust = rgb(0.28, 0.09, 0.05);
    c.rect(3, 2, 42, 60, rgb(0.04, 0.035, 0.04));
    c.rect(5, 4, 38, 56, metal);
    for i in 0..4 {
        c.rect(7 + i * 9, 4, 1, 56, rgb(0.06, 0.05, 0.05));
    }
    // rdza
    for i in 0..15 {
        c.circle(
            6.0 + (i * 29 % 36) as f32,
            5.0 + (i * 17 % 52) as f32,
            1.5,
            rust,
        );
    }
    // okno
    c.rect(14, 10, 20, 12, rgb(0.015, 0.012, 0.015));
    for i in 0..4 {
        c.rect(14 + i * 5, 10, 1, 12, metal);
    }
    c.rect(14, 15, 20, 1, metal);
    c.circle(21.0, 16.0, 1.0, rgb(0.8, 0.05, 0.04));
    c.circle(27.0, 16.0, 1.0, rgb(0.8, 0.05, 0.04));
    // twarz
    c.ellipse(24.0, 38.0, 10.0, 12.0, rgba(0.45, 0.38, 0.29, 0.65));
    c.ellipse(20.0, 35.0, 2.0, 2.6, rgb(0.02, 0.005, 0.005));
    c.ellipse(28.0, 35.0, 2.0, 2.6, rgb(0.02, 0.005, 0.005));
    c.ellipse(24.0, 43.0, 5.0, 2.8, rgb(0.02, 0.005, 0.005));
    draw_teeth(&mut c, 20, 43, 4, 3, true);
    // runy
    for i in 0..5 {
        c.line(8 + i * 7, 52, 11 + i * 7, 57, rgba(0.55, 0.03, 0.03, 0.7));
        c.line(11 + i * 7, 52, 8 + i * 7, 57, rgba(0.55, 0.03, 0.03, 0.7));
    }
    // okucie
    for (x, y) in [(8, 8), (40, 8), (8, 56), (40, 56)] {
        c.circle(x as f32, y as f32, 1.6, rgb(0.05, 0.04, 0.04));
    }
    c.outline(rgba(0.005, 0.003, 0.003, 1.0));
    c.grain(4242, 0.06);
    c.to_texture(FilterMode::Nearest)
}

fn face_texture() -> Texture2D {
    let mut c = Canvas::new(220, 220);
    let bone = rgb(0.52, 0.47, 0.38);
    let dark = rgb(0.015, 0.005, 0.008);
    // czaszka
    c.ellipse(110.0, 100.0, 78.0, 92.0, bone);
    c.ellipse(110.0, 120.0, 66.0, 66.0, rgb(0.42, 0.37, 0.30));
    // pęknięcia
    for i in 0..12 {
        let mut x = 40 + (i * 47 % 140);
        let mut y = 20 + (i * 31 % 70);
        for _ in 0..8 {
            c.put(x, y, rgba(0.06, 0.02, 0.02, 0.6));
            x += 2;
            y += if i % 2 == 0 { 2 } else { -1 };
        }
    }
    // wiele oczu
    let eyes = [
        (72.0, 76.0, 14.0),
        (146.0, 74.0, 15.0),
        (110.0, 54.0, 10.0),
        (54.0, 118.0, 9.0),
        (166.0, 116.0, 8.0),
        (88.0, 142.0, 7.0),
        (134.0, 144.0, 7.0),
    ];
    for (i, (x, y, r)) in eyes.iter().enumerate() {
        c.ellipse(*x, *y, *r, *r * 1.25, dark);
        c.circle(*x, *y, r * 0.55, rgb(0.33, 0.04, 0.03));
        c.circle(*x, *y, r * 0.25, rgb(0.005, 0.002, 0.002));
        c.circle(
            *x - r * 0.25,
            *y - r * 0.3,
            r * 0.10,
            rgba(0.9, 0.8, 0.65, 0.8),
        );
        if i % 2 == 0 {
            c.line(
                (*x - r * 1.6) as i32,
                *y as i32,
                (*x + r * 1.6) as i32,
                (*y + 2.0) as i32,
                rgba(0.32, 0.02, 0.02, 0.55),
            );
        }
    }
    // paszcza
    c.ellipse(110.0, 174.0, 62.0, 26.0, dark);
    draw_teeth(&mut c, 58, 168, 20, 5, true);
    draw_teeth(&mut c, 59, 181, 19, 5, false);
    c.ellipse(110.0, 174.0, 44.0, 12.0, rgb(0.18, 0.01, 0.01));
    // macki
    for i in 0..10 {
        let x = 20 + i * 20;
        let len = 24 + (i * 17 % 40);
        c.line(
            x,
            190,
            x + (i % 3) * 3 - 3,
            190 + len,
            rgba(0.12, 0.02, 0.03, 0.85),
        );
        c.circle(
            (x + (i % 3) * 3 - 3) as f32,
            (190 + len) as f32,
            3.0,
            rgba(0.30, 0.03, 0.03, 0.8),
        );
    }
    c.noise_erode(991, 0.22);
    c.outline(rgba(0.005, 0.002, 0.003, 1.0));
    c.grain(177, 0.09);
    c.to_texture(FilterMode::Linear)
}

// ---------------------------------------------------------------------------
// AUDIO
// ---------------------------------------------------------------------------

fn wav_mono(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn fade_edges(samples: &mut [f32], sample_rate: u32, seconds: f32) {
    let n = (seconds * sample_rate as f32) as usize;
    let n = n.min(samples.len() / 2);
    for i in 0..n {
        let a = i as f32 / n as f32;
        samples[i] *= a;
        let j = samples.len() - 1 - i;
        samples[j] *= a;
    }
}

fn ambient_samples() -> Vec<f32> {
    let sr = 44_100u32;
    let duration = 14.0f32;
    let n = (sr as f32 * duration) as usize;
    let mut rng = Rng::new(0xA11CE);
    let mut low = 0.0f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr as f32;
        let drone = (std::f32::consts::TAU * 41.0 * t).sin() * 0.20
            + (std::f32::consts::TAU * 55.0 * t + 0.4).sin() * 0.16
            + (std::f32::consts::TAU * 82.5 * t).sin() * 0.07;
        let noise = rng.range(-1.0, 1.0);
        low += (noise - low) * 0.012;
        let wind = low * 0.30 * (0.55 + 0.45 * (std::f32::consts::TAU * t / 7.0).sin());
        let mut metal = 0.0;
        for t0 in [3.1f32, 8.7, 11.9] {
            let dt = t - t0;
            if dt > 0.0 {
                metal += (-dt * 2.2).exp() * (std::f32::consts::TAU * 197.0 * dt).sin() * 0.05;
            }
        }
        let mut s = drone + wind + metal;
        s = (s * 1.2).tanh() * 0.8;
        out.push(s);
    }
    fade_edges(&mut out, sr, 0.05);
    out
}

fn heartbeat_samples() -> Vec<f32> {
    let sr = 44_100u32;
    let n = (sr as f32 * 1.35) as usize;
    let mut rng = Rng::new(0xBEA7);
    let mut out = vec![0.0f32; n];
    for (start, amp) in [(0.07f32, 1.0f32), (0.34, 0.7)] {
        let s0 = (start * sr as f32) as usize;
        for i in 0..(sr as usize / 4) {
            if s0 + i >= n {
                break;
            }
            let t = i as f32 / sr as f32;
            let env = (-t * 18.0).exp();
            let freq = 54.0 - t * 40.0;
            let thump = (std::f32::consts::TAU * freq * t).sin() * env * amp;
            let click = rng.range(-1.0, 1.0) * (-t * 90.0).exp() * 0.20 * amp;
            out[s0 + i] += thump * 0.8 + click;
        }
    }
    fade_edges(&mut out, sr, 0.01);
    out
}

fn pickup_samples() -> Vec<f32> {
    let sr = 44_100u32;
    let n = (sr as f32 * 1.8) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr as f32;
        let env = (-t * 2.0).exp();
        let mut s = 0.0;
        for (f, a) in [
            (660.0f32, 0.25f32),
            (990.0, 0.18),
            (1320.0, 0.12),
            (1760.0, 0.06),
        ] {
            s += (std::f32::consts::TAU * f * t).sin() * a * env;
        }
        s += (std::f32::consts::TAU * (330.0 + t * 80.0) * t).sin() * 0.12 * env;
        out.push(s * 0.8);
    }
    fade_edges(&mut out, sr, 0.01);
    out
}

fn hit_samples() -> Vec<f32> {
    let sr = 44_100u32;
    let n = (sr as f32 * 0.85) as usize;
    let mut rng = Rng::new(0xDEAD);
    let mut out = Vec::with_capacity(n);
    let mut low = 0.0f32;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        let env = (-t * 8.0).exp();
        let noise = rng.range(-1.0, 1.0);
        low += (noise - low) * 0.2;
        let boom = (std::f32::consts::TAU * (70.0 - t * 35.0) * t).sin() * (-t * 6.0).exp();
        let crack = low * (-t * 30.0).exp() * 0.7;
        out.push(((boom * 0.8 + crack) * env * 1.2).tanh());
    }
    fade_edges(&mut out, sr, 0.01);
    out
}

fn whisper_samples() -> Vec<f32> {
    let sr = 44_100u32;
    let n = (sr as f32 * 2.4) as usize;
    let mut rng = Rng::new(0x5151);
    let mut out = Vec::with_capacity(n);
    let mut low = 0.0f32;
    let mut band = 0.0f32;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        let noise = rng.range(-1.0, 1.0);
        low += (noise - low) * 0.08;
        band += (noise - band) * 0.45;
        let syll = (t * 9.0).sin().max(0.0).powf(2.0) * (t * 2.3).sin().max(0.0);
        let env = syll * (1.0 - (t / 2.4)).clamp(0.0, 1.0);
        out.push((band - low) * env * 0.45);
    }
    fade_edges(&mut out, sr, 0.03);
    out
}

fn step_samples() -> Vec<f32> {
    let sr = 44_100u32;
    let n = (sr as f32 * 0.16) as usize;
    let mut rng = Rng::new(0xF007);
    let mut out = Vec::with_capacity(n);
    let mut low = 0.0f32;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        let noise = rng.range(-1.0, 1.0);
        low += (noise - low) * 0.12;
        out.push(low * (-t * 30.0).exp() * 0.8);
    }
    fade_edges(&mut out, sr, 0.005);
    out
}

fn win_samples() -> Vec<f32> {
    let sr = 44_100u32;
    let n = (sr as f32 * 4.0) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / sr as f32;
        let env = (t * 0.6).min(1.0) * (-(t - 1.0).max(0.0) * 1.4).exp();
        let mut s = 0.0;
        for (f, a) in [
            (196.0f32, 0.18f32),
            (246.94, 0.15),
            (293.66, 0.13),
            (392.0, 0.06),
        ] {
            s += (std::f32::consts::TAU * f * t + (t * 1.7).sin()).sin() * a;
        }
        let shimmer = (std::f32::consts::TAU * 1174.0 * t).sin() * 0.03;
        out.push((s + shimmer) * env * 0.9);
    }
    fade_edges(&mut out, sr, 0.05);
    out
}

fn death_samples() -> Vec<f32> {
    let sr = 44_100u32;
    let n = (sr as f32 * 3.2) as usize;
    let mut rng = Rng::new(0xD347);
    let mut out = Vec::with_capacity(n);
    let mut low = 0.0f32;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        let f = 110.0 * (-t * 0.8).exp() + 24.0;
        let tone = (std::f32::consts::TAU * f * t).sin();
        let noise = rng.range(-1.0, 1.0);
        low += (noise - low) * 0.02;
        let env = (t * 3.0).min(1.0) * (-t * 0.5).exp();
        out.push(((tone * 0.35 + low * 0.4) * env).tanh());
    }
    fade_edges(&mut out, sr, 0.03);
    out
}

pub struct Assets {
    pub floors: Vec<Texture2D>,
    pub walls: Vec<Texture2D>,
    pub player: Texture2D,
    pub monsters: Vec<Texture2D>,
    pub relic: Texture2D,
    pub door: Texture2D,
    pub face: Texture2D,
    pub ambient: Option<Sound>,
    pub heartbeat: Option<Sound>,
    pub pickup: Option<Sound>,
    pub hit: Option<Sound>,
    pub whisper: Option<Sound>,
    pub step: Option<Sound>,
    pub win: Option<Sound>,
    pub death: Option<Sound>,
}

impl Assets {
    pub async fn load() -> Self {
        let floors = (0..4).map(floor_texture).collect();
        let walls = (0..4).map(wall_texture).collect();
        let player = player_texture();
        let monsters = (0..3).map(monster_texture).collect();
        let relic = relic_texture();
        let door = door_texture();
        let face = face_texture();

        let ambient = load_sound_from_bytes(&wav_mono(&ambient_samples(), 44_100))
            .await
            .ok();
        let heartbeat = load_sound_from_bytes(&wav_mono(&heartbeat_samples(), 44_100))
            .await
            .ok();
        let pickup = load_sound_from_bytes(&wav_mono(&pickup_samples(), 44_100))
            .await
            .ok();
        let hit = load_sound_from_bytes(&wav_mono(&hit_samples(), 44_100))
            .await
            .ok();
        let whisper = load_sound_from_bytes(&wav_mono(&whisper_samples(), 44_100))
            .await
            .ok();
        let step = load_sound_from_bytes(&wav_mono(&step_samples(), 44_100))
            .await
            .ok();
        let win = load_sound_from_bytes(&wav_mono(&win_samples(), 44_100))
            .await
            .ok();
        let death = load_sound_from_bytes(&wav_mono(&death_samples(), 44_100))
            .await
            .ok();

        Self {
            floors,
            walls,
            player,
            monsters,
            relic,
            door,
            face,
            ambient,
            heartbeat,
            pickup,
            hit,
            whisper,
            step,
            win,
            death,
        }
    }
}
