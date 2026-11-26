//! Proceduralny labirynt + kolizje + wyszukiwanie sciezek.

use std::collections::VecDeque;

use macroquad::prelude::*;

use crate::rng::{hash2, Rng};

pub const TILE: f32 = 48.0;
pub const MAP_W: usize = 45;
pub const MAP_H: usize = 27;

#[derive(Clone)]
pub struct Map {
    pub solid: Vec<bool>,
    pub variant: Vec<u8>,
    pub floor_tiles: Vec<(i32, i32)>,
    pub w: usize,
    pub h: usize,
}

impl Map {
    pub fn generate(rng: &mut Rng) -> Self {
        let w = MAP_W;
        let h = MAP_H;
        let mut solid = vec![true; w * h];
        let mut visited = vec![false; w * h];

        // --- szkielet labiryntu (DFS na komorkach nieparzystych) ---
        let start = (1usize, 1usize);
        let mut stack = vec![start];
        visited[w + 1] = true;
        solid[w + 1] = false;

        while let Some(&(cx, cy)) = stack.last() {
            let mut options: Vec<(usize, usize)> = Vec::with_capacity(4);
            for (dx, dy) in [(2i32, 0i32), (-2, 0), (0, 2), (0, -2)] {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;
                if nx > 0
                    && ny > 0
                    && nx < w as i32 - 1
                    && ny < h as i32 - 1
                    && !visited[ny as usize * w + nx as usize]
                {
                    options.push((nx as usize, ny as usize));
                }
            }
            if options.is_empty() {
                stack.pop();
                continue;
            }
            let (nx, ny) = options[rng.usize(options.len())];
            let mx = (cx + nx) / 2;
            let my = (cy + ny) / 2;
            solid[my * w + mx] = false;
            solid[ny * w + nx] = false;
            visited[ny * w + nx] = true;
            stack.push((nx, ny));
        }

        // --- kilka wiekszych sal, zawsze polaczonych z labiryntem ---
        for _ in 0..7 {
            let rw = 5 + 2 * rng.usize(2);
            let rh = 4 + 2 * rng.usize(2);
            let max_x = (w.saturating_sub(rw + 2)) / 2;
            let max_y = (h.saturating_sub(rh + 2)) / 2;
            if max_x == 0 || max_y == 0 {
                continue;
            }
            let rx = 1 + 2 * rng.usize(max_x);
            let ry = 1 + 2 * rng.usize(max_y);
            for y in ry..(ry + rh).min(h - 1) {
                for x in rx..(rx + rw).min(w - 1) {
                    solid[y * w + x] = false;
                }
            }
        }

        // --- dodatkowe przejscia: mniej przewidywalny, koszmarny uklad ---
        for _ in 0..34 {
            let x = 2 + rng.usize(w - 4);
            let y = 2 + rng.usize(h - 4);
            if rng.chance(0.7) {
                solid[y * w + x] = false;
            }
        }

        // --- szczelny border ---
        for x in 0..w {
            solid[x] = true;
            solid[(h - 1) * w + x] = true;
        }
        for y in 0..h {
            solid[y * w] = true;
            solid[y * w + w - 1] = true;
        }

        // Upewniamy sie, ze start istnieje.
        solid[w + 1] = false;

        let mut map = Self {
            solid,
            variant: vec![0; w * h],
            floor_tiles: Vec::new(),
            w,
            h,
        };

        let variant_seed = rng.next_u32();
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let i = y as usize * w + x as usize;
                let h = hash2(x, y, variant_seed);
                // Wiekszosc kafli jest "zwykla"; twarze, oczy i paszcze maja
                // byc rzadkoscia, ktora wybija z rytmu.
                map.variant[i] = if !map.solid[i] {
                    if h < 0.52 {
                        0
                    } else if h < 0.70 {
                        1
                    } else if h < 0.86 {
                        2
                    } else {
                        3
                    }
                } else if h < 0.64 {
                    0
                } else if h < 0.78 {
                    1
                } else if h < 0.91 {
                    2
                } else {
                    3
                };
                if !map.solid[i] {
                    map.floor_tiles.push((x, y));
                }
            }
        }

        // Zostawiamy tylko osiagalne pola (reszta i tak jest dekoracja).
        let dist = map.distances_from(Map::tile_center(1, 1));
        map.floor_tiles
            .retain(|(x, y)| dist[*y as usize * w + *x as usize] != u32::MAX);

        map
    }

    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> usize {
        y as usize * self.w + x as usize
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h
    }

    #[inline]
    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        !self.in_bounds(x, y) || self.solid[self.idx(x, y)]
    }

    #[inline]
    pub fn tile_of(p: Vec2) -> (i32, i32) {
        ((p.x / TILE).floor() as i32, (p.y / TILE).floor() as i32)
    }

    #[inline]
    pub fn tile_center(x: i32, y: i32) -> Vec2 {
        vec2((x as f32 + 0.5) * TILE, (y as f32 + 0.5) * TILE)
    }

    pub fn world_size(&self) -> Vec2 {
        vec2(self.w as f32 * TILE, self.h as f32 * TILE)
    }

    pub fn solid_at_world(&self, p: Vec2) -> bool {
        let (x, y) = Self::tile_of(p);
        self.is_solid(x, y)
    }

    /// Czy okrag moze stac w danym miejscu (kolizja z kaflami).
    pub fn can_stand(&self, p: Vec2, radius: f32) -> bool {
        if self.solid_at_world(p) {
            return false;
        }
        let r = radius;
        for (dx, dy) in [
            (-r, -r),
            (r, -r),
            (-r, r),
            (r, r),
            (0.0, -r),
            (0.0, r),
            (-r, 0.0),
            (r, 0.0),
        ] {
            if self.solid_at_world(p + vec2(dx, dy)) {
                return false;
            }
        }
        true
    }

    /// BFS z jednego kafla; u32::MAX = sciana / nieosiagalne.
    pub fn distances_from(&self, start: Vec2) -> Vec<u32> {
        let (sx, sy) = Self::tile_of(start);
        let mut dist = vec![u32::MAX; self.w * self.h];
        if self.is_solid(sx, sy) {
            return dist;
        }
        let mut queue = VecDeque::new();
        let si = self.idx(sx, sy);
        dist[si] = 0;
        queue.push_back(si);

        while let Some(i) = queue.pop_front() {
            let d = dist[i];
            let x = (i % self.w) as i32;
            let y = (i / self.w) as i32;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let nx = x + dx;
                let ny = y + dy;
                if self.is_solid(nx, ny) {
                    continue;
                }
                let ni = self.idx(nx, ny);
                if dist[ni] == u32::MAX {
                    dist[ni] = d + 1;
                    queue.push_back(ni);
                }
            }
        }
        dist
    }

    pub fn farthest_floor_tile(&self, dist: &[u32]) -> (i32, i32) {
        let mut best = (1, 1);
        let mut best_d = 0;
        for &(x, y) in &self.floor_tiles {
            let d = dist[self.idx(x, y)];
            if d != u32::MAX && d > best_d {
                best_d = d;
                best = (x, y);
            }
        }
        best
    }

    /// BFS pathfinding po kaflach. Zwraca srodek kolejnych kafli.
    pub fn bfs_path(&self, start: Vec2, goal: Vec2) -> Vec<Vec2> {
        let (sx, sy) = Self::tile_of(start);
        let (mut gx, mut gy) = Self::tile_of(goal);
        if !self.in_bounds(sx, sy) {
            return Vec::new();
        }
        if self.is_solid(gx, gy) {
            // Cel mogl zostac wypchniety w sciane - szukamy najblizszego pola.
            let mut found = None;
            'outer: for r in 1..4 {
                for yy in (gy - r)..=(gy + r) {
                    for xx in (gx - r)..=(gx + r) {
                        if !self.is_solid(xx, yy) {
                            found = Some((xx, yy));
                            break 'outer;
                        }
                    }
                }
            }
            if let Some((x, y)) = found {
                gx = x;
                gy = y;
            } else {
                return Vec::new();
            }
        }

        let si = self.idx(sx, sy);
        let gi = self.idx(gx, gy);
        if si == gi {
            return vec![Self::tile_center(gx, gy)];
        }

        let mut prev = vec![-1i32; self.w * self.h];
        prev[si] = si as i32;
        let mut queue = VecDeque::new();
        queue.push_back(si);

        while let Some(i) = queue.pop_front() {
            if i == gi {
                break;
            }
            let x = (i % self.w) as i32;
            let y = (i / self.w) as i32;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let nx = x + dx;
                let ny = y + dy;
                if self.is_solid(nx, ny) {
                    continue;
                }
                let ni = self.idx(nx, ny);
                if prev[ni] == -1 {
                    prev[ni] = i as i32;
                    queue.push_back(ni);
                }
            }
        }

        if prev[gi] == -1 {
            return Vec::new();
        }

        let mut path = Vec::new();
        let mut cur = gi;
        while cur != si {
            let x = (cur % self.w) as i32;
            let y = (cur / self.w) as i32;
            path.push(Self::tile_center(x, y));
            let p = prev[cur];
            if p < 0 {
                break;
            }
            cur = p as usize;
        }
        path.reverse();
        path
    }

    pub fn line_of_sight(&self, a: Vec2, b: Vec2) -> bool {
        let d = b - a;
        let len = d.length();
        if len < 1.0 {
            return true;
        }
        let steps = (len / 8.0).ceil() as i32;
        for i in 1..steps {
            let t = i as f32 / steps as f32;
            if self.solid_at_world(a + d * t) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    #[test]
    fn generated_map_is_connected_for_many_seeds() {
        for seed in [1u64, 2, 3, 42, 12_345] {
            let mut rng = Rng::new(seed);
            let map = Map::generate(&mut rng);
            assert!(!map.floor_tiles.is_empty(), "seed {seed}: brak podlog");
            let start = Map::tile_center(1, 1);
            let dist = map.distances_from(start);
            for &(x, y) in &map.floor_tiles {
                assert_ne!(
                    dist[map.idx(x, y)],
                    u32::MAX,
                    "seed {seed}: kafel ({x},{y}) nieosiagalny"
                );
            }
        }
    }

    #[test]
    fn bfs_reaches_farthest_tile() {
        let mut rng = Rng::new(999);
        let map = Map::generate(&mut rng);
        let start = Map::tile_center(1, 1);
        let dist = map.distances_from(start);
        let (fx, fy) = map.farthest_floor_tile(&dist);
        let goal = Map::tile_center(fx, fy);
        let path = map.bfs_path(start, goal);
        assert!(!path.is_empty());
        assert!(path.last().unwrap().distance(goal) < 1.0);
    }

    #[test]
    fn walls_reject_the_player_circle() {
        let mut rng = Rng::new(5);
        let map = Map::generate(&mut rng);
        let mut checked = 0;
        for y in 0..map.h as i32 {
            for x in 0..map.w as i32 {
                if map.is_solid(x, y) {
                    assert!(!map.can_stand(Map::tile_center(x, y), 13.0));
                    checked += 1;
                }
            }
        }
        assert!(checked > 0);
    }
}
