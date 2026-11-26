//! Shader post-processingu i pomocnicze efekty wizualne.

use macroquad::prelude::*;

use crate::assets::Assets;
use crate::rng::hash2;

pub const VIEW_W: f32 = 640.0;
pub const VIEW_H: f32 = 360.0;

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying vec2 uv;
varying vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    color = color0 / 255.0;
}
"#;

const FRAGMENT: &str = r#"#version 100
precision highp float;

varying vec2 uv;

uniform sampler2D Texture;
uniform vec2 Resolution;
uniform float Time;
uniform float Sanity;
uniform float Fear;
uniform float Flash;
uniform vec2 Player;
uniform vec2 LightDir;
uniform float LightOn;
uniform float Damage;
uniform float Ambient;

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float hash1(float x) {
    return fract(sin(x * 12.9898) * 43758.5453);
}

void main() {
    vec2 p = vec2(gl_FragCoord.x / Resolution.x, 1.0 - gl_FragCoord.y / Resolution.y);
    float insane = clamp(1.0 - Sanity, 0.0, 1.0);

    // falujacy, chory obraz
    float wob = insane * 0.006 + Fear * 0.0025 + Damage * 0.004;
    vec2 uvw = uv;
    uvw.x += sin(p.y * 27.0 + Time * 2.7) * wob;
    uvw.y += cos(p.x * 21.0 + Time * 1.9) * wob;

    // poziome pasy zaklocen
    float band_id = floor(p.y * 52.0);
    float band = step(0.982, hash1(band_id + floor(Time * 9.0) * 7.13));
    float band_shift = (hash(vec2(band_id, floor(Time * 7.0))) - 0.5)
        * 0.055 * (0.35 + insane + Fear * 0.5);
    uvw.x += band * band_shift;

    // rzadkie rozdarcia
    float tear = step(0.997, hash1(floor(p.y * 140.0) + floor(Time * 3.0) * 3.7));
    uvw.x += tear * (hash(vec2(p.y, Time)) - 0.5) * 0.12;

    // aberracja chromatyczna
    float ca = 0.00035 + Fear * 0.0035 + Damage * 0.006 + insane * 0.0025;
    vec3 col;
    col.r = texture2D(Texture, uvw + vec2(ca, 0.0)).r;
    col.g = texture2D(Texture, uvw).g;
    col.b = texture2D(Texture, uvw - vec2(ca, 0.0)).b;

    // latarka
    float aspect = Resolution.x / max(Resolution.y, 1.0);
    vec2 d = (p - Player) * vec2(aspect, 1.0);
    float dist = length(d);
    vec2 ld = normalize(LightDir * vec2(aspect, 1.0) + vec2(0.0001, 0.0001));
    vec2 nd = normalize(d + vec2(0.0001, 0.0001));
    float cone = smoothstep(0.44, 0.90, dot(nd, ld));
    float flicker = 0.88 + 0.12 * sin(Time * 23.0) * sin(Time * 5.3 + 1.7);
    float fall = 1.0 / (1.0 + dist * dist * 9.0);
    float lamp = cone * fall * 2.9 + 1.0 / (1.0 + dist * dist * 38.0) * 0.42;
    float lit = clamp(LightOn * lamp * flicker + Ambient, 0.0, 1.0);
    float light = mix(0.065, 1.0, lit);

    // emisja: oczy i ogien przebijaja ciemnosc
    float em = smoothstep(0.48, 0.92, max(col.r, max(col.g, col.b)) * (1.0 - min(col.g, col.b)));
    light = mix(light, 1.0, em * 0.55);
    col *= light;

    // lekka mgla w snopie swiatla
    col += vec3(0.14, 0.12, 0.08) * cone * fall * LightOn * 0.25;

    // chora kolorystyka
    float luma = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(vec3(luma) * vec3(0.55, 0.78, 0.60), col, 1.12);
    col.r *= 1.08;
    col.b *= 1.04;

    // scanlines + ziarno
    col *= 0.93 + 0.07 * sin(p.y * 620.0);
    float grain = hash(p * Resolution + vec2(Time * 60.0, Time * 37.0));
    col += (grain - 0.5) * (0.045 + insane * 0.09 + Fear * 0.03);

    // winieta
    vec2 v = p - 0.5;
    float vig = smoothstep(0.82, 0.20, length(v * vec2(1.05, 1.0)));
    col *= mix(0.55, 1.0, vig);

    // czerwone brzegi przy niskim sanity
    float edge = smoothstep(0.22, 0.72, length(v));
    col += vec3(0.25, 0.0, 0.015) * edge * insane * 0.55;

    // blysk / obrazenia
    col += vec3(1.0, 0.9, 0.8) * Flash * 0.6;
    col += vec3(0.65, 0.0, 0.0) * Damage * 0.45;

    // drop-outy linii
    float drop = step(0.9965, hash1(floor(p.y * 220.0) + floor(Time * 15.0)));
    col *= 1.0 - drop * 0.65;

    gl_FragColor = vec4(clamp(col, 0.0, 1.0), 1.0);
}
"#;

pub struct PostFx {
    material: Option<Material>,
}

impl PostFx {
    pub fn new() -> Self {
        let material = match load_material(
            ShaderSource::Glsl {
                vertex: VERTEX,
                fragment: FRAGMENT,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("Resolution", UniformType::Float2),
                    UniformDesc::new("Time", UniformType::Float1),
                    UniformDesc::new("Sanity", UniformType::Float1),
                    UniformDesc::new("Fear", UniformType::Float1),
                    UniformDesc::new("Flash", UniformType::Float1),
                    UniformDesc::new("Player", UniformType::Float2),
                    UniformDesc::new("LightDir", UniformType::Float2),
                    UniformDesc::new("LightOn", UniformType::Float1),
                    UniformDesc::new("Damage", UniformType::Float1),
                    UniformDesc::new("Ambient", UniformType::Float1),
                ],
                ..Default::default()
            },
        ) {
            Ok(mat) => Some(mat),
            Err(err) => {
                eprintln!("[MROK] BLAD SHADERA: {err}");
                None
            }
        };
        Self { material }
    }
}

pub struct PostParams {
    pub time: f32,
    pub sanity: f32,
    pub fear: f32,
    pub flash: f32,
    pub player: Vec2,
    pub light_dir: Vec2,
    pub light_on: f32,
    pub damage: f32,
    pub ambient: f32,
}

impl Default for PostParams {
    fn default() -> Self {
        Self {
            time: 0.0,
            sanity: 1.0,
            fear: 0.0,
            flash: 0.0,
            player: vec2(0.5, 0.5),
            light_dir: vec2(1.0, 0.0),
            light_on: 1.0,
            damage: 0.0,
            ambient: 0.0,
        }
    }
}

pub fn present(rt: &Texture2D, fx: &PostFx, p: &PostParams) {
    let sw = screen_width();
    let sh = screen_height();
    // gl_FragCoord jest w pikselach fizycznych (Retina = 2x), a screen_width()
    // zwraca piksele logiczne. Shader dostaje wiec fizyczny rozmiar bufora.
    let physical = vec2(sw, sh) * screen_dpi_scale();
    clear_background(BLACK);

    let params = DrawTextureParams {
        dest_size: Some(vec2(sw, sh)),
        flip_y: true,
        ..Default::default()
    };

    if let Some(mat) = &fx.material {
        gl_use_material(mat);
        mat.set_uniform("Resolution", physical);
        mat.set_uniform("Time", p.time);
        mat.set_uniform("Sanity", p.sanity.clamp(0.0, 1.0));
        mat.set_uniform("Fear", p.fear.clamp(0.0, 1.0));
        mat.set_uniform("Flash", p.flash.clamp(0.0, 1.0));
        mat.set_uniform("Player", p.player);
        mat.set_uniform("LightDir", p.light_dir);
        mat.set_uniform("LightOn", p.light_on.clamp(0.0, 1.0));
        mat.set_uniform("Damage", p.damage.clamp(0.0, 1.0));
        mat.set_uniform("Ambient", p.ambient.clamp(0.0, 1.0));
        draw_texture_ex(rt, 0.0, 0.0, WHITE, params);
        gl_use_default_material();
    } else {
        draw_texture_ex(rt, 0.0, 0.0, WHITE, params);
    }
}

/// Tlo menu: tapeta z koszmarnych kafli + gigantyczna twarz.
pub fn draw_menu_scene(assets: &Assets, time: f32) {
    clear_background(Color::new(0.015, 0.006, 0.010, 1.0));

    // tlo z kafli
    let mut y = 0.0;
    let mut row = 0;
    while y < VIEW_H {
        let mut x = 0.0;
        let mut col = 0;
        while x < VIEW_W {
            let idx = (row * 3 + col) % assets.walls.len();
            let tint = 0.20 + hash2(col as i32, row as i32, 9) * 0.10;
            draw_texture_ex(
                &assets.walls[idx],
                x,
                y,
                Color::new(tint * 1.2, tint * 0.85, tint * 0.75, 1.0),
                DrawTextureParams {
                    dest_size: Some(vec2(48.0, 48.0)),
                    ..Default::default()
                },
            );
            x += 48.0;
            col += 1;
        }
        y += 48.0;
        row += 1;
    }

    // przyciemnienie
    draw_rectangle(0.0, 0.0, VIEW_W, VIEW_H, Color::new(0.01, 0.0, 0.005, 0.55));

    // gigantyczna twarz
    let pulse = 1.0 + (time * 0.7).sin() * 0.025;
    let alpha = 0.26 + (time * 1.3).sin() * 0.04;
    let size = 280.0 * pulse;
    draw_texture_ex(
        &assets.face,
        VIEW_W * 0.5 - size * 0.5,
        VIEW_H * 0.46 - size * 0.5,
        Color::new(0.9, 0.78, 0.68, alpha),
        DrawTextureParams {
            dest_size: Some(vec2(size, size)),
            ..Default::default()
        },
    );

    // oczy w ciemnosci
    for i in 0..16 {
        let h1 = hash2(i, 3, 77);
        let h2 = hash2(i, 7, 91);
        let x = 20.0 + h1 * (VIEW_W - 40.0);
        let y = 20.0 + h2 * (VIEW_H - 40.0);
        let blink = if ((time * 0.8 + i as f32 * 1.7).sin()) > -0.92 {
            1.0
        } else {
            0.0
        };
        if blink > 0.0 {
            draw_circle(x, y, 6.0, Color::new(0.45, 0.01, 0.01, 0.10));
            draw_circle(x - 3.0, y, 1.6, Color::new(0.9, 0.05, 0.03, 0.75));
            draw_circle(x + 3.0, y, 1.6, Color::new(0.9, 0.05, 0.03, 0.75));
        }
    }

    // czerwona mgla przy dolnej krawedzi
    for i in 0..12 {
        let a = 0.05 - i as f32 * 0.003;
        draw_rectangle(
            0.0,
            VIEW_H - 70.0 + i as f32 * 6.0,
            VIEW_W,
            8.0,
            Color::new(0.22, 0.0, 0.01, a.max(0.0)),
        );
    }
}

pub fn glitch_text(text: &str, x: f32, y: f32, size: f32, color: Color, time: f32, strength: f32) {
    let jitter = (time * 17.0).sin() * strength;
    draw_text_ex(
        text,
        x - strength * 1.6 + jitter,
        y,
        TextParams {
            font_size: size as u16,
            color: Color::new(0.75, 0.02, 0.03, color.a * 0.55),
            ..Default::default()
        },
    );
    draw_text_ex(
        text,
        x + strength * 1.3 - jitter,
        y + 0.6,
        TextParams {
            font_size: size as u16,
            color: Color::new(0.02, 0.48, 0.52, color.a * 0.45),
            ..Default::default()
        },
    );
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font_size: size as u16,
            color,
            ..Default::default()
        },
    );
}

pub fn glitch_text_centered(
    text: &str,
    cx: f32,
    cy: f32,
    size: f32,
    color: Color,
    time: f32,
    strength: f32,
) {
    let dims = measure_text(text, None, size as u16, 1.0);
    glitch_text(text, cx - dims.width * 0.5, cy, size, color, time, strength);
}
