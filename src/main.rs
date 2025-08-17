use macroquad::prelude::*;

// ---- Zombie Shooter 3D (Waves + Powerups) ----
// Controls: WASD move, Shift sprint, LMB shoot, R restart
// Camera: simple chase cam. Zombies have types & waves scale difficulty.
// Powerups: Heal, Bomb, RapidFire, Slow.
// -----------------------------------------------

const PLAYER_BASE_SPEED: f32 = 6.0;
const PLAYER_SPRINT_SPEED: f32 = 9.5;
const PLAYER_EYE: f32 = 0.8; // for drawing gun offset
const PLAYER_MAX_HP: i32 = 6;

const BULLET_SPEED: f32 = 28.0;
const BULLET_RADIUS: f32 = 0.18;
const BULLET_COOLDOWN_BASE: f32 = 0.20;

const ZOMBIE_RADIUS: f32 = 0.55;
const FAST_ZOMBIE_RADIUS: f32 = 0.45;
const TANK_ZOMBIE_RADIUS: f32 = 0.7;

const ARENA_HALF: f32 = 24.0; // -24..+24 square

#[derive(Clone, Copy)]
struct Player { pos: Vec3, hp: i32, cd: f32, rapid_left: f32, slow_left: f32 }

#[derive(Clone, Copy)]
struct Bullet { pos: Vec3, vel: Vec3, ttl: f32 }

#[derive(Clone, Copy)]
enum ZKind { Normal, Fast, Tank }

#[derive(Clone, Copy)]
struct Zombie { pos: Vec3, speed: f32, kind: ZKind, hp: i32 }

#[derive(Clone, Copy)]
enum Drop { Heal, Bomb, Rapid, Slow }

#[derive(Clone, Copy)]
struct Powerup { pos: Vec3, kind: Drop, ttl: f32 }

#[derive(PartialEq, Eq)]
enum State { Running, InterWave, GameOver }

#[macroquad::main("Zombie Shooter 3D — Waves & Powerups")]
async fn main() {
    let mut player = Player { pos: vec3(0.0, 0.5, 0.0), hp: PLAYER_MAX_HP, cd: 0.0, rapid_left: 0.0, slow_left: 0.0 };
    let mut bullets: Vec<Bullet> = vec![];
    let mut zombies: Vec<Zombie> = vec![];
    let mut drops: Vec<Powerup> = vec![];

    let mut score: u32 = 0;
    let mut combo: f32 = 1.0; let mut combo_timer: f32 = 0.0;
    let mut wave: u32 = 1; let mut state = State::Running; let mut inter_timer = 0.0;
    let mut spawn_budget_left = wave_spawn_budget(wave);

    // camera shake
    let mut shake: f32 = 0.0;

    // place some cover boxes
    let mut covers: Vec<Vec3> = vec![];
    for i in -2..=2 { covers.push(vec3(i as f32 * 6.5, 0.6, 8.0)); }
    for i in -1..=1 { covers.push(vec3(-12.0, 0.6, i as f32 * 6.0)); covers.push(vec3(12.0, 0.6, i as f32 * 6.0)); }

    loop {
        let dt = get_frame_time();
        if dt <= 0.0 { next_frame().await; continue; }

        // timers
        player.cd -= dt; player.rapid_left = (player.rapid_left - dt).max(0.0); player.slow_left = (player.slow_left - dt).max(0.0);
        shake = (shake - dt * 4.0).max(0.0);
        combo_timer = (combo_timer - dt).max(0.0); if combo_timer == 0.0 { combo = 1.0; }

        clear_background(Color::from_rgba(12, 14, 20, 255));

        match state {
            State::Running => {
                // spawn logic while running this wave
                let max_alive = 10 + wave as usize * 2;
                if zombies.len() < max_alive && spawn_budget_left > 0 {
                    let batch = (1 + (wave as usize / 2)).min(spawn_budget_left);
                    for _ in 0..batch { zombies.push(spawn_zombie(wave, player.pos)); }
                    spawn_budget_left -= batch;
                }

                // input movement
                let mut dir = Vec3::ZERO;
                if is_key_down(KeyCode::W) { dir.z -= 1.0; }
                if is_key_down(KeyCode::S) { dir.z += 1.0; }
                if is_key_down(KeyCode::A) { dir.x -= 1.0; }
                if is_key_down(KeyCode::D) { dir.x += 1.0; }
                if dir.length_squared() > 0.0 { dir = dir.normalize(); }
                let speed = if is_key_down(KeyCode::LeftShift) { PLAYER_SPRINT_SPEED } else { PLAYER_BASE_SPEED };
                player.pos += dir * speed * dt;
                // keep inside arena, avoid overlapping cover a bit
                player.pos.x = player.pos.x.clamp(-ARENA_HALF + 1.0, ARENA_HALF - 1.0);
                player.pos.z = player.pos.z.clamp(-ARENA_HALF + 1.0, ARENA_HALF - 1.0);

                // shooting (forward along -Z for simplicity)
                let cd = if player.rapid_left > 0.0 { BULLET_COOLDOWN_BASE * 0.45 } else { BULLET_COOLDOWN_BASE };
                if is_mouse_button_down(MouseButton::Left) && player.cd <= 0.0 {
                    player.cd = cd;
                    let dirz = vec3(0.0, 0.0, -1.0); // simple forward shot
                    bullets.push(Bullet { pos: player.pos + vec3(0.0, PLAYER_EYE, 0.0) + dirz * 0.8, vel: dirz * BULLET_SPEED, ttl: 1.8 });
                    shake = (shake + 0.25).min(1.0);
                }

                // bullets advance
                for b in &mut bullets { b.pos += b.vel * dt; b.ttl -= dt; }
                bullets.retain(|b| b.ttl > 0.0 && in_bounds(b.pos));

                // zombies seek player, slowed if slow_power active
                let slow_factor = if player.slow_left > 0.0 { 0.55 } else { 1.0 };
                for z in &mut zombies {
                    let to_p = (player.pos - z.pos).with_y(0.0);
                    if to_p.length_squared() > 0.0004 { z.pos += to_p.normalize() * z.speed * slow_factor * dt; }
                    // simple arena clamp
                    z.pos.x = z.pos.x.clamp(-ARENA_HALF, ARENA_HALF); z.pos.z = z.pos.z.clamp(-ARENA_HALF, ARENA_HALF);
                }

                // bullet ↔ zombie
                let mut zi = 0usize;
                while zi < zombies.len() {
                    let mut dead = false;
                    let mut bj = 0usize;
                    while bj < bullets.len() {
                        let rad = match zombies[zi].kind { ZKind::Tank => TANK_ZOMBIE_RADIUS, ZKind::Fast => FAST_ZOMBIE_RADIUS, _ => ZOMBIE_RADIUS };
                        if (zombies[zi].pos - bullets[bj].pos).length() <= rad + BULLET_RADIUS {
                            bullets.swap_remove(bj);
                            zombies[zi].hp -= 1;
                            if zombies[zi].hp <= 0 { dead = true; }
                            break;
                        } else { bj += 1; }
                    }
                    if dead {
                        // score + combo, and chance to drop
                        score += (10.0 * combo).round() as u32;
                        combo = (combo + 0.25).min(4.0); combo_timer = 2.0;
                        maybe_drop(&mut drops, zombies[zi].pos);
                        zombies.swap_remove(zi);
                    } else { zi += 1; }
                }

                // zombie ↔ player
                let mut k = 0usize;
                while k < zombies.len() {
                    let rad = match zombies[k].kind { ZKind::Tank => TANK_ZOMBIE_RADIUS, ZKind::Fast => FAST_ZOMBIE_RADIUS, _ => ZOMBIE_RADIUS };
                    if (zombies[k].pos - player.pos).length() <= rad + 0.5 {
                        zombies.swap_remove(k);
                        player.hp -= 1; shake = (shake + 0.6).min(1.4);
                        if player.hp <= 0 { state = State::GameOver; }
                    } else { k += 1; }
                }

                // powerup pickups
                let mut di = 0usize;
                while di < drops.len() {
                    drops[di].ttl -= dt; if drops[di].ttl <= 0.0 { drops.swap_remove(di); continue; }
                    if (drops[di].pos - player.pos).length() < 1.0 {
                        apply_powerup(&mut player, drops[di].kind, &mut zombies, &mut score);
                        drops.swap_remove(di);
                    } else { di += 1; }
                }

                // wave cleared?
                if zombies.is_empty() && spawn_budget_left == 0 { state = State::InterWave; inter_timer = 2.0; }

                // render world
                render_world(&player, &bullets, &zombies, &drops, &covers, shake);
                draw_hud(score, player.hp, wave, combo, false);
            }
            State::InterWave => {
                inter_timer -= dt;
                render_world(&player, &bullets, &zombies, &drops, &covers, shake);
                draw_hud(score, player.hp, wave, combo, true);
                let msg = format!("Wave {} cleared! Next in {:.1}s", wave, inter_timer.max(0.0));
                let tw = measure_text(&msg, None, 36, 1.0);
                draw_text(&msg, screen_width()*0.5 - tw.width*0.5, screen_height()*0.5, 36.0, YELLOW);
                if inter_timer <= 0.0 {
                    wave += 1; spawn_budget_left = wave_spawn_budget(wave); state = State::Running;
                    // small heal each wave
                    player.hp = (player.hp + 1).min(PLAYER_MAX_HP);
                }
            }
            State::GameOver => {
                set_default_camera();
                let msg = "GAME OVER — Press R to restart";
                let tw = measure_text(msg, None, 44, 1.0);
                draw_text(msg, screen_width()*0.5 - tw.width*0.5, screen_height()*0.45, 44.0, RED);
                draw_text(&format!("Final Score: {}", score), screen_width()*0.5 - 120.0, screen_height()*0.55, 28.0, WHITE);
                if is_key_pressed(KeyCode::R) { // reset
                    player = Player { pos: vec3(0.0, 0.5, 0.0), hp: PLAYER_MAX_HP, cd: 0.0, rapid_left: 0.0, slow_left: 0.0 };
                    bullets.clear(); zombies.clear(); drops.clear();
                    score = 0; combo = 1.0; combo_timer = 0.0; wave = 1; spawn_budget_left = wave_spawn_budget(wave); state = State::Running; shake = 0.0;
                }
            }
        }

        next_frame().await;
    }
}

fn render_world(player: &Player, bullets: &[Bullet], zombies: &[Zombie], drops: &[Powerup], covers: &[Vec3], shake: f32) {
    // camera: chase w/ shake
    let mut cam_pos = vec3(player.pos.x, 8.0, player.pos.z + 16.0);
    let jitter = vec3((rand::gen_range(-1.0, 1.0))*0.15*shake, (rand::gen_range(-1.0, 1.0))*0.10*shake, (rand::gen_range(-1.0, 1.0))*0.2*shake);
    cam_pos += jitter;
    set_camera(&Camera3D { position: cam_pos, target: player.pos, up: vec3(0.0,1.0,0.0), fovy: 45.0, ..Default::default() });

    // arena floor & bounds
    draw_grid(40, 1.0, Color::from_rgba(30,32,40,255), Color::from_rgba(58,62,74,255));
    // boundary walls (low)
    for i in 0..4 { let rot = i as f32 * std::f32::consts::FRAC_PI_2; let dir = vec3(rot.sin(), 0.0, rot.cos());
        let center = dir * ARENA_HALF; draw_cube(center + vec3(0.0, 0.5, 0.0), vec3(ARENA_HALF*2.0, 1.0, 0.6), None, Color::from_rgba(46,50,64,255)); }

    // covers
    for c in covers { draw_cube(*c, vec3(1.4, 1.2, 1.4), None, Color::from_rgba(64,66,86,255)); }

    // player
    draw_cube(player.pos, vec3(1.0, 1.0, 1.0), None, SKYBLUE);
    // bullets
    for b in bullets { draw_sphere(b.pos, BULLET_RADIUS, None, YELLOW); }
    // zombies
    for z in zombies {
        let (col, s) = match z.kind { ZKind::Normal => (Color::from_rgba(40,180,90,255), vec3(1.1, 1.5, 1.1)), ZKind::Fast => (Color::from_rgba(60,220,120,255), vec3(0.9, 1.2, 0.9)), ZKind::Tank => (Color::from_rgba(30,140,70,255), vec3(1.4, 1.9, 1.4)) };
        draw_cube(z.pos + vec3(0.0, 0.2, 0.0), s, None, col);
    }
    // powerups
    for d in drops { let col = match d.kind { Drop::Heal=>PINK, Drop::Bomb=>ORANGE, Drop::Rapid=>SKYBLUE, Drop::Slow=>VIOLET }; draw_sphere(d.pos + vec3(0.0,0.5,0.0), 0.35, None, col); }

    set_default_camera();
}

fn draw_hud(score: u32, hp: i32, wave: u32, combo: f32, paused: bool) {
    let hud = format!("Score: {}    HP: {}    Wave: {}    Combo: x{:.1}{}", score, hp.max(0), wave, combo, if paused { "  [Intermission]" } else { "" });
    draw_text(&hud, 16.0, 28.0, 28.0, WHITE);
    let info = "WASD move • Shift sprint • LMB shoot • R restart";
    let t = measure_text(info, None, 20, 1.0);
    draw_text(info, screen_width()*0.5 - t.width*0.5, screen_height() - 18.0, 20.0, GRAY);
}

fn spawn_zombie(wave: u32, center: Vec3) -> Zombie {
    let angle = rand::gen_range(0.0, 360.0f32).to_radians();
    let r = rand::gen_range(14.0, ARENA_HALF - 1.5);
    let pos = vec3(center.x + angle.sin()*r, 0.5, center.z + angle.cos()*r);
    // choose type weighted by wave
    let roll = rand::gen_range(0.0, 1.0);
    if roll < (0.15 + wave as f32 * 0.01).min(0.35) { // tank chance grows
        Zombie { pos, speed: 1.6, kind: ZKind::Tank, hp: 3 }
    } else if roll < 0.55 { // fast
        Zombie { pos, speed: 3.6 + wave as f32 * 0.05, kind: ZKind::Fast, hp: 1 }
    } else { // normal
        Zombie { pos, speed: 2.4 + wave as f32 * 0.03, kind: ZKind::Normal, hp: 2 }
    }
}

fn maybe_drop(out: &mut Vec<Powerup>, pos: Vec3) {
    let p = rand::gen_range(0.0, 1.0);
    if p < 0.22 {
        let kind = if p < 0.07 { Drop::Heal } else if p < 0.12 { Drop::Bomb } else if p < 0.18 { Drop::Rapid } else { Drop::Slow };
        out.push(Powerup { pos, kind, ttl: 12.0 });
    }
}

fn apply_powerup(player: &mut Player, kind: Drop, zombies: &mut Vec<Zombie>, score: &mut u32) {
    match kind {
        Drop::Heal => { player.hp = (player.hp + 2).min(PLAYER_MAX_HP); }
        Drop::Bomb => {
            let mut killed = 0u32; let radius = 4.2;
            let mut i=0; while i < zombies.len() { if (zombies[i].pos - player.pos).length() <= radius { zombies.swap_remove(i); killed+=1; } else { i+=1; } }
            *score += killed * 15;
        }
        Drop::Rapid => { player.rapid_left = 6.0; }
        Drop::Slow => { player.slow_left = 6.0; }
    }
}

fn in_bounds(p: Vec3) -> bool { p.x.abs() <= ARENA_HALF+2.0 && p.z.abs() <= ARENA_HALF+2.0 }

fn wave_spawn_budget(wave: u32) -> usize { (8 + (wave as usize)*5).min(120) }