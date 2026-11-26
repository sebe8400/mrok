//! Stan gry, aktorzy, AI, interakcje i rysowanie swiata.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::fx::{glitch_text, glitch_text_centered, PostParams, VIEW_H, VIEW_W};
use crate::map::{Map, TILE};
use crate::rng::{hash2, Rng};

const PLAYER_RADIUS: f32 = 13.0;
const MONSTER_RADIUS: f32 = 15.0;
const TOTAL_RELICS: usize = 5;

const GRAFFITI: [&str; 12] = [
    "NIE PATRZ",
    "ONO TU JEST",
    "UCIEKAJ",
    "ZOSTAN",
    "TO TY",
    "OTWORZ",
    "NIE SPI",
    "ZABIJ MNIE",
    "WROC",
    "NIE UFAJ",
    "JESZCZE ZYJE",
    "SA TU",
];

const OMENS: [&str; 14] = [
    "SLYSZYSZ TO?",
    "ONO CIAGLE IDZIE.",
    "NIE PATRZ ZA SIEBIE.",
    "TO NIE JEST TWOJ DOM.",
    "ZOSTAN Z NAMI.",
    "ZABIERZ SWIATLO.",
    "ONO NIE MA OCZU.",
    "KTOS ODDYCHA W SCIANIE.",
    "NIE UFAJ CISZY.",
    "DRZWI SA BLIZEJ NIZ MYSLISZ.",
    "TWOJ CIEN SIE PORUSZA.",
    "JESZCZE NIE JESTES MARTWY.",
    "ONO ZNA TWOJE IMIE.",
    "NIE OGLADAJ SIE.",
];

#[derive(Debug, Clone, Copy)]
pub enum GameEvent {
    Pickup,
    Hit,
    Heartbeat(f32),
    Whisper,
    Locked,
    Step,
}

pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub knock: Vec2,
    pub aim: f32,
    pub health: f32,
    pub sanity: f32,
    pub stamina: f32,
    pub relics: usize,
    pub light_on: bool,
    pub light_level: f32,
    pub invuln: f32,
    pub walk_phase: f32,
    pub moving: bool,
    pub sprinting: bool,
    pub won: bool,
}

struct Monster {
    pos: Vec2,
    variant: usize,
    phase: f32,
    face: f32,
    speed: f32,
    path: Vec<Vec2>,
    path_timer: f32,
    alert: f32,
    stun: f32,
}

struct Relic {
    pos: Vec2,
    phase: f32,
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max_life: f32,
    size: f32,
    color: Color,
    gravity: f32,
}

struct Decal {
    pos: Vec2,
    radius: f32,
    seed: u32,
}

struct Eye {
    pos: Vec2,
    phase: f32,
}

struct GraffitiTag {
    pos: Vec2,
    text: usize,
    angle: f32,
}

struct Phantom {
    pos: Vec2,
    timer: f32,
    total: f32,
    variant: usize,
}

struct Omen {
    text: String,
    timer: f32,
    total: f32,
}

pub struct Game {
    pub map: Map,
    pub player: Player,
    monsters: Vec<Monster>,
    relics: Vec<Relic>,
    pub exit: Vec2,
    pub camera: Vec2,
    pub time: f32,
    pub fear: f32,
    pub damage_flash: f32,
    pub flash: f32,
    particles: Vec<Particle>,
    decals: Vec<Decal>,
    eyes: Vec<Eye>,
    graffiti: Vec<GraffitiTag>,
    phantom: Option<Phantom>,
    omen: Option<Omen>,
    events: Vec<GameEvent>,
    rng: Rng,
    total_relics: usize,
    pub near_relic: bool,
    pub near_exit: bool,
    pub aim_on_monster: bool,
    omen_cooldown: f32,
    hallucination_cooldown: f32,
    heart_cooldown: f32,
    lock_cooldown: f32,
    step_cooldown: f32,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let map = Map::generate(&mut rng);
        // Start blisko srodka mapy: kamera nie jest wtedy przycieta do
        // krawedzi, a gracz od razu widzi labirynt wokol siebie.
        let (sx, sy) = map
            .floor_tiles
            .iter()
            .copied()
            .min_by_key(|(x, y)| {
                let dx = *x - map.w as i32 / 2;
                let dy = *y - map.h as i32 / 2;
                dx * dx + dy * dy
            })
            .unwrap_or((1, 1));
        let spawn = Map::tile_center(sx, sy);
        let dist = map.distances_from(spawn);
        let (ex, ey) = map.farthest_floor_tile(&dist);
        let exit = Map::tile_center(ex, ey);

        // --- relikwie: daleko od siebie i od startu ---
        let mut candidates: Vec<(i32, i32)> = map
            .floor_tiles
            .iter()
            .copied()
            .filter(|(x, y)| {
                let d = dist[map.idx(*x, *y)];
                d != u32::MAX && d >= 6 && ((*x - sx).abs() + (*y - sy).abs()) > 4
            })
            .collect();
        rng.shuffle(&mut candidates);

        let mut relic_tiles: Vec<(i32, i32)> = Vec::new();
        for &(x, y) in &candidates {
            if relic_tiles.len() >= TOTAL_RELICS {
                break;
            }
            let spaced = relic_tiles
                .iter()
                .all(|&(rx, ry)| (rx - x).abs() + (ry - y).abs() >= 8);
            let away_exit = (x - ex).abs() + (y - ey).abs() >= 3;
            if spaced && away_exit {
                relic_tiles.push((x, y));
            }
        }
        if relic_tiles.len() < TOTAL_RELICS {
            for &(x, y) in &candidates {
                if relic_tiles.len() >= TOTAL_RELICS {
                    break;
                }
                if !relic_tiles.contains(&(x, y)) && (x - ex).abs() + (y - ey).abs() >= 2 {
                    relic_tiles.push((x, y));
                }
            }
        }
        if relic_tiles.len() < TOTAL_RELICS {
            for &(x, y) in &map.floor_tiles {
                if relic_tiles.len() >= TOTAL_RELICS {
                    break;
                }
                if (x, y) != (ex, ey) && (x, y) != (1, 1) && !relic_tiles.contains(&(x, y)) {
                    relic_tiles.push((x, y));
                }
            }
        }

        let relics: Vec<Relic> = relic_tiles
            .iter()
            .map(|&(x, y)| Relic {
                pos: Map::tile_center(x, y),
                phase: rng.range(0.0, TAU),
            })
            .collect();

        // --- potwory: daleko od gracza, ale blisko siebie robi sie ciasno ---
        let mut monster_candidates: Vec<(i32, i32)> = map
            .floor_tiles
            .iter()
            .copied()
            .filter(|(x, y)| {
                let d = dist[map.idx(*x, *y)];
                let near_exit = (x - ex).abs() + (y - ey).abs() < 4;
                let near_relic = relic_tiles
                    .iter()
                    .any(|&(rx, ry)| (rx - x).abs() + (ry - y).abs() < 4);
                d != u32::MAX && d >= 14 && !near_exit && !near_relic
            })
            .collect();
        rng.shuffle(&mut monster_candidates);

        let mut monster_tiles: Vec<(i32, i32)> = Vec::new();
        for &(x, y) in &monster_candidates {
            if monster_tiles.len() >= 3 {
                break;
            }
            if monster_tiles
                .iter()
                .all(|&(mx, my)| (mx - x).abs() + (my - y).abs() >= 6)
            {
                monster_tiles.push((x, y));
            }
        }
        if monster_tiles.len() < 3 {
            for &(x, y) in &monster_candidates {
                if monster_tiles.len() >= 3 {
                    break;
                }
                if !monster_tiles.contains(&(x, y)) {
                    monster_tiles.push((x, y));
                }
            }
        }

        let monsters: Vec<Monster> = monster_tiles
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                let pos = Map::tile_center(x, y);
                Monster {
                    pos,
                    variant: i % 3,
                    phase: rng.range(0.0, TAU),
                    face: rng.range(0.0, TAU),
                    speed: 56.0 + (i % 3) as f32 * 4.0 + rng.range(0.0, 8.0),
                    path: Vec::new(),
                    path_timer: rng.range(0.0, 0.5),
                    alert: 0.0,
                    stun: 0.0,
                }
            })
            .collect();

        // --- oczy w scianach ---
        let mut eyes = Vec::new();
        for y in 1..map.h as i32 - 1 {
            for x in 1..map.w as i32 - 1 {
                if !map.is_solid(x, y) {
                    continue;
                }
                let near_floor = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(dx, dy)| !map.is_solid(x + dx, y + dy));
                if near_floor && hash2(x, y, seed as u32 ^ 0x51A7) < 0.075 && eyes.len() < 48 {
                    eyes.push(Eye {
                        pos: Map::tile_center(x, y)
                            + vec2(rng.range(-14.0, 14.0), rng.range(-12.0, 12.0)),
                        phase: rng.range(0.0, TAU),
                    });
                }
            }
        }

        // --- graffiti ---
        let mut tag_tiles: Vec<(i32, i32)> = map
            .floor_tiles
            .iter()
            .copied()
            .filter(|(x, y)| dist[map.idx(*x, *y)] > 2)
            .collect();
        rng.shuffle(&mut tag_tiles);
        let graffiti: Vec<GraffitiTag> = tag_tiles
            .iter()
            .take(16)
            .enumerate()
            .map(|(i, &(x, y))| GraffitiTag {
                pos: Map::tile_center(x, y) + vec2(rng.range(-8.0, 8.0), rng.range(-6.0, 6.0)),
                text: i % GRAFFITI.len(),
                angle: rng.range(-0.30, 0.30),
            })
            .collect();

        // --- stare plamy ---
        let mut decal_tiles = map.floor_tiles.clone();
        rng.shuffle(&mut decal_tiles);
        let decals: Vec<Decal> = decal_tiles
            .iter()
            .take(24)
            .map(|&(x, y)| Decal {
                pos: Map::tile_center(x, y) + vec2(rng.range(-12.0, 12.0), rng.range(-12.0, 12.0)),
                radius: rng.range(5.0, 16.0),
                seed: rng.next_u32(),
            })
            .collect();

        let half = vec2(VIEW_W * 0.5, VIEW_H * 0.5);
        let world = map.world_size();
        let camera = vec2(
            spawn.x.clamp(half.x, (world.x - half.x).max(half.x)),
            spawn.y.clamp(half.y, (world.y - half.y).max(half.y)),
        );

        let mut game = Self {
            map,
            player: Player {
                pos: spawn,
                vel: Vec2::ZERO,
                knock: Vec2::ZERO,
                aim: 0.0,
                health: 100.0,
                sanity: 100.0,
                stamina: 100.0,
                relics: 0,
                light_on: true,
                light_level: 1.0,
                invuln: 0.0,
                walk_phase: 0.0,
                moving: false,
                sprinting: false,
                won: false,
            },
            monsters,
            relics,
            exit,
            camera,
            time: 0.0,
            fear: 0.0,
            damage_flash: 0.0,
            flash: 0.0,
            particles: Vec::new(),
            decals,
            eyes,
            graffiti,
            phantom: None,
            omen: None,
            events: Vec::new(),
            rng,
            total_relics: 0,
            near_relic: false,
            near_exit: false,
            aim_on_monster: false,
            omen_cooldown: 26.0,
            hallucination_cooldown: 12.0,
            heart_cooldown: 0.0,
            lock_cooldown: 0.0,
            step_cooldown: 0.0,
        };
        game.total_relics = game.relics.len();
        game.set_omen("ZNALEZ 5 RELIKWII. WROC DO WYJSCIA.", 6.0);
        game
    }

    pub fn total_relics(&self) -> usize {
        self.total_relics
    }

    fn set_omen(&mut self, text: &str, duration: f32) {
        self.omen = Some(Omen {
            text: text.to_string(),
            timer: duration,
            total: duration,
        });
    }

    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn update(&mut self, dt: f32, mouse_virtual: Vec2) {
        let dt = dt.clamp(0.0, 0.05);
        self.time += dt;
        self.update_camera(dt);

        let mouse_world = self.camera + (mouse_virtual - vec2(VIEW_W * 0.5, VIEW_H * 0.5));
        let to_mouse = mouse_world - self.player.pos;
        if to_mouse.length_squared() > 1.0 {
            self.player.aim = to_mouse.y.atan2(to_mouse.x);
        }

        self.update_player(dt);
        self.update_monsters(dt);
        self.update_interactions();
        self.update_sanity(dt);
        self.update_events(dt);
        self.update_particles(dt);

        self.aim_on_monster = self
            .monsters
            .iter()
            .any(|m| m.pos.distance_squared(mouse_world) < 36.0 * 36.0);

        self.damage_flash = (self.damage_flash - dt * 2.2).max(0.0);
        self.flash = (self.flash - dt * 1.8).max(0.0);
        self.player.invuln = (self.player.invuln - dt).max(0.0);
        self.lock_cooldown = (self.lock_cooldown - dt).max(0.0);
    }

    fn update_camera(&mut self, dt: f32) {
        let half = vec2(VIEW_W * 0.5, VIEW_H * 0.5);
        let world = self.map.world_size();
        let target = vec2(
            self.player
                .pos
                .x
                .clamp(half.x, (world.x - half.x).max(half.x)),
            self.player
                .pos
                .y
                .clamp(half.y, (world.y - half.y).max(half.y)),
        );
        let t = 1.0 - (-7.0 * dt).exp();
        self.camera = self.camera.lerp(target, t);
    }

    fn update_player(&mut self, dt: f32) {
        let mut input = Vec2::ZERO;
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            input.y -= 1.0;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            input.y += 1.0;
        }
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            input.x -= 1.0;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            input.x += 1.0;
        }
        let input = input.normalize_or_zero();
        let want_sprint = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let sprinting = want_sprint && self.player.stamina > 1.0 && input.length_squared() > 0.0;
        self.player.sprinting = sprinting;

        let speed = if sprinting { 232.0 } else { 148.0 };
        let target = input * speed;
        let accel = if input.length_squared() > 0.0 {
            13.0
        } else {
            9.0
        };
        self.player.vel = self.player.vel.lerp(target, (accel * dt).min(1.0));

        if sprinting {
            self.player.stamina = (self.player.stamina - 34.0 * dt).max(0.0);
        } else {
            self.player.stamina = (self.player.stamina + 19.0 * dt).min(100.0);
        }

        self.player.knock *= (-dt * 7.0).exp();
        let delta = (self.player.vel + self.player.knock) * dt;
        move_circle(&self.map, &mut self.player.pos, delta, PLAYER_RADIUS);

        self.player.moving = self.player.vel.length_squared() > 25.0;
        if self.player.moving {
            self.player.walk_phase += dt * if sprinting { 16.0 } else { 10.0 };
            self.step_cooldown -= dt;
            if self.step_cooldown <= 0.0 {
                self.step_cooldown = if sprinting { 0.26 } else { 0.40 };
                self.events.push(GameEvent::Step);
                for _ in 0..2 {
                    let a = self.rng.range(0.0, TAU);
                    let life = self.rng.range(0.2, 0.45);
                    self.particles.push(Particle {
                        pos: self.player.pos + vec2(0.0, 10.0),
                        vel: vec2(a.cos(), a.sin()) * self.rng.range(4.0, 18.0),
                        life,
                        max_life: life,
                        size: self.rng.range(1.0, 2.2),
                        color: Color::new(0.25, 0.22, 0.19, 0.35),
                        gravity: -10.0,
                    });
                }
            }
        }

        if is_key_pressed(KeyCode::F) {
            self.player.light_on = !self.player.light_on;
            if !self.player.light_on {
                self.set_omen("CIEMNOSC. ONE TO LUBIA.", 2.4);
            }
        }
        let target_light = if self.player.light_on { 1.0 } else { 0.0 };
        self.player.light_level += (target_light - self.player.light_level) * (dt * 9.0).min(1.0);
    }

    fn update_monsters(&mut self, dt: f32) {
        let mut monsters = std::mem::take(&mut self.monsters);
        for m in &mut monsters {
            m.phase += dt * 2.0;
            if m.stun > 0.0 {
                m.stun -= dt;
                continue;
            }

            let dist = m.pos.distance(self.player.pos);
            let sees = dist < 430.0 && self.map.line_of_sight(m.pos, self.player.pos);
            if sees {
                m.alert = 4.0;
            } else {
                m.alert = (m.alert - dt).max(0.0);
            }

            m.path_timer -= dt;
            if m.path_timer <= 0.0 {
                let target = if sees || m.alert > 0.0 {
                    self.player.pos
                } else {
                    wander_target(&self.map, &mut self.rng, m.pos)
                };
                m.path = self.map.bfs_path(m.pos, target);
                m.path_timer = 0.35 + self.rng.range(0.0, 0.2);
            }

            let insane_boost = 1.0 + (1.0 - self.player.sanity / 100.0) * 0.28;
            let speed = m.speed * if m.alert > 0.0 { 1.18 } else { 0.72 } * insane_boost;

            let mut dir = Vec2::ZERO;
            while let Some(&wp) = m.path.first() {
                let to = wp - m.pos;
                if to.length() < 10.0 {
                    m.path.remove(0);
                } else {
                    dir = to.normalize_or_zero();
                    break;
                }
            }
            if dir.length_squared() < 0.001 && (sees || m.alert > 0.0) {
                dir = (self.player.pos - m.pos).normalize_or_zero();
            }
            if dir.length_squared() > 0.001 {
                m.face = dir.y.atan2(dir.x);
                let delta = dir * speed * dt;
                let before = m.pos;
                move_circle(&self.map, &mut m.pos, delta, MONSTER_RADIUS);
                if m.pos.distance_squared(before) < 0.01 {
                    m.path_timer = 0.0;
                }
            } else if dist < 240.0 {
                m.face = (self.player.pos - m.pos)
                    .y
                    .atan2(self.player.pos.x - m.pos.x);
            }

            // dotyk potwora
            if m.pos.distance(self.player.pos) < 30.0 && self.player.invuln <= 0.0 {
                let away = (self.player.pos - m.pos).normalize_or_zero();
                let dmg = 14.0 + m.variant as f32 * 3.5;
                self.player.health = (self.player.health - dmg).max(0.0);
                self.player.sanity = (self.player.sanity - 9.0).max(0.0);
                self.player.invuln = 1.15;
                self.player.knock = away * 430.0;
                self.damage_flash = 1.0;
                self.fear = 1.0;
                self.flash = 0.35;
                m.stun = 1.35;
                m.path.clear();
                self.events.push(GameEvent::Hit);
                spawn_burst(
                    &mut self.particles,
                    &mut self.rng,
                    self.player.pos,
                    26,
                    Color::new(0.42, 0.02, 0.02, 1.0),
                    150.0,
                );
                self.decals.push(Decal {
                    pos: self.player.pos + away * 14.0,
                    radius: self.rng.range(8.0, 18.0),
                    seed: self.rng.next_u32(),
                });
                if self.decals.len() > 80 {
                    self.decals.remove(0);
                }
                if self.player.health <= 0.0 {
                    self.events.push(GameEvent::Whisper);
                }
            }
        }
        self.monsters = monsters;
    }

    fn update_interactions(&mut self) {
        self.near_relic = false;
        let mut best = f32::MAX;
        for r in &self.relics {
            let d = r.pos.distance_squared(self.player.pos);
            if d < 50.0 * 50.0 && d < best {
                best = d;
                self.near_relic = true;
            }
        }
        self.near_exit = self.exit.distance_squared(self.player.pos) < 64.0 * 64.0;

        if !is_key_pressed(KeyCode::E) || self.player.won {
            return;
        }

        if self.near_relic {
            let mut best_idx = None;
            let mut best_d = f32::MAX;
            for (i, r) in self.relics.iter().enumerate() {
                let d = r.pos.distance_squared(self.player.pos);
                if d < 50.0 * 50.0 && d < best_d {
                    best_d = d;
                    best_idx = Some(i);
                }
            }
            if let Some(i) = best_idx {
                let relic = self.relics.remove(i);
                self.player.relics += 1;
                self.player.sanity = (self.player.sanity + 28.0).min(100.0);
                self.player.health = (self.player.health + 8.0).min(100.0);
                self.flash = 0.8;
                self.events.push(GameEvent::Pickup);
                spawn_burst(
                    &mut self.particles,
                    &mut self.rng,
                    relic.pos,
                    34,
                    Color::new(0.95, 0.55, 0.12, 1.0),
                    120.0,
                );
                for m in &mut self.monsters {
                    m.alert = 5.0;
                    m.path_timer = 0.0;
                }
                if self.player.relics >= self.total_relics {
                    self.set_omen("WYJSCIE ODBLOKOWANE. NIE OGLADAJ SIE ZA SIEBIE.", 5.0);
                } else {
                    self.set_omen("RELIKWIA ZABRANA. ONO SIE ZBUDZILO.", 3.5);
                }
            }
        } else if self.near_exit {
            if self.player.relics >= self.total_relics {
                self.player.won = true;
                self.flash = 1.0;
            } else if self.lock_cooldown <= 0.0 {
                self.lock_cooldown = 2.5;
                self.events.push(GameEvent::Locked);
                self.set_omen(
                    &format!(
                        "ZAMKNIETE. BRAKUJE {} RELIKWII.",
                        self.total_relics - self.player.relics
                    ),
                    3.0,
                );
            }
        }
    }

    fn update_sanity(&mut self, dt: f32) {
        let nearest = self
            .monsters
            .iter()
            .map(|m| m.pos.distance(self.player.pos))
            .fold(f32::MAX, f32::min);

        let target_fear = if nearest < 260.0 {
            (1.0 - nearest / 260.0).clamp(0.15, 1.0)
        } else if self.player.light_level < 0.4 {
            0.35
        } else {
            0.08
        };
        self.fear += (target_fear - self.fear) * (dt * 2.0).min(1.0);

        if nearest < 260.0 {
            self.player.sanity -= (1.8 + (1.0 - nearest / 260.0) * 5.5) * dt;
        } else if self.player.light_level > 0.5 {
            self.player.sanity += 0.12 * dt;
        } else {
            self.player.sanity -= 0.9 * dt;
        }
        self.player.sanity = self.player.sanity.clamp(0.0, 100.0);

        if self.player.sanity <= 0.0 {
            self.player.health = (self.player.health - 2.0 * dt).max(0.0);
            self.damage_flash = self.damage_flash.max(0.25);
        }
    }

    fn update_events(&mut self, dt: f32) {
        // bicie serca
        let nearest = self
            .monsters
            .iter()
            .map(|m| m.pos.distance(self.player.pos))
            .fold(f32::MAX, f32::min);
        if nearest < 300.0 {
            self.heart_cooldown -= dt;
            if self.heart_cooldown <= 0.0 {
                let vol = (0.35 + (1.0 - nearest / 300.0) * 0.65).clamp(0.0, 1.0);
                self.events.push(GameEvent::Heartbeat(vol));
                self.heart_cooldown = (0.45 + nearest / 300.0 * 0.8).max(0.3);
            }
        } else {
            self.heart_cooldown = 0.25;
        }

        // omamy
        if let Some(p) = &mut self.phantom {
            p.timer -= dt;
            if p.timer <= 0.0 {
                self.phantom = None;
            }
        }
        self.hallucination_cooldown -= dt;
        if self.phantom.is_none() && self.hallucination_cooldown <= 0.0 && self.player.sanity < 62.0
        {
            let a = self.rng.range(0.0, TAU);
            let d = self.rng.range(110.0, 210.0);
            self.phantom = Some(Phantom {
                pos: self.player.pos + vec2(a.cos(), a.sin()) * d,
                timer: self.rng.range(1.4, 2.6),
                total: 2.6,
                variant: self.rng.usize(3),
            });
            self.hallucination_cooldown = self.rng.range(8.0, 17.0);
            self.events.push(GameEvent::Whisper);
        }

        // teksty z ciemnosci
        if let Some(o) = &mut self.omen {
            o.timer -= dt;
        }
        if self.omen.as_ref().is_some_and(|o| o.timer <= 0.0) {
            self.omen = None;
        }
        self.omen_cooldown -= dt;
        if self.omen_cooldown <= 0.0 && self.omen.is_none() {
            let text = OMENS[self.rng.usize(OMENS.len())];
            self.set_omen(text, 3.8);
            self.omen_cooldown = self.rng.range(18.0, 34.0);
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.life -= dt;
            p.pos += p.vel * dt;
            p.vel *= (-dt * 2.0).exp();
            p.vel.y += p.gravity * dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        if self.particles.len() > 340 {
            let excess = self.particles.len() - 340;
            self.particles.drain(0..excess);
        }
    }

    pub fn post_params(&self, clock: f32) -> PostParams {
        let player_screen = (self.player.pos - self.camera + vec2(VIEW_W * 0.5, VIEW_H * 0.5))
            / vec2(VIEW_W, VIEW_H);
        PostParams {
            time: clock,
            sanity: self.player.sanity / 100.0,
            fear: self.fear,
            flash: self.flash,
            player: player_screen,
            light_dir: vec2(self.player.aim.cos(), self.player.aim.sin()),
            light_on: self.player.light_level,
            damage: self.damage_flash,
            ambient: 0.0,
        }
    }

    pub fn draw_world(&self, assets: &Assets) {
        clear_background(Color::new(0.004, 0.002, 0.005, 1.0));

        let half = vec2(VIEW_W * 0.5, VIEW_H * 0.5);
        let min = self.camera - half - vec2(TILE, TILE);
        let max = self.camera + half + vec2(TILE, TILE);
        let tx0 = (min.x / TILE).floor() as i32;
        let ty0 = (min.y / TILE).floor() as i32;
        let tx1 = (max.x / TILE).ceil() as i32;
        let ty1 = (max.y / TILE).ceil() as i32;

        // podlogi
        for y in ty0..=ty1 {
            for x in tx0..=tx1 {
                if self.map.is_solid(x, y) {
                    continue;
                }
                let idx = self.map.idx(x, y);
                let v = self.map.variant[idx] as usize % assets.floors.len();
                let tint = 0.80 + hash2(x, y, 123) * 0.20;
                let p = Map::tile_center(x, y) - vec2(TILE * 0.5, TILE * 0.5);
                draw_texture_ex(
                    &assets.floors[v],
                    p.x,
                    p.y,
                    Color::new(tint, tint, tint, 1.0),
                    DrawTextureParams {
                        dest_size: Some(vec2(TILE, TILE)),
                        flip_x: hash2(x, y, 17) < 0.5,
                        flip_y: hash2(x, y, 19) < 0.5,
                        ..Default::default()
                    },
                );
            }
        }

        // plamy
        for d in &self.decals {
            if !self.in_view(d.pos, 40.0) {
                continue;
            }
            draw_circle(
                d.pos.x,
                d.pos.y,
                d.radius,
                Color::new(0.18, 0.005, 0.008, 0.50),
            );
            for i in 0..4 {
                let a = hash2(i, d.seed as i32, 4) * TAU;
                let r = d.radius * (0.4 + hash2(i, d.seed as i32, 8) * 0.9);
                draw_circle(
                    d.pos.x + a.cos() * r,
                    d.pos.y + a.sin() * r,
                    d.radius * (0.18 + hash2(i, d.seed as i32, 12) * 0.25),
                    Color::new(0.12, 0.003, 0.005, 0.42),
                );
            }
        }

        // graffiti
        for g in &self.graffiti {
            if !self.in_view(g.pos, 80.0) {
                continue;
            }
            draw_text_ex(
                GRAFFITI[g.text],
                g.pos.x - 24.0,
                g.pos.y,
                TextParams {
                    font_size: 15,
                    color: Color::new(0.32, 0.015, 0.02, 0.80),
                    rotation: g.angle,
                    ..Default::default()
                },
            );
        }

        // sciany
        for y in ty0..=ty1 {
            for x in tx0..=tx1 {
                if !self.map.in_bounds(x, y) || !self.map.is_solid(x, y) {
                    continue;
                }
                let idx = self.map.idx(x, y);
                let v = self.map.variant[idx] as usize % assets.walls.len();
                let tint = 0.78 + hash2(x, y, 321) * 0.22;
                let p = Map::tile_center(x, y) - vec2(TILE * 0.5, TILE * 0.5);
                draw_rectangle(
                    p.x + 5.0,
                    p.y + 6.0,
                    TILE,
                    TILE,
                    Color::new(0.0, 0.0, 0.0, 0.55),
                );
                draw_texture_ex(
                    &assets.walls[v],
                    p.x,
                    p.y,
                    Color::new(tint, tint, tint, 1.0),
                    DrawTextureParams {
                        dest_size: Some(vec2(TILE, TILE)),
                        flip_x: hash2(x, y, 23) < 0.5,
                        flip_y: hash2(x, y, 29) < 0.5,
                        ..Default::default()
                    },
                );
            }
        }

        // oczy w scianach
        for e in &self.eyes {
            if !self.in_view(e.pos, 30.0) {
                continue;
            }
            let blink = (self.time * 0.7 + e.phase).sin();
            if blink < -0.94 {
                continue;
            }
            let to_player = (self.player.pos - e.pos).normalize_or_zero() * 1.1;
            draw_circle(e.pos.x, e.pos.y, 8.0, Color::new(0.30, 0.0, 0.0, 0.10));
            for side in [-3.2f32, 3.2] {
                let ex = e.pos.x + side;
                let ey = e.pos.y;
                draw_circle(ex, ey, 2.8, Color::new(0.72, 0.60, 0.46, 0.9));
                draw_circle(
                    ex + to_player.x,
                    ey + to_player.y,
                    1.3,
                    Color::new(0.95, 0.04, 0.02, 0.95),
                );
                draw_circle(ex + to_player.x, ey + to_player.y, 0.55, BLACK);
            }
        }

        // wyjscie
        self.draw_exit(assets);

        // relikwie
        for r in &self.relics {
            if !self.in_view(r.pos, 60.0) {
                continue;
            }
            let bob = (self.time * 2.2 + r.phase).sin() * 3.0;
            let p = r.pos + vec2(0.0, bob);
            for i in 0..4 {
                draw_circle(
                    p.x,
                    p.y - 8.0,
                    8.0 + i as f32 * 8.0,
                    Color::new(0.95, 0.40, 0.05, 0.035),
                );
            }
            draw_texture_ex(
                &assets.relic,
                p.x - 10.0,
                p.y - 24.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(20.0, 32.0)),
                    ..Default::default()
                },
            );
        }

        // potwory
        for m in &self.monsters {
            if !self.in_view(m.pos, 90.0) {
                continue;
            }
            draw_circle(
                m.pos.x,
                m.pos.y + 18.0,
                18.0,
                Color::new(0.0, 0.0, 0.0, 0.45),
            );
            draw_circle(m.pos.x, m.pos.y, 30.0, Color::new(0.30, 0.0, 0.0, 0.055));
            let wob = 1.0 + (self.time * 5.5 + m.phase).sin() * 0.035;
            let size = vec2(48.0, 56.0) * wob;
            let rot = m.face + PI * 0.5;
            draw_texture_ex(
                &assets.monsters[m.variant % assets.monsters.len()],
                m.pos.x - size.x * 0.5,
                m.pos.y - size.y * 0.55,
                Color::new(1.0, 0.86, 0.86, 1.0),
                DrawTextureParams {
                    dest_size: Some(size),
                    rotation: rot,
                    ..Default::default()
                },
            );
            if self.fear > 0.45 {
                let glitch = (self.time * 19.0 + m.phase).sin() * 4.0;
                draw_texture_ex(
                    &assets.monsters[m.variant % assets.monsters.len()],
                    m.pos.x - size.x * 0.5 + glitch,
                    m.pos.y - size.y * 0.55,
                    Color::new(0.7, 0.1, 0.1, 0.13),
                    DrawTextureParams {
                        dest_size: Some(size),
                        rotation: rot,
                        ..Default::default()
                    },
                );
            }
        }

        // omamy
        if let Some(p) = &self.phantom {
            let a = (p.timer / p.total).clamp(0.0, 1.0) * 0.32;
            let jitter = (self.time * 23.0).sin() * 5.0;
            draw_texture_ex(
                &assets.monsters[p.variant % assets.monsters.len()],
                p.pos.x - 28.0 + jitter,
                p.pos.y - 34.0,
                Color::new(0.85, 0.85, 0.85, a),
                DrawTextureParams {
                    dest_size: Some(vec2(56.0, 66.0)),
                    rotation: (self.time * 0.7).sin() * 0.15,
                    ..Default::default()
                },
            );
        }

        // gracz
        let bob = if self.player.moving {
            (self.player.walk_phase).sin() * 1.2
        } else {
            (self.time * 1.8).sin() * 0.5
        };
        // cieply oddech latarki wokol gracza
        draw_circle(
            self.player.pos.x,
            self.player.pos.y,
            34.0,
            Color::new(1.0, 0.52, 0.20, 0.045),
        );
        draw_circle(
            self.player.pos.x,
            self.player.pos.y,
            21.0,
            Color::new(1.0, 0.62, 0.28, 0.060),
        );
        draw_ellipse(
            self.player.pos.x,
            self.player.pos.y + 18.0,
            15.0,
            7.0,
            0.0,
            Color::new(0.0, 0.0, 0.0, 0.55),
        );
        let size = vec2(46.0, 60.0);
        let rot = self.player.aim + PI * 0.5;
        let center = vec2(self.player.pos.x, self.player.pos.y - 3.0 + bob);
        let outline = size * 1.12;
        draw_texture_ex(
            &assets.player,
            center.x - outline.x * 0.5,
            center.y - outline.y * 0.5,
            Color::new(0.62, 0.035, 0.03, 0.38),
            DrawTextureParams {
                dest_size: Some(outline),
                rotation: rot,
                ..Default::default()
            },
        );
        draw_texture_ex(
            &assets.player,
            center.x - size.x * 0.5,
            center.y - size.y * 0.5,
            WHITE,
            DrawTextureParams {
                dest_size: Some(size),
                rotation: rot,
                ..Default::default()
            },
        );

        // czastki
        for p in &self.particles {
            let a = (p.life / p.max_life).clamp(0.0, 1.0);
            let c = Color::new(p.color.r, p.color.g, p.color.b, p.color.a * a);
            draw_circle(p.pos.x, p.pos.y, p.size * (0.4 + a * 0.6), c);
        }
    }

    fn draw_exit(&self, assets: &Assets) {
        if !self.in_view(self.exit, 100.0) {
            return;
        }
        let unlocked = self.player.relics >= self.total_relics;
        let pulse = 0.55 + (self.time * 3.0).sin() * 0.25;
        let col = if unlocked {
            Color::new(0.9, 0.65, 0.12, 0.10 * pulse)
        } else {
            Color::new(0.8, 0.02, 0.02, 0.10 * pulse)
        };
        draw_circle(self.exit.x, self.exit.y, 38.0, col);
        draw_circle(self.exit.x, self.exit.y, 24.0, col);
        let x = self.exit.x - 24.0;
        let y = self.exit.y - 40.0;
        draw_rectangle(
            x - 4.0,
            y - 4.0,
            56.0,
            72.0,
            Color::new(0.0, 0.0, 0.0, 0.65),
        );
        draw_texture_ex(
            &assets.door,
            x,
            y,
            if unlocked {
                Color::new(1.0, 0.9, 0.65, 1.0)
            } else {
                Color::new(0.85, 0.75, 0.75, 1.0)
            },
            DrawTextureParams {
                dest_size: Some(vec2(48.0, 64.0)),
                ..Default::default()
            },
        );
        let label = if unlocked { "WYJDZ" } else { "ZAMKNIETE" };
        draw_text_ex(
            label,
            self.exit.x - 22.0,
            y - 10.0,
            TextParams {
                font_size: 13,
                color: if unlocked {
                    Color::new(1.0, 0.8, 0.35, 0.9)
                } else {
                    Color::new(0.9, 0.12, 0.10, 0.8)
                },
                ..Default::default()
            },
        );
    }

    fn in_view(&self, p: Vec2, margin: f32) -> bool {
        let half = vec2(VIEW_W * 0.5 + margin, VIEW_H * 0.5 + margin);
        let d = (p - self.camera).abs();
        d.x < half.x && d.y < half.y
    }

    pub fn draw_hud(&self, assets: &Assets) {
        let sw = screen_width();
        let sh = screen_height();
        let s = (sh / 720.0).max(0.55);
        let clock = get_time() as f32;

        // panel statusu
        draw_rectangle(
            16.0 * s,
            16.0 * s,
            302.0 * s,
            112.0 * s,
            Color::new(0.01, 0.0, 0.008, 0.50),
        );
        draw_rectangle(
            16.0 * s,
            16.0 * s,
            302.0 * s,
            1.5 * s,
            Color::new(0.5, 0.06, 0.05, 0.35),
        );

        draw_stat_bar(
            30.0 * s,
            44.0 * s,
            250.0 * s,
            11.0 * s,
            self.player.health / 100.0,
            Color::new(0.68, 0.03, 0.04, 0.95),
            "ZYCIE",
            clock,
            s,
        );
        draw_stat_bar(
            30.0 * s,
            78.0 * s,
            250.0 * s,
            9.0 * s,
            self.player.sanity / 100.0,
            Color::new(0.24, 0.55, 0.45, 0.95),
            "PAMIEC",
            clock,
            s,
        );
        draw_stat_bar(
            30.0 * s,
            108.0 * s,
            250.0 * s,
            7.0 * s,
            self.player.stamina / 100.0,
            Color::new(0.72, 0.48, 0.08, 0.9),
            "ODDECH",
            clock,
            s,
        );

        // relikwie / czas
        let rt = format!("RELIKWIE  {}/{}", self.player.relics, self.total_relics);
        let dims = measure_text(&rt, None, (23.0 * s) as u16, 1.0);
        glitch_text(
            &rt,
            sw - 26.0 * s - dims.width,
            46.0 * s,
            23.0 * s,
            Color::new(0.82, 0.74, 0.60, 0.95),
            clock,
            1.2 * s,
        );
        let time = format_time(self.time);
        let dims = measure_text(&time, None, (18.0 * s) as u16, 1.0);
        draw_text_ex(
            &time,
            sw - 26.0 * s - dims.width,
            72.0 * s,
            TextParams {
                font_size: (18.0 * s) as u16,
                color: Color::new(0.55, 0.50, 0.44, 0.75),
                ..Default::default()
            },
        );

        // cel
        if self.player.relics >= self.total_relics {
            glitch_text_centered(
                "WYJSCIE ODBLOKOWANE",
                sw * 0.5,
                38.0 * s,
                19.0 * s,
                Color::new(0.92, 0.72, 0.30, 0.85),
                clock,
                1.0 * s,
            );
        }

        // celownik
        if !is_paused_ui() {
            let (mx, my) = mouse_position();
            let col = if self.aim_on_monster {
                Color::new(1.0, 0.12, 0.08, 0.95)
            } else {
                Color::new(0.82, 0.78, 0.68, 0.80)
            };
            let r = 9.0 * s;
            draw_line(mx - r, my, mx - 3.0 * s, my, 1.6 * s, col);
            draw_line(mx + 3.0 * s, my, mx + r, my, 1.6 * s, col);
            draw_line(mx, my - r, mx, my - 3.0 * s, 1.6 * s, col);
            draw_line(mx, my + 3.0 * s, mx, my + r, 1.6 * s, col);
            draw_circle(mx, my, 1.2 * s, col);
        }

        // strzalka do wyjscia po zebraniu relikwii
        if self.player.relics >= self.total_relics {
            let sx = sw / VIEW_W;
            let sy = sh / VIEW_H;
            let ps = vec2(
                (self.player.pos.x - self.camera.x + VIEW_W * 0.5) * sx,
                (self.player.pos.y - self.camera.y + VIEW_H * 0.5) * sy,
            );
            let dir = (self.exit - self.player.pos).normalize_or_zero();
            let end = ps + vec2(dir.x * sx, dir.y * sy) * 72.0;
            draw_line(
                ps.x,
                ps.y,
                end.x,
                end.y,
                2.2 * s,
                Color::new(0.9, 0.15, 0.08, 0.35),
            );
            draw_circle(end.x, end.y, 4.0 * s, Color::new(1.0, 0.25, 0.08, 0.5));
        }

        // podpowiedz przy obiekcie
        if self.near_relic {
            glitch_text_centered(
                "E   ZABIERZ RELIKWIE",
                sw * 0.5,
                sh - 74.0 * s,
                22.0 * s,
                Color::new(0.95, 0.78, 0.45, 0.95),
                clock,
                0.8 * s,
            );
        } else if self.near_exit {
            let txt = if self.player.relics >= self.total_relics {
                "E   WYJDZ".to_string()
            } else {
                format!("ZABLOKOWANE   {}/{}", self.player.relics, self.total_relics)
            };
            glitch_text_centered(
                &txt,
                sw * 0.5,
                sh - 74.0 * s,
                22.0 * s,
                if self.player.relics >= self.total_relics {
                    Color::new(0.95, 0.78, 0.35, 0.95)
                } else {
                    Color::new(0.8, 0.12, 0.10, 0.9)
                },
                clock,
                0.8 * s,
            );
        }

        // omen
        if let Some(o) = &self.omen {
            let fade_in = ((o.total - o.timer) / 0.35).clamp(0.0, 1.0);
            let fade_out = (o.timer / 0.8).clamp(0.0, 1.0);
            let a = fade_in.min(fade_out);
            glitch_text_centered(
                &o.text,
                sw * 0.5,
                sh * 0.72,
                24.0 * s,
                Color::new(0.72, 0.16, 0.12, a * 0.95),
                clock,
                1.6 * s,
            );
        }

        // ostrzezenie o sanity
        if self.player.sanity < 20.0 {
            let a = 0.35 + (clock * 4.0).sin().abs() * 0.35;
            glitch_text_centered(
                "ODDECH CI SIE KONCZY",
                sw * 0.5,
                sh - 38.0 * s,
                16.0 * s,
                Color::new(0.85, 0.05, 0.04, a),
                clock,
                2.0 * s,
            );
        }

        // wskaznik latarki
        draw_text_ex(
            if self.player.light_on {
                "LATARKA: ON"
            } else {
                "LATARKA: OFF"
            },
            sw - 26.0 * s
                - measure_text(
                    if self.player.light_on {
                        "LATARKA: ON"
                    } else {
                        "LATARKA: OFF"
                    },
                    None,
                    (13.0 * s) as u16,
                    1.0,
                )
                .width,
            94.0 * s,
            TextParams {
                font_size: (13.0 * s) as u16,
                color: Color::new(0.55, 0.50, 0.44, 0.65),
                ..Default::default()
            },
        );

        // tlo pod twarz przy bardzo niskim sanity
        if self.player.sanity < 12.0 {
            let a = 0.05 + (1.0 - self.player.sanity / 12.0) * 0.12;
            draw_texture_ex(
                &assets.face,
                sw * 0.5 - 200.0 * s,
                sh * 0.5 - 200.0 * s,
                Color::new(0.8, 0.6, 0.55, a),
                DrawTextureParams {
                    dest_size: Some(vec2(400.0 * s, 400.0 * s)),
                    ..Default::default()
                },
            );
        }
    }

    pub fn draw_end_screen(&self, assets: &Assets, won: bool) {
        let sw = screen_width();
        let sh = screen_height();
        let s = (sh / 720.0).max(0.55);
        let clock = get_time() as f32;

        draw_rectangle(
            0.0,
            0.0,
            sw,
            sh,
            if won {
                Color::new(0.01, 0.02, 0.01, 0.72)
            } else {
                Color::new(0.10, 0.0, 0.0, 0.70)
            },
        );
        let face_size = 430.0 * s;
        draw_texture_ex(
            &assets.face,
            sw * 0.5 - face_size * 0.5,
            sh * 0.5 - face_size * 0.45,
            Color::new(0.8, 0.65, 0.55, 0.14),
            DrawTextureParams {
                dest_size: Some(vec2(face_size, face_size)),
                ..Default::default()
            },
        );

        // panel pod tekstem - kontrast wobec twarzy i swiata w tle
        let panel_w = 840.0 * s;
        let panel_h = 350.0 * s;
        draw_rectangle(
            sw * 0.5 - panel_w * 0.5,
            sh * 0.29,
            panel_w,
            panel_h,
            Color::new(0.004, 0.001, 0.003, 0.58),
        );
        draw_rectangle_lines(
            sw * 0.5 - panel_w * 0.5,
            sh * 0.29,
            panel_w,
            panel_h,
            1.2 * s,
            Color::new(0.36, 0.03, 0.03, 0.45),
        );

        let (title, sub, col, sub_col) = if won {
            (
                "WYJDZIESZ.",
                "ALE NIE SAM.",
                Color::new(1.0, 0.88, 0.58, 0.98),
                Color::new(0.88, 0.80, 0.62, 0.92),
            )
        } else {
            (
                "NIE ZYJESZ.",
                "ONO ZOSTALO.",
                Color::new(1.0, 0.16, 0.10, 0.98),
                Color::new(0.95, 0.55, 0.45, 0.92),
            )
        };
        glitch_text_centered(title, sw * 0.5, sh * 0.40, 86.0 * s, col, clock, 1.8 * s);
        glitch_text_centered(
            sub,
            sw * 0.5,
            sh * 0.40 + 42.0 * s,
            24.0 * s,
            sub_col,
            clock,
            0.9 * s,
        );

        let stats = format!(
            "PRZEZYTY CZAS {}   //   RELIKWIE {}/{}",
            format_time(self.time),
            self.player.relics,
            self.total_relics
        );
        glitch_text_centered(
            &stats,
            sw * 0.5,
            sh * 0.60,
            20.0 * s,
            Color::new(0.86, 0.81, 0.68, 0.95),
            clock,
            0.5 * s,
        );

        glitch_text_centered(
            "R   NOWA GRA          ESC   MENU",
            sw * 0.5,
            sh * 0.70,
            20.0 * s,
            Color::new(0.92, 0.82, 0.62, 0.95),
            clock,
            0.7 * s,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_stat_bar(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    frac: f32,
    color: Color,
    label: &str,
    clock: f32,
    s: f32,
) {
    draw_text_ex(
        label,
        x,
        y - 5.0 * s,
        TextParams {
            font_size: (13.0 * s) as u16,
            color: Color::new(0.62, 0.56, 0.48, 0.8),
            ..Default::default()
        },
    );
    draw_rectangle(
        x - 1.0,
        y - 1.0,
        w + 2.0,
        h + 2.0,
        Color::new(0.0, 0.0, 0.0, 0.7),
    );
    draw_rectangle(x, y, w, h, Color::new(0.07, 0.045, 0.055, 0.85));
    draw_rectangle(x, y, w * frac.clamp(0.0, 1.0), h, color);
    // glitchowe przerwy
    for i in 0..5 {
        let gx = x + hash2(i, (clock * 5.0) as i32, 31) * w;
        draw_rectangle(
            gx,
            y,
            (1.0 + (i % 2) as f32) * s,
            h,
            Color::new(0.02, 0.0, 0.01, 0.55),
        );
    }
}

pub fn format_time(t: f32) -> String {
    let total = t.max(0.0) as u32;
    format!("{:02}:{:02}", total / 60, total % 60)
}

// Uzywane przez HUD; pause jest obslugiwany w main, wiec celownik jest
// rysowany tylko gdy mysz jest ukryta (w trybie gry).
fn is_paused_ui() -> bool {
    false
}

fn move_circle(map: &Map, pos: &mut Vec2, delta: Vec2, radius: f32) {
    let nx = pos.x + delta.x;
    if map.can_stand(vec2(nx, pos.y), radius) {
        pos.x = nx;
    }
    let ny = pos.y + delta.y;
    if map.can_stand(vec2(pos.x, ny), radius) {
        pos.y = ny;
    }
}

fn wander_target(map: &Map, rng: &mut Rng, pos: Vec2) -> Vec2 {
    let (tx, ty) = Map::tile_of(pos);
    for _ in 0..24 {
        let nx = tx + rng.usize(13) as i32 - 6;
        let ny = ty + rng.usize(13) as i32 - 6;
        if !map.is_solid(nx, ny) {
            return Map::tile_center(nx, ny);
        }
    }
    pos
}

fn spawn_burst(
    particles: &mut Vec<Particle>,
    rng: &mut Rng,
    pos: Vec2,
    count: usize,
    color: Color,
    speed: f32,
) {
    for _ in 0..count {
        let a = rng.range(0.0, TAU);
        let s = rng.range(speed * 0.15, speed);
        let life = rng.range(0.25, 0.9);
        particles.push(Particle {
            pos,
            vel: vec2(a.cos(), a.sin()) * s,
            life,
            max_life: life,
            size: rng.range(1.0, 3.2),
            color,
            gravity: 90.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_places_goal_objects_on_floor() {
        for seed in [7u64, 42, 1234] {
            let game = Game::new(seed);
            assert_eq!(game.total_relics(), 5, "seed {seed}");
            assert_eq!(game.relics.len(), 5, "seed {seed}");
            assert_eq!(game.monsters.len(), 3, "seed {seed}");
            assert!(!game.map.solid_at_world(game.player.pos), "seed {seed}");
            assert!(!game.map.solid_at_world(game.exit), "seed {seed}");
            for relic in &game.relics {
                assert!(!game.map.solid_at_world(relic.pos), "seed {seed}");
            }
            for monster in &game.monsters {
                assert!(!game.map.solid_at_world(monster.pos), "seed {seed}");
            }
        }
    }
}
