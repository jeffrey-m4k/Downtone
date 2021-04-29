#![allow(dead_code)]

use std::path;
use std::env;
use std::f32::consts::PI;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler, KeyCode, KeyMods};
use ggez::graphics;
use ggez::graphics::{Text, TextFragment, Rect, Scale, DEFAULT_FONT_SCALE, Color};
use ggez::nalgebra::{Point2, Vector2};
use ggez::timer;
use ggez::audio;
use ggez::audio::SoundSource;
use ggez::input::keyboard;
use ggez::conf::WindowMode;
use fastrand;

mod level;

pub fn main() -> GameResult {
    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    let cb = ggez::ContextBuilder::new("gamething", "jeffrey-m").add_resource_path(resource_dir);
    let (ctx, event_loop) = &mut cb.build()?;
    graphics::set_default_filter(ctx, graphics::FilterMode::Nearest);
    graphics::set_window_title(ctx, "Downtone");
    graphics::set_mode(ctx, WindowMode::default().resizable(true))?;
    let state = &mut MainState::new(ctx)?;
    event::run(ctx, event_loop, state)
}

#[derive(PartialEq)]
enum GameState {
    Menu(MenuState),
    InGame,
    HaltScreen
}

#[derive(PartialEq)]
enum MenuState {
    Main,
    Options
}

pub struct MainState {
    state: GameState,
    paused: bool,
    spritebatch: graphics::spritebatch::SpriteBatch,
    music_source: audio::Source,
    font: graphics::Font,
    text_common: [Text; 4],
    player_stats: GameStats,
    player_pos: Vector2<f32>,
    player_vel: Vector2<f32>,
    player_facing: Facing,
    player_jump_time: u32,
    generator: level::Generator,
    level: level::Level,
    screen_size: Vector2<f32>,
    camera: CameraView
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {

        let mut atlas: graphics::Image = graphics::Image::new(ctx, "/atlas.png").expect("Could not load texture atlas!");
        atlas.set_filter(graphics::FilterMode::Nearest);
        let batch = graphics::spritebatch::SpriteBatch::new(atlas.clone());

        let mut music = audio::Source::new(ctx, "/audio/menu_loop.ogg")?;
        music.set_repeat(true);

        let font_emulogic =  graphics::Font::new(ctx, "/font/emulogic.ttf").expect("Could not load font!");

        graphics::set_default_filter(ctx, graphics::FilterMode::Nearest);
        let text_common: [_; 4] = [
            Text::new(TextFragment::new("PRESS ENTER").font(font_emulogic)),
            Text::new(TextFragment::new("a game for the 2020-21 APCSP create task").scale(Scale::uniform(0.75 * DEFAULT_FONT_SCALE))),
            Text::new(TextFragment::new("HP").font(font_emulogic)),
            Text::new(TextFragment::new("100").font(font_emulogic))
        ];

        let stats = GameStats {
            floor: 0,
            score: 0,
            health: 100.0,
            max_health: 100.0,
            attack: 10,
            defense: 5,
            speed: 2.5,
            tone: 15,
            accessories: [None; 5]
        };

        let piece = level::piece_from_dntp(ctx, "/piece/0.dntp").unwrap();
        let generator = level::Generator {
            pieces: vec!(piece.clone()),
            colors: [
                Color::from_rgb(77, 83, 102),
                Color::from_rgb(41, 59, 42), //77,102,83
                Color::from_rgb(92, 49, 59), //102,77,83
                Color::from_rgb(99, 40, 40)  //102,83,77
            ]
        };
        let mut level = level::Level {
            tiles: vec!(),
            color: {
                let colors = generator.colors;
                let i = fastrand::usize(..colors.len());
                colors[i]
            }
        };
        level.push_piece(ctx, &piece);
        level.init_textures(ctx);

        let drawable_size = graphics::drawable_size(ctx);

        let state = MainState {
            state: GameState::Menu(MenuState::Main),
            //state: GameState::InGame,
            paused: false,
            spritebatch: batch,
            music_source: music,
            font: font_emulogic,
            text_common: text_common,
            player_stats: stats,
            player_pos: Vector2::new(150.0, 150.0),
            player_vel: Vector2::new(0.0, 0.0),
            player_facing: Facing::Right,
            player_jump_time: 40,
            generator: generator,
            level: level,
            screen_size: Vector2::new(drawable_size.0, drawable_size.1),
            camera: CameraView::new()
        };

        Ok(state)
    }

    fn set_music(&mut self, ctx: &mut Context, src: &str) {
        self.music_source = audio::Source::new(ctx, format!("/audio/{}", src)).expect("Failed to change music!");
        self.music_source.set_repeat(true);
    }

    fn get_player_x(&self, _ctx: &mut Context) -> f32 { self.player_pos.x }
    fn get_player_y(&self, _ctx: &mut Context) -> f32 { self.player_pos.y }

    fn is_in_game(&self, _ctx: &mut Context) -> bool { self.state == GameState::InGame }

    fn modify_player_health(&mut self, ctx: &mut Context, num: f32) {
        assert!(self.is_in_game(ctx), "Tried to modify player health while not in game!");
        self.player_stats.health = clamp(self.player_stats.health + num, 0.0, self.player_stats.max_health);
        self.text_common[3] = Text::new(TextFragment::new(format!("{}", self.player_stats.health as i32)).font(self.font));
    }

    fn is_player_colliding(&self, ctx: &mut Context, dir: Direction) -> bool {
        assert!(self.is_in_game(ctx), "Tried to check player state while not in game!");
        let lvl_pos = level::screen_to_lvl_coords(ctx, self.player_pos.x, self.player_pos.y, self.screen_size.x, self.screen_size.y);

        let test_x = match dir {
            Direction::Left => lvl_pos.0 as usize-1,
            Direction::Right => lvl_pos.0 as usize+1,
            _ => lvl_pos.0 as usize
        };
        let test_y = match dir {
            Direction::Up => lvl_pos.1 as usize-1,
            Direction::Down => lvl_pos.1 as usize+1,
            _ => lvl_pos.1 as usize
        };
        let test_tile = self.level.get_tile(ctx, test_x, test_y);
        
        let x_offset = lvl_pos.0 % 1.0;
        let y_offset = lvl_pos.1 % 1.0;

        let test_tile_b: Option<level::LevelTile> = match dir {
            Direction::Up | Direction::Down => match x_offset {
                z if z > 0.70 => self.level.get_tile(ctx, test_x+1, test_y),
                z if z < 0.30 => self.level.get_tile(ctx, test_x-1, test_y),
                _ => None
            },
            Direction::Left | Direction::Right => match y_offset {
                z if z > 0.58 => self.level.get_tile(ctx, test_x, test_y+1),
                z if z < 0.42 => self.level.get_tile(ctx, test_x, test_y-1),
                _ => None
            }
        };
        (match (test_tile, test_tile_b) {
            (Some(tile_a), Some(tile_b)) => tile_a.collide || tile_b.collide,
            (Some(tile_a), None) => tile_a.collide,
            (None, Some(tile_b)) => tile_b.collide,
            _ => false
        } && match dir {
            Direction::Left => x_offset < 0.5,
            Direction::Right => x_offset > 0.5,
            Direction::Up => y_offset < 0.5,
            Direction::Down => y_offset > 0.5
        })
    }

    fn get_camera_scroll(&self, ctx: &mut Context) -> Vector2<f32> {
        assert!(self.is_in_game(ctx), "Tried to check camera state while not in game!");
        let tile_size = level::get_tile_drawn_size(ctx, self.camera.scale);
        let screen_y_half = self.screen_size.y / 2.0;

        let player_adjusted_y = level::screen_to_lvl_coords(ctx, self.player_pos.x, self.player_pos.y, self.screen_size.x, self.screen_size.y).1;
        let scroll_y: f32;
        if player_adjusted_y * tile_size <= screen_y_half {
            scroll_y = 0.0;
        } else if (self.level.height() as f32 - player_adjusted_y) * tile_size <= screen_y_half {
            scroll_y = self.level.height() as f32 * tile_size - self.screen_size.y;
        } else {
            scroll_y = player_adjusted_y * tile_size - screen_y_half;
        }
        Vector2::new(0.0, scroll_y)
    }
}

const MAX_FALL_SPEED: f32 = 60.0;

impl EventHandler for MainState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        match &self.state {
            GameState::Menu(state) => match state {
                MenuState::Main => {
                    if !self.music_source.playing() { 
                        self.set_music(ctx, "menu_loop.ogg");
                        self.music_source.play()?;
                    }
                    if keyboard::is_key_pressed(ctx, KeyCode::Return) {
                        self.state = GameState::InGame;
                        self.music_source.stop();
                        ()
                    }
                },
                _ => {}
            },
            GameState::InGame => {
                let player_tile = level::screen_to_lvl_coords(ctx, self.player_pos.x, self.player_pos.y, self.screen_size.x, self.screen_size.y);
                let grounded = self.is_player_colliding(ctx, Direction::Down);
                let x_speed_mult = if grounded { 1.0 } else { 1.25 };

                self.player_vel.x *= 0.85;
                if keyboard::is_key_pressed(ctx, KeyCode::A) {
                    self.player_vel.x -= self.player_stats.speed * x_speed_mult;
                }
                if keyboard::is_key_pressed(ctx, KeyCode::D) {
                    self.player_vel.x += self.player_stats.speed * x_speed_mult;
                }

                if self.player_vel.x > 0.0 { 
                    self.player_facing = Facing::Right;
                    if self.is_player_colliding(ctx, Direction::Right) {
                        self.player_vel.x = 0.0;
                    }
                } else if self.player_vel.x < 0.0 { 
                    self.player_facing = Facing::Left;
                    if self.is_player_colliding(ctx, Direction::Left) {
                        self.player_vel.x = 0.0;
                    }
                }

                if keyboard::is_key_pressed(ctx, KeyCode::Space) && self.player_jump_time > 0 {
                    if self.player_jump_time == 40 {
                        self.player_vel.y -= 30.0;
                    }
                    self.player_jump_time -= 1;
                }

                if !grounded { 
                    if !keyboard::is_key_pressed(ctx, KeyCode::Space) || self.player_jump_time == 0 {
                        self.player_jump_time = 0;
                        if self.player_vel.y < MAX_FALL_SPEED {
                            self.player_vel.y += 0.7; 
                        } else {
                            self.player_vel.y = MAX_FALL_SPEED;
                        }
                    }
                } else {
                    if !keyboard::is_key_pressed(ctx, KeyCode::Space) {
                        self.player_jump_time = 40;
                    }
                    if player_tile.1 % 1.0 > 0.51 {
                        self.player_pos.y -= 0.01 * level::TILE_DIMS * 6.0;
                    }
                }

                if self.player_vel.y > 0.0 && grounded || self.player_vel.y < 0.0 && self.is_player_colliding(ctx, Direction::Up) {
                    self.player_vel.y = 0.0;
                }

                //self.modify_player_health(ctx, -0.01);

                self.player_pos.x += self.player_vel.x / 8.0;
                self.player_pos.y += self.player_vel.y / 8.0;

                println!("{:?}", player_tile);
            },
            _ => {}
        };
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);
        graphics::set_default_filter(ctx, graphics::FilterMode::Nearest);
        let (max_width, max_height): (f32, f32) = (self.screen_size.x, self.screen_size.y);
        let time = (timer::duration_to_f64(timer::time_since_start(ctx)) * 1000.0) as f32;
        let text_scalef: f32 = 2.0;

        match &self.state {
            GameState::Menu(state) => match state {
                MenuState::Main => {
                    let cycle_time: f32 = 4000.0;
                    {
                        let logo = atlas_drawparam_base(ctx, Rect::new(48.0, 0.0, 65.0, 21.0))
                            .dest(Point2::new(max_width / 2.0, max_height / 4.0 + 15.0 + (2.0 * PI * time / cycle_time).cos() * 5.0))
                            .scale(Vector2::new(9.0, 9.0))
                            .offset(Point2::new(0.5, 0.5));
                        self.spritebatch.add(logo);
                    }

                    {
                        let bgr = graphics::Image::new(ctx, "/menu_bgr.png").expect("Could not load image!");
                        let bgr_param = graphics::DrawParam::new()
                            .dest(Point2::new(max_width / 2.0, max_height))
                            .scale(Vector2::new(max_width / bgr.width() as f32, max_height / bgr.height() as f32 + (2.0 * PI * time / cycle_time / 2.0).sin() * 0.25))
                            .offset(Point2::new(0.5, 1.0));
                        graphics::draw(ctx, &bgr, bgr_param)?;
                    }
                    
                    let text_width = self.text_common[1].width(ctx);
                    graphics::queue_text(ctx, &self.text_common[1], Point2::new(-(text_width as f32)/2.0, (max_height/2.0 - 80.0)/text_scalef), None);

                    if (time + 300.0) % 1263.0 > 631.5 {
                        let text_width = self.text_common[0].width(ctx);
                        graphics::queue_text(ctx, &self.text_common[0], Point2::new(-(text_width as f32) / 2.0, 35.0/* + (2.0 * PI * time / cycle_time / 2.0).sin() * 2.0*/), None);
                    }
                },
                _ => {}
            },
            GameState::InGame => {
                self.camera.scroll = 0.9 * self.camera.scroll + 0.1 * self.get_camera_scroll(ctx);
                
                // Level drawing
                {
                    for i in 0..self.level.tiles.len() {
                        for n in 0..self.level.tiles[i].len() {
                            //println!("self.level.tiles[{:?}][{:?}].tile_texture.unwrap()", i, n);
                            let tile = atlas_drawparam_base(ctx, self.level.tiles[i][n].tile_texture.unwrap())
                                .dest(Point2::new(level::TILE_DIMS * 6.0 * (n as f32 + (max_width / 6.0 / level::TILE_DIMS - level::LEVEL_WIDTH) / 2.0), level::TILE_DIMS * 6.0 * i as f32))
                                .scale(Vector2::new(6.0, 6.0))
                                .color(self.level.color);
                            self.spritebatch.add(tile);
                        }
                    }
                }

                // Player drawing
                {
                    let player_running = self.player_vel.x.abs() >= self.player_stats.speed * 0.2;
                    let player_rect: Rect = if player_running { 
                        pick_frame_rect(ctx, Rect::new(0.0, 0.0, 26.0, 8.0), 3, 133.3, time) 
                    } else { 
                        Rect::new(0.0, 0.0, 8.0, 8.0)
                    };
                    let player_bounce = if player_running { 6.0 } else { 1.0 };
                    let player = atlas_drawparam_base(ctx, player_rect)
                        .dest(Point2::new(self.get_player_x(ctx) + 18.0, self.get_player_y(ctx) + 24.0))
                        .scale(Vector2::new(
                            if self.player_facing == Facing::Left { -1.0 } else { 1.0 } * (5.9) + (2.0* PI*time/4000.0*player_bounce).sin() * 0.15, 
                            6.0 + (2.0 * PI * time / 4000.0 * player_bounce).cos() * 0.3))
                        .offset(Point2::new(0.5, 1.0));
                    self.spritebatch.add(player);
                }

                // Interface drawing
                {
                    let hp_bar = atlas_drawparam_base(ctx, Rect::new(48.0, 27.0, 46.0, 4.0))
                        .dest(Point2::new(80.0, 12.0 + self.camera.scroll.y))
                        .scale(Vector2::new(6.0, 6.0))
                        .offset(Point2::new(0.0, 0.0));
                    let hp_bar_frame = hp_bar.src(atlas_rect(ctx, Rect::new(48.0, 22.0, 46.0, 4.0)));

                    let hp_bar_shadow = graphics::DrawParam::color(hp_bar.dest(Point2::new(80.0, 15.0 + self.camera.scroll.y)), Color::from_rgb(0,0,0));
                    let hp_bar_frame_shadow = graphics::DrawParam::color(hp_bar_frame.dest(Point2::new(80.0, 15.0 + self.camera.scroll.y)), Color::from_rgb(0,0,0));
                    
                    let hp_prog: f32 = self.player_stats.health / self.player_stats.max_health;

                    self.spritebatch.add(hp_bar_shadow);
                    self.spritebatch.add(hp_bar_frame_shadow);
                    self.spritebatch.add(hp_bar.scale(Vector2::new(hp_prog * 6.0, 6.0)));
                    self.spritebatch.add(hp_bar_frame);
                    
                    graphics::queue_text(ctx, &self.text_common[2], Point2::new(8.0 - max_width/2.0/text_scalef, 4.0 + 2.0 - max_height/2.0/text_scalef), Some(Color::from_rgb(0,0,0)));
                    graphics::queue_text(ctx, &self.text_common[2], Point2::new(8.0 - max_width/2.0/text_scalef, 4.0 - max_height/2.0/text_scalef), None);

                    graphics::queue_text(ctx, &self.text_common[3], Point2::new(184.0 - max_width/2.0/text_scalef, 4.0 + 2.0 - max_height/2.0/text_scalef), Some(Color::from_rgb(0,0,0)));
                    graphics::queue_text(ctx, &self.text_common[3], Point2::new(184.0 - max_width/2.0/text_scalef, 4.0 - max_height/2.0/text_scalef), None);
                }
            },
            _ => {}
        };

        graphics::draw(ctx, &self.spritebatch, graphics::DrawParam::new().dest(Point2::new(0.0, -self.camera.scroll.y)))?;
        graphics::draw_queued_text(ctx, graphics::DrawParam::new()
            .dest(Point2::new(max_width/2.0, max_height/2.0))
            .scale(Vector2::new(text_scalef, text_scalef))
            .offset(Point2::new(0.5, 0.5)), 
            None, graphics::FilterMode::Nearest).expect("Failed to draw text!");
        self.spritebatch.clear();
        graphics::present(ctx)?;
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: KeyCode, _keymods: KeyMods, _repeat: bool) {
        match keycode {
            KeyCode::A => { if self.is_in_game(ctx) { self.player_facing = Facing::Left; } },
            KeyCode::D => { if self.is_in_game(ctx) { self.player_facing = Facing::Right; } },
            _ => {}
        }
    }
}

const ATLAS_WIDTH: f32 = 128.0;
const ATLAS_HEIGHT: f32 = 128.0;

/// Converts a Rect to texture atlas coordinates
pub fn atlas_rect(_ctx: &mut Context, rect: Rect) -> Rect {
    Rect::new(rect.x/ATLAS_WIDTH, rect.y/ATLAS_HEIGHT, rect.w/ATLAS_WIDTH, rect.h/ATLAS_HEIGHT)
}

/// Generates a DrawParam for an atlas texture with the given Rect as src
pub fn atlas_drawparam_base(ctx: &mut Context, rect: Rect) -> graphics::DrawParam {
    graphics::DrawParam::new().src(atlas_rect(ctx, rect))
}

/// Picks the next frame from a given atlas Rect based on current game time
fn pick_frame_rect(_ctx: &mut Context, frame_rect: Rect, frames: usize, interval: f32, cur_time: f32) -> Rect {
    assert!(frame_rect.x+frame_rect.w < ATLAS_WIDTH && frame_rect.y+frame_rect.h < ATLAS_HEIGHT);
    let anim_length: f32 = interval * frames as f32;
    let frame_index: usize = ((cur_time%anim_length/anim_length)*frames as f32) as usize;
    let frame_width: f32 = (frame_rect.w-frames as f32+1.0)/frames as f32;

    let rect = Rect::new(
        frame_rect.x/ATLAS_WIDTH + frame_index as f32*frame_width + frame_index as f32, 
        frame_rect.y/ATLAS_HEIGHT, 
        frame_width, 
        frame_rect.h
    );

    rect
}

/// Clamps input value between min and max
fn clamp<T>(input: T, min: T, max: T) -> T 
where T: PartialOrd<T> {
    assert!(max > min);
    if input < min { 
        min
    } else if input > max {
        max
    } else {
        input
    }
}

struct GameStats {
    floor: u32,
    score: i32,
    health: f32,
    max_health: f32,
    attack: i32,
    defense: i32,
    speed: f32,
    tone: i32,
    accessories: [Option<Accessory>; 5]
}

#[derive(PartialEq)]
enum Facing {
    Left,
    Right
}

// maybe redundant
#[derive(PartialEq)]
enum Direction {
    Up,
    Down,
    Left,
    Right
}

#[derive(Copy, Clone)]
enum Accessory {
    Belt(Modifier),
    Ring(Modifier),
    Necklace(Modifier),
    Gauntlet(Modifier),
    Armband(Modifier),
    Crystal(Modifier)
}

#[derive(Copy, Clone)]
enum Modifier {
    AtkBoost,
    DefBoost,
    SpdBoost,
    TonBoost
}

struct CameraView {
    scale: f32,
    scroll: Vector2<f32>
}

impl CameraView {
    pub fn new() -> Self {
        CameraView {
            scale: 1.0,
            scroll: Vector2::new(0.0, 0.0)
        }
    }
}