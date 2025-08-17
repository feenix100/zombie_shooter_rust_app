use macroquad::prelude::*;

// -------- 2D Zombie Shooter (Rust + macroquad) --------
// Controls: WASD move • Mouse aim • LMB shoot • R restart
// ------------------------------------------------------

const SCREEN_W: f32 = 960.0;
const SCREEN_H: f32 = 540.0;

const PLAYER_RADIUS: f32 = 14.0;
const ZOMBIE_RADIUS: f32 = 18.0;
const BULLET_RADIUS: f32 = 4.0;

const PLAYER_SPEED: f32 = 240.0;
const BULLET_SPEED: f32 = 520.0;

fn conf() -> Conf {
    Conf {
        window_title: "Zombie Shooter 2D (Rust + macroquad)".to_owned(),
        window_width: SCREEN_W as i32,
        window_height: SCREEN_H as i32,
        sample_count: 4,
        ..Default::default()
    }
}

#[derive(Clone, Copy)]
struct Circle { pos: Vec2, r: f32 }

struct Player { body: Circle, hp: i32 }
struct Bullet { body: Circle, vel: Vec2 }
struct Zombie { body: Circle, speed: f32 }

#[derive(PartialEq, Eq)]
enum State { Running, GameOver }

#[macroquad::main(conf)]
async fn main() {
    let mut player = Player { body: Circle { pos: vec2(SCREEN_W*0.5, SCREEN_H*0.5), r: PLAYER_RADIUS }, hp: 5 };
    let mut bullets: Vec<Bullet> = vec![];
    let mut zombies: Vec<Zombie> = vec![];

    let mut state = State::Running;
    let mut score: u32 = 0;

    let mut elapsed = 0.0f32;
    let mut spawn_timer = 0.0f32;
    let mut shoot_cd = 0.0f32;

    loop {
        let dt = get_frame_time();
        elapsed += dt; spawn_timer += dt; shoot_cd -= dt;

        clear_background(Color::from_rgba(18, 20, 26, 255));

        match state {
            State::Running => {
                // spawn
                let spawn_interval = (0.25f32).max(1.2 - elapsed * 0.02);
                let zombie_speed = (60.0 + elapsed * 3.0).min(180.0);
                if spawn_timer >= spawn_interval {
                    spawn_timer = 0.0; zombies.push(spawn_zombie(zombie_speed));
                }

                // movement
                let mut dir = Vec2::ZERO;
                if is_key_down(KeyCode::W) { dir.y -= 1.0; }
                if is_key_down(KeyCode::S) { dir.y += 1.0; }
                if is_key_down(KeyCode::A) { dir.x -= 1.0; }
                if is_key_down(KeyCode::D) { dir.x += 1.0; }
                if dir.length_squared() > 0.0 { dir = dir.normalize(); }
                player.body.pos += dir * PLAYER_SPEED * dt;
                clamp_to_bounds(&mut player.body.pos, player.body.r);

                // shooting
                if is_mouse_button_down(MouseButton::Left) && shoot_cd <= 0.0 {
                    shoot_cd = 0.12;
                    let (mx, my) = mouse_position();
                    let aim = vec2(mx, my) - player.body.pos;
                    let vel = if aim.length_squared() > 0.0 { aim.normalize() * BULLET_SPEED } else { Vec2::ZERO };
                    bullets.push(Bullet { body: Circle { pos: player.body.pos, r: BULLET_RADIUS }, vel });
                }

                // bullets
                for b in &mut bullets { b.body.pos += b.vel * dt; }
                bullets.retain(|b| in_bounds(b.body.pos, 64.0));

                // zombies toward player
                for z in &mut zombies {
                    let to_p = player.body.pos - z.body.pos;
                    if to_p.length_squared() > 0.01 { z.body.pos += to_p.normalize() * z.speed * dt; }
                }

                // bullet ↔ zombie
                let mut i = 0usize;
                while i < zombies.len() {
                    let mut hit = false;
                    let zr = zombies[i].body.r; let zpos = zombies[i].body.pos;
                    let mut j = 0usize;
                    while j < bullets.len() {
                        if circles_overlap(zpos, zr, bullets[j].body.pos, bullets[j].body.r) {
                            bullets.swap_remove(j); hit = true; score += 1; break;
                        } else { j += 1; }
                    }
                    if hit { zombies.swap_remove(i); } else { i += 1; }
                }

                // zombie ↔ player
                let mut k = 0usize;
                while k < zombies.len() {
                    if circles_overlap(zombies[k].body.pos, zombies[k].body.r, player.body.pos, player.body.r) {
                        zombies.swap_remove(k); player.hp -= 1; if player.hp <= 0 { state = State::GameOver; }
                    } else { k += 1; }
                }

                // draw
                draw_world(&player, &bullets, &zombies, score, player.hp, false);
            }
            State::GameOver => {
                draw_world(&player, &bullets, &zombies, score, player.hp.max(0), true);
                let msg = "GAME OVER — Press R to restart";
                let sz = 42.0; let tw = measure_text(msg, None, sz as u16, 1.0);
                draw_text(msg, SCREEN_W*0.5 - tw.width*0.5, SCREEN_H*0.5, sz, RED);
                if is_key_pressed(KeyCode::R) {
                    player = Player { body: Circle { pos: vec2(SCREEN_W*0.5, SCREEN_H*0.5), r: PLAYER_RADIUS }, hp: 5 };
                    bullets.clear(); zombies.clear(); score = 0; elapsed = 0.0; spawn_timer = 0.0; shoot_cd = 0.0; state = State::Running;
                }
            }
        }

        next_frame().await;
    }
}

fn draw_world(player: &Player, bullets: &[Bullet], zombies: &[Zombie], score: u32, hp: i32, paused: bool) {
    // ground
    draw_rectangle(0.0, 0.0, SCREEN_W, SCREEN_H, Color::from_rgba(26, 28, 36, 255));

    // player
    draw_circle(player.body.pos.x, player.body.pos.y, player.body.r + 3.0, Color::from_rgba(40, 40, 52, 255));
    draw_circle(player.body.pos.x, player.body.pos.y, player.body.r, SKYBLUE);

    // gun line to cursor
    let (mx, my) = mouse_position();
    let aim = (vec2(mx, my) - player.body.pos).clamp_length_max(24.0);
    draw_line(player.body.pos.x, player.body.pos.y, player.body.pos.x + aim.x, player.body.pos.y + aim.y, 3.0, WHITE);

    // bullets
    for b in bullets { draw_circle(b.body.pos.x, b.body.pos.y, b.body.r, YELLOW); }

    // zombies
    for z in zombies {
        draw_circle(z.body.pos.x, z.body.pos.y, z.body.r + 2.0, Color::from_rgba(46, 14, 14, 255));
        draw_circle(z.body.pos.x, z.body.pos.y, z.body.r, Color::from_rgba(40, 180, 90, 255));
    }

    // HUD
    let hud = format!("Score: {}    HP: {}{}", score, hp.max(0), if paused { "    [Paused]" } else { "" });
    draw_text(&hud, 16.0, 28.0, 28.0, WHITE);

    // instructions
    let info = "WASD to move  •  Mouse to aim  •  LMB to shoot  •  R to restart";
    let t = measure_text(info, None, 20, 1.0);
    draw_text(info, SCREEN_W * 0.5 - t.width * 0.5, SCREEN_H - 18.0, 20.0, GRAY);

    // crosshair
    draw_circle_lines(mx, my, 10.0, 1.5, LIGHTGRAY);
    draw_line(mx - 14.0, my, mx + 14.0, my, 1.0, LIGHTGRAY);
    draw_line(mx, my - 14.0, mx, my + 14.0, 1.0, LIGHTGRAY);
}

fn spawn_zombie(speed: f32) -> Zombie {
    let side = rand::gen_range(0, 4); // 0 top, 1 right, 2 bottom, 3 left
    let m = 30.0;
    let pos = match side {
        0 => vec2(rand::gen_range(0.0, SCREEN_W), -m),
        1 => vec2(SCREEN_W + m, rand::gen_range(0.0, SCREEN_H)),
        2 => vec2(rand::gen_range(0.0, SCREEN_W), SCREEN_H + m),
        _ => vec2(-m, rand::gen_range(0.0, SCREEN_H)),
    };
    Zombie { body: Circle { pos, r: ZOMBIE_RADIUS }, speed }
}

fn circles_overlap(a: Vec2, ar: f32, b: Vec2, br: f32) -> bool { (a - b).length_squared() <= (ar + br) * (ar + br) }
fn in_bounds(p: Vec2, pad: f32) -> bool { p.x >= -pad && p.y >= -pad && p.x <= SCREEN_W + pad && p.y <= SCREEN_H + pad }
fn clamp_to_bounds(p: &mut Vec2, r: f32) { p.x = p.x.clamp(r, SCREEN_W - r); p.y = p.y.clamp(r, SCREEN_H - r); }