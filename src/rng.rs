//! Maly, deterministyczny generator liczb pseudolosowych i szum.
//! Bez zewnetrznych zaleznosci - mniej kodu do kompilacji.

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1) ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    pub fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    pub fn usize(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            self.next_u32() as usize % n
        }
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.usize(i + 1);
            items.swap(i, j);
        }
    }
}

#[inline]
pub fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

#[inline]
pub fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let h = hash_u32(
        (x as u32)
            .wrapping_mul(0x8da6_b343)
            .wrapping_add((y as u32).wrapping_mul(0xd816_3841))
            .wrapping_add(seed.wrapping_mul(0xcb1a_b31f)),
    );
    (h & 0x00ff_ffff) as f32 / 16_777_215.0
}

#[inline]
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Prostowany szum wartosciowy (bilinear + smoothstep).
pub fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let fx = smooth(x - xi);
    let fy = smooth(y - yi);
    let x0 = xi as i32;
    let y0 = yi as i32;

    let a = hash2(x0, y0, seed);
    let b = hash2(x0 + 1, y0, seed);
    let c = hash2(x0, y0 + 1, seed);
    let d = hash2(x0 + 1, y0 + 1, seed);

    let top = a + (b - a) * fx;
    let bottom = c + (d - c) * fx;
    top + (bottom - top) * fy
}

pub fn fbm(x: f32, y: f32, seed: u32, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut total = 0.0;
    for i in 0..octaves {
        value += value_noise(x * freq, y * freq, seed.wrapping_add(i * 7919)) * amp;
        total += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    if total > 0.0 {
        value / total
    } else {
        0.0
    }
}
