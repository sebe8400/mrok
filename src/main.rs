//! MROK // ZAKLAD
//! Proceduralny horror top-down. Wszystko - tekstury, dzwiek, mapa - powstaje
//! w kodzie. Bez assetow, bez internetu, bez litosci.

mod assets;
mod fx;
mod game;
mod map;
mod rng;

use macroquad::audio::{play_sound, PlaySoundParams, Sound};
use macroquad::prelude::*;

use assets::Assets;
use fx::{glitch_text_centered, PostFx, PostParams, VIEW_H, VIEW_W};
use game::{Game, GameEvent};

const MENU_ITEMS: [&str; 3] = ["NOWA GRA", "JAK GRAC", "WYJDZ"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Playing,
    Paused,
    Dead,
    Won,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "MROK // ZAKLAD".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        window_resizable: true,
        sample_count: 1,
        fullscreen: false,
        platform: macroquad::miniquad::conf::Platform {
            swap_interval: Some(1),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let assets = Assets::load().await;
    let post = PostFx::new();
    let render = render_target(VIEW_W as u32, VIEW_H as u32);
    render.texture.set_filter(FilterMode::Nearest);

    let mut screen = Screen::Menu;
    let mut game: Option<Game> = None;
    let mut clock = 0.0f32;
    let mut menu_sel = 0usize;
    let mut show_help = false;
    let mut fullscreen = false;
    let mut runs = 0u64;

    // Tryb testowy: MROK_AUTOPLAY=1 pomija menu. Przydatne w CI i przy
    // automatycznych zrzutach ekranu.
    let autoplay = std::env::var("MROK_AUTOPLAY").is_ok();
    let debug_screen = std::env::var("MROK_SCREEN").ok();
    let debug_freeze = debug_screen.is_some();
    let shot_at: Option<f32> = std::env::var("MROK_SHOT_AT")
        .ok()
        .and_then(|v| v.parse().ok());
    let quit_after: Option<f32> = std::env::var("MROK_QUIT_AFTER")
        .ok()
        .and_then(|v| v.parse().ok());
    let mut shot_taken = false;

    if autoplay || debug_screen.is_some() {
        start_game(&mut game, &mut runs);
        screen = match debug_screen.as_deref() {
            Some("help") => Screen::Menu,
            Some("pause") => Screen::Paused,
            Some("dead") => Screen::Dead,
            Some("win") => Screen::Won,
            _ => Screen::Playing,
        };
        if let Some(g) = game.as_mut() {
            match debug_screen.as_deref() {
                Some("dead") => g.player.health = 0.0,
                Some("win") => {
                    let total = g.total_relics();
                    g.player.relics = total;
                    g.player.won = true;
                    g.time = 137.0;
                }
                _ => {}
            }
        }
        if screen == Screen::Menu {
            show_help = debug_screen.as_deref() == Some("help");
        }
        show_mouse(screen != Screen::Playing);
    } else {
        show_mouse(true);
    }

    if let Some(ambient) = &assets.ambient {
        play_sound(
            ambient,
            PlaySoundParams {
                looped: true,
                volume: 0.38,
            },
        );
    }

    loop {
        let dt = get_frame_time().min(0.05);
        clock += dt;

        if is_key_pressed(KeyCode::F11) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }

        match screen {
            Screen::Menu => {
                if !debug_freeze {
                    update_menu(
                        &mut screen,
                        &mut game,
                        &mut menu_sel,
                        &mut show_help,
                        &mut runs,
                        &assets,
                    );
                }
                draw_menu(&assets, &post, &render, clock, menu_sel, show_help);
            }
            Screen::Playing => {
                if is_key_pressed(KeyCode::Escape) {
                    screen = Screen::Paused;
                    show_mouse(true);
                } else if let Some(g) = game.as_mut() {
                    let (mx, my) = mouse_position();
                    let sw = screen_width().max(1.0);
                    let sh = screen_height().max(1.0);
                    let mouse_virtual = vec2(mx / sw * VIEW_W, my / sh * VIEW_H);
                    g.update(dt, mouse_virtual);
                    let events = g.drain_events();
                    play_events(events, &assets);
                    if g.player.health <= 0.0 {
                        screen = Screen::Dead;
                        show_mouse(true);
                        play_opt(&assets.death, 0.9);
                    } else if g.player.won {
                        screen = Screen::Won;
                        show_mouse(true);
                        play_opt(&assets.win, 0.85);
                    }
                } else {
                    screen = Screen::Menu;
                    show_mouse(true);
                }
                draw_game(&assets, &post, &render, game.as_ref(), clock);
            }
            Screen::Paused => {
                if !debug_freeze {
                    if is_key_pressed(KeyCode::Escape) {
                        screen = Screen::Playing;
                        show_mouse(false);
                    } else if is_key_pressed(KeyCode::R) {
                        start_game(&mut game, &mut runs);
                        screen = Screen::Playing;
                        show_mouse(false);
                    } else if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::M) {
                        screen = Screen::Menu;
                        menu_sel = 0;
                        show_help = false;
                        show_mouse(true);
                    }
                }
                draw_pause(&assets, &post, &render, game.as_ref(), clock);
            }
            Screen::Dead | Screen::Won => {
                if !debug_freeze {
                    if is_key_pressed(KeyCode::R) {
                        start_game(&mut game, &mut runs);
                        screen = Screen::Playing;
                        show_mouse(false);
                    } else if is_key_pressed(KeyCode::Escape) {
                        screen = Screen::Menu;
                        menu_sel = 0;
                        show_help = false;
                        show_mouse(true);
                    }
                }
                draw_end(
                    &assets,
                    &post,
                    &render,
                    game.as_ref(),
                    clock,
                    screen == Screen::Won,
                );
            }
        }

        if is_key_pressed(KeyCode::F12) {
            let image = get_screen_data();
            let name = format!("mrok_{}.png", (get_time() * 1000.0) as u64);
            image.export_png(&name);
            println!("[MROK] zrzut ekranu: {name}");
        }
        if !shot_taken && shot_at.is_some_and(|t| clock >= t) {
            let image = get_screen_data();
            image.export_png("mrok_auto.png");
            println!("[MROK] zrzut ekranu: mrok_auto.png");
            shot_taken = true;
        }
        if quit_after.is_some_and(|t| clock >= t) {
            std::process::exit(0);
        }

        next_frame().await;
    }
}

// ---------------------------------------------------------------------------
// MENU
// ---------------------------------------------------------------------------

fn update_menu(
    screen: &mut Screen,
    game: &mut Option<Game>,
    sel: &mut usize,
    show_help: &mut bool,
    runs: &mut u64,
    assets: &Assets,
) {
    if *show_help {
        if is_key_pressed(KeyCode::Escape)
            || is_key_pressed(KeyCode::Enter)
            || is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::H)
            || is_mouse_button_pressed(MouseButton::Left)
        {
            *show_help = false;
        }
        return;
    }

    if is_key_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }

    let last = MENU_ITEMS.len() - 1;
    if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
        *sel = if *sel == 0 { last } else { *sel - 1 };
        play_opt(&assets.step, 0.20);
    }
    if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
        *sel = if *sel == last { 0 } else { *sel + 1 };
        play_opt(&assets.step, 0.20);
    }

    let (mx, my) = mouse_position();
    for i in 0..MENU_ITEMS.len() {
        if rect_hit(menu_item_rect(i), mx, my) {
            *sel = i;
        }
    }

    let mut activate = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space);
    if is_mouse_button_pressed(MouseButton::Left) {
        for i in 0..MENU_ITEMS.len() {
            if rect_hit(menu_item_rect(i), mx, my) {
                *sel = i;
                activate = true;
            }
        }
    }

    if activate {
        match *sel {
            0 => {
                start_game(game, runs);
                *screen = Screen::Playing;
                show_mouse(false);
                play_opt(&assets.whisper, 0.45);
            }
            1 => {
                *show_help = true;
            }
            _ => std::process::exit(0),
        }
    }
}

fn draw_menu(
    assets: &Assets,
    post: &PostFx,
    render: &RenderTarget,
    clock: f32,
    sel: usize,
    show_help: bool,
) {
    let sw = screen_width();
    let sh = screen_height();
    let s = ui_scale();

    set_virtual_camera(vec2(VIEW_W * 0.5, VIEW_H * 0.5), render);
    fx::draw_menu_scene(assets, clock);
    set_default_camera();

    let params = PostParams {
        time: clock,
        sanity: 0.42,
        fear: 0.50,
        flash: 0.0,
        player: vec2(0.5, 0.60),
        light_dir: vec2(0.0, -1.0),
        light_on: 0.75,
        damage: 0.0,
        ambient: 0.03,
    };
    fx::present(&render.texture, post, &params);

    let flicker = 0.78 + 0.22 * ((clock * 8.7).sin() * 0.5 + 0.5);
    glitch_text_centered(
        "MROK",
        sw * 0.5,
        238.0 * s,
        126.0 * s,
        Color::new(0.88, 0.09, 0.07, flicker),
        clock,
        4.2 * s,
    );
    glitch_text_centered(
        "// ZAKLAD",
        sw * 0.5,
        286.0 * s,
        32.0 * s,
        Color::new(0.80, 0.72, 0.55, 0.90),
        clock,
        2.0 * s,
    );

    // cienka czerwona linia pod tytulem
    let line_w = 190.0 * s;
    draw_rectangle(
        sw * 0.5 - line_w * 0.5,
        300.0 * s,
        line_w,
        1.5 * s,
        Color::new(0.52, 0.03, 0.03, 0.65),
    );

    draw_text_ex(
        "PROCEDURALNY HORROR",
        sw * 0.5 - measure_text("PROCEDURALNY HORROR", None, (15.0 * s) as u16, 1.0).width * 0.5,
        328.0 * s,
        TextParams {
            font_size: (15.0 * s) as u16,
            color: Color::new(0.58, 0.52, 0.44, 0.70),
            ..Default::default()
        },
    );

    for (i, label) in MENU_ITEMS.iter().enumerate() {
        let r = menu_item_rect(i);
        let selected = i == sel;
        if selected {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.20, 0.008, 0.015, 0.62));
            draw_rectangle_lines(
                r.x,
                r.y,
                r.w,
                r.h,
                1.4 * s,
                Color::new(0.70, 0.10, 0.08, 0.55),
            );
            let cy = r.y + r.h * 0.5;
            draw_triangle(
                vec2(r.x + 16.0 * s, cy - 6.0 * s),
                vec2(r.x + 16.0 * s, cy + 6.0 * s),
                vec2(r.x + 24.0 * s, cy),
                Color::new(0.95, 0.55, 0.16, 0.9),
            );
            draw_triangle(
                vec2(r.x + r.w - 16.0 * s, cy - 6.0 * s),
                vec2(r.x + r.w - 16.0 * s, cy + 6.0 * s),
                vec2(r.x + r.w - 24.0 * s, cy),
                Color::new(0.95, 0.55, 0.16, 0.9),
            );
        }
        let col = if selected {
            Color::new(1.0, 0.78, 0.40, 0.98)
        } else {
            Color::new(0.62, 0.57, 0.49, 0.82)
        };
        glitch_text_centered(
            label,
            sw * 0.5,
            r.y + r.h * 0.62,
            28.0 * s,
            col,
            clock,
            if selected { 1.4 * s } else { 0.4 * s },
        );
    }

    draw_text_ex(
        "WASD RUCH   //   SHIFT BIEG   //   E AKCJA   //   F LATARKA   //   ESC PAUZA",
        sw * 0.5
            - measure_text(
                "WASD RUCH   //   SHIFT BIEG   //   E AKCJA   //   F LATARKA   //   ESC PAUZA",
                None,
                (14.0 * s) as u16,
                1.0,
            )
            .width
                * 0.5,
        sh - 28.0 * s,
        TextParams {
            font_size: (14.0 * s) as u16,
            color: Color::new(0.48, 0.44, 0.38, 0.70),
            ..Default::default()
        },
    );
    draw_text_ex(
        "F11 PELNY EKRAN   //   F12 ZRZUT EKRANU",
        sw * 0.5
            - measure_text(
                "F11 PELNY EKRAN   //   F12 ZRZUT EKRANU",
                None,
                (11.0 * s) as u16,
                1.0,
            )
            .width
                * 0.5,
        sh - 11.0 * s,
        TextParams {
            font_size: (11.0 * s) as u16,
            color: Color::new(0.38, 0.35, 0.31, 0.60),
            ..Default::default()
        },
    );

    draw_text_ex(
        "v0.1",
        sw - 52.0 * s,
        30.0 * s,
        TextParams {
            font_size: (12.0 * s) as u16,
            color: Color::new(0.40, 0.36, 0.32, 0.55),
            ..Default::default()
        },
    );

    if show_help {
        draw_help_overlay(assets, clock);
    }
}

fn draw_help_overlay(assets: &Assets, clock: f32) {
    let sw = screen_width();
    let sh = screen_height();
    let s = ui_scale();
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.006, 0.002, 0.005, 0.97));
    draw_texture_ex(
        &assets.face,
        sw * 0.5 - 160.0 * s,
        sh * 0.5 - 150.0 * s,
        Color::new(0.7, 0.5, 0.45, 0.04),
        DrawTextureParams {
            dest_size: Some(vec2(320.0 * s, 320.0 * s)),
            ..Default::default()
        },
    );

    glitch_text_centered(
        "JAK GRAC",
        sw * 0.5,
        92.0 * s,
        54.0 * s,
        Color::new(0.88, 0.55, 0.18, 0.96),
        clock,
        2.0 * s,
    );

    let keys = ["WASD / STRZALKI", "SHIFT", "F", "E", "ESC", "F11", "F12"];
    let descs = [
        "RUCH",
        "BIEG (ZUZYWA ODDECH)",
        "LATARKA ON / OFF",
        "ZABIERZ RELIKWIE / UZYJ WYJSCIA",
        "PAUZA",
        "PELNY EKRAN",
        "ZRZUT EKRANU",
    ];

    let key_right = sw * 0.5 - 30.0 * s;
    let desc_x = sw * 0.5 + 24.0 * s;
    let mut y = 178.0 * s;
    for (key, desc) in keys.iter().zip(descs.iter()) {
        let kw = measure_text(key, None, (19.0 * s) as u16, 1.0).width;
        draw_text_ex(
            key,
            key_right - kw,
            y,
            TextParams {
                font_size: (19.0 * s) as u16,
                color: Color::new(0.92, 0.70, 0.34, 0.95),
                ..Default::default()
            },
        );
        draw_text_ex(
            desc,
            desc_x,
            y,
            TextParams {
                font_size: (19.0 * s) as u16,
                color: Color::new(0.70, 0.65, 0.56, 0.92),
                ..Default::default()
            },
        );
        y += 34.0 * s;
    }

    y += 18.0 * s;
    let paragraphs = [
        "ZBIERZ 5 RELIKWII. WROC DO WYJSCIA.",
        "NIE POZWOL, ZEBY PAMIEC SPADLA DO ZERA.",
        "ONE WIDZA CIAGLE. TY WIDZISZ TYLKO STOZEK.",
    ];
    for line in paragraphs {
        let size = (16.0 * s) as u16;
        let w = measure_text(line, None, size, 1.0).width;
        draw_text_ex(
            line,
            sw * 0.5 - w * 0.5,
            y,
            TextParams {
                font_size: size,
                color: Color::new(0.76, 0.66, 0.50, 0.92),
                ..Default::default()
            },
        );
        y += 30.0 * s;
    }

    glitch_text_centered(
        "ESC / ENTER   WROC",
        sw * 0.5,
        sh - 44.0 * s,
        18.0 * s,
        Color::new(0.88, 0.22, 0.12, 0.90),
        clock,
        0.6 * s,
    );
}

// ---------------------------------------------------------------------------
// GRA
// ---------------------------------------------------------------------------

fn draw_game(
    assets: &Assets,
    post: &PostFx,
    render: &RenderTarget,
    game: Option<&Game>,
    clock: f32,
) {
    let Some(g) = game else {
        clear_background(BLACK);
        return;
    };
    set_virtual_camera(g.camera, render);
    g.draw_world(assets);
    set_default_camera();
    fx::present(&render.texture, post, &g.post_params(clock));
    g.draw_hud(assets);
}

fn draw_pause(
    assets: &Assets,
    post: &PostFx,
    render: &RenderTarget,
    game: Option<&Game>,
    clock: f32,
) {
    let sw = screen_width();
    let sh = screen_height();
    let s = ui_scale();

    if let Some(g) = game {
        set_virtual_camera(g.camera, render);
        g.draw_world(assets);
        set_default_camera();
        let mut pp = g.post_params(clock);
        pp.sanity *= 0.55;
        pp.fear = (pp.fear + 0.20).min(1.0);
        fx::present(&render.texture, post, &pp);
    } else {
        clear_background(BLACK);
    }

    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.006, 0.001, 0.005, 0.74));
    glitch_text_centered(
        "PAUZA",
        sw * 0.5,
        sh * 0.36,
        74.0 * s,
        Color::new(0.90, 0.14, 0.09, 0.98),
        clock,
        1.8 * s,
    );

    let lines = ["ESC      WROC DO GRY", "R        NOWA GRA", "Q        MENU"];
    let mut y = sh * 0.48;
    for line in lines {
        glitch_text_centered(
            line,
            sw * 0.5,
            y,
            22.0 * s,
            Color::new(0.72, 0.66, 0.55, 0.90),
            clock,
            0.6 * s,
        );
        y += 38.0 * s;
    }

    if let Some(g) = game {
        let txt = format!(
            "RELIKWIE {}/{}   //   CZAS {}",
            g.player.relics,
            g.total_relics(),
            game::format_time(g.time)
        );
        glitch_text_centered(
            &txt,
            sw * 0.5,
            sh * 0.70,
            18.0 * s,
            Color::new(0.55, 0.50, 0.42, 0.75),
            clock,
            0.4 * s,
        );
    }

    draw_text_ex(
        "ONO NIE CZEKA. ALE TY MOZESZ.",
        sw * 0.5
            - measure_text(
                "ONO NIE CZEKA. ALE TY MOZESZ.",
                None,
                (14.0 * s) as u16,
                1.0,
            )
            .width
                * 0.5,
        sh - 34.0 * s,
        TextParams {
            font_size: (14.0 * s) as u16,
            color: Color::new(0.50, 0.03, 0.03, 0.65),
            ..Default::default()
        },
    );
}

fn draw_end(
    assets: &Assets,
    post: &PostFx,
    render: &RenderTarget,
    game: Option<&Game>,
    clock: f32,
    won: bool,
) {
    if let Some(g) = game {
        set_virtual_camera(g.camera, render);
        g.draw_world(assets);
        set_default_camera();
        let mut pp = g.post_params(clock);
        if won {
            pp.sanity = 1.0;
            pp.fear = 0.0;
            pp.damage = 0.0;
            pp.flash = 0.0;
            pp.ambient = 0.08;
        } else {
            pp.sanity = 0.0;
            pp.fear = 1.0;
            pp.damage = 0.55;
            pp.flash = 0.0;
        }
        fx::present(&render.texture, post, &pp);
        g.draw_end_screen(assets, won);
    } else {
        clear_background(BLACK);
    }
}

// ---------------------------------------------------------------------------
// POMOCNICZE
// ---------------------------------------------------------------------------

fn ui_scale() -> f32 {
    (screen_height() / 720.0).max(0.55)
}

fn menu_item_rect(i: usize) -> Rect {
    let s = ui_scale();
    let w = 390.0 * s;
    let h = 54.0 * s;
    Rect::new(
        screen_width() * 0.5 - w * 0.5,
        (386.0 + i as f32 * 68.0) * s,
        w,
        h,
    )
}

fn rect_hit(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && y >= r.y && x <= r.x + r.w && y <= r.y + r.h
}

fn set_virtual_camera(target: Vec2, render: &RenderTarget) {
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, VIEW_W, VIEW_H));
    cam.target = target;
    cam.render_target = Some(render.clone());
    set_camera(&cam);
}

fn start_game(game: &mut Option<Game>, runs: &mut u64) {
    *runs += 1;
    *game = Some(Game::new(new_seed(*runs)));
}

fn new_seed(run: u64) -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x1234_5678);
    nanos ^ run.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5_5A5A_DEAD_BEEF
}

fn play_opt(sound: &Option<Sound>, volume: f32) {
    if let Some(s) = sound {
        play_sound(
            s,
            PlaySoundParams {
                looped: false,
                volume: volume.clamp(0.0, 1.0),
            },
        );
    }
}

fn play_events(events: Vec<GameEvent>, assets: &Assets) {
    for event in events {
        match event {
            GameEvent::Pickup => play_opt(&assets.pickup, 0.85),
            GameEvent::Hit => play_opt(&assets.hit, 0.95),
            GameEvent::Heartbeat(v) => play_opt(&assets.heartbeat, 0.22 + v * 0.55),
            GameEvent::Whisper => play_opt(&assets.whisper, 0.55),
            GameEvent::Locked => play_opt(&assets.hit, 0.35),
            GameEvent::Step => play_opt(&assets.step, 0.22),
        }
    }
}
