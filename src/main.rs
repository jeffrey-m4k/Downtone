#![allow(dead_code)]

use std::path;
use std::env;
use std::f32::consts::PI;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler, KeyCode, KeyMods};
use ggez::graphics;
use ggez::graphics::{Text, TextFragment, Rect, Scale, DEFAULT_FONT_SCALE};
use ggez::nalgebra;
use ggez::timer;
use ggez::audio;
use ggez::audio::SoundSource;
use ggez::input::keyboard;

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

struct MainState {
    state: GameState,
    paused: bool,
    spritebatch: graphics::spritebatch::SpriteBatch,
    music_source: audio::Source,
    font: graphics::Font,
    text_common: [Text; 4],
    player_stats: GameStats,
    player_pos: (f32, f32),
    player_vel: (f32, f32),
    player_facing: Facing,
    generator: level::Generator,
    level: level::Level
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
            speed: 10,
            tone: 15,
            accessories: [None; 5]
        };

        let piece = level::piece_from_dntp(ctx, "/piece/0.dntp").unwrap();
        let generator = level::Generator {
            pieces: vec!(piece.clone())
        };
        let mut level = level::Level {
            tiles: vec!()
        };
        level.push_piece(ctx, &piece);

        let state = MainState {
            state: GameState::Menu(MenuState::Main),
            //state: GameState::InGame,
            paused: false,
            spritebatch: batch,
            music_source: music,
            font: font_emulogic,
            text_common: text_common,
            player_stats: stats,
            player_pos: (100.0, 300.0),
            player_vel: (0.0, 0.0),
            player_facing: Facing::Right,
            generator: generator,
            level: level
        };

        Ok(state)
    }

    fn set_music(&mut self, ctx: &mut Context, src: &str) {
        self.music_source = audio::Source::new(ctx, format!("/audio/{}", src)).expect("Failed to change music!");
        self.music_source.set_repeat(true);
    }

    fn get_player_x(&self, _ctx: &mut Context) -> f32 { self.player_pos.0 }
    fn get_player_y(&self, _ctx: &mut Context) -> f32 { self.player_pos.1 }

    fn is_in_game(&self, _ctx: &mut Context) -> bool { self.state == GameState::InGame }

    fn modify_player_health(&mut self, ctx: &mut Context, num: f32) {
        assert!(self.is_in_game(ctx), "Tried to modify player health while not in game!");
        self.player_stats.health = clamp(self.player_stats.health + num, 0.0, self.player_stats.max_health);
        self.text_common[3] = Text::new(TextFragment::new(format!("{}", self.player_stats.health as i32)).font(self.font));
    }
}

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
                self.player_vel.0 = 0.0;
                if keyboard::is_key_pressed(ctx, KeyCode::A) {
                    self.player_vel.0 -= self.player_stats.speed as f32;
                }
                if keyboard::is_key_pressed(ctx, KeyCode::D) {
                    self.player_vel.0 += self.player_stats.speed as f32;
                }

                self.player_pos.0 += self.player_vel.0 / 8.0;
                self.player_pos.1 += self.player_vel.1 / 8.0;

                if self.player_vel.0 > 0.0 { 
                    self.player_facing = Facing::Right 
                } else if self.player_vel.0 < 0.0 { 
                    self.player_facing = Facing::Left
                }

                self.modify_player_health(ctx, -0.01);
            },
            _ => {}
        };
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);
        graphics::set_default_filter(ctx, graphics::FilterMode::Nearest);
        let (max_width, max_height): (f32, f32) = graphics::drawable_size(ctx);
        let time = (timer::duration_to_f64(timer::time_since_start(ctx)) * 1000.0) as f32;
        let text_scalef: f32 = 2.0;

        match &self.state {
            GameState::Menu(state) => match state {
                MenuState::Main => {
                    let cycle_time: f32 = 4000.0;
                    {
                        let logo = atlas_drawparam_base(ctx, Rect::new(48.0, 0.0, 65.0, 21.0))
                            .dest(nalgebra::Point2::new(max_width / 2.0, max_height / 4.0 + 15.0 + (2.0 * PI * time / cycle_time).cos() * 5.0))
                            .scale(nalgebra::Vector2::new(9.0, 9.0))
                            .offset(nalgebra::Point2::new(0.5, 0.5));
                        self.spritebatch.add(logo);
                    }

                    {
                        let bgr = graphics::Image::new(ctx, "/menu_bgr.png").expect("Could not load image!");
                        let bgr_param = graphics::DrawParam::new()
                            .dest(nalgebra::Point2::new(max_width / 2.0, max_height))
                            .scale(nalgebra::Vector2::new(max_width / bgr.width() as f32, max_height / bgr.height() as f32 + (2.0 * PI * time / cycle_time / 2.0).sin() * 0.25))
                            .offset(nalgebra::Point2::new(0.5, 1.0));
                        graphics::draw(ctx, &bgr, bgr_param)?;
                    }
                    
                    let text_width = self.text_common[1].width(ctx);
                    graphics::queue_text(ctx, &self.text_common[1], nalgebra::Point2::new(-(text_width as f32)/2.0, (max_height/2.0 - 80.0)/text_scalef), None);

                    if (time + 300.0) % 1263.0 > 631.5 {
                        let text_width = self.text_common[0].width(ctx);
                        graphics::queue_text(ctx, &self.text_common[0], nalgebra::Point2::new(-(text_width as f32) / 2.0, 35.0/* + (2.0 * PI * time / cycle_time / 2.0).sin() * 2.0*/), None);
                    }
                },
                _ => {}
            },
            GameState::InGame => {
                {
                    for i in 0..self.level.tiles.len() {
                        for n in 0..self.level.tiles[i].len() {
                            //println!("self.level.tiles[{:?}][{:?}].tile_texture.unwrap()", i, n);
                            let tile = atlas_drawparam_base(ctx, self.level.tiles[i][n].tile_texture.unwrap())
                                .dest(nalgebra::Point2::new(level::TILE_DIMS * 6.0 * n as f32, level::TILE_DIMS * 6.0 * i as f32))
                                .scale(nalgebra::Vector2::new(6.0, 6.0));
                            self.spritebatch.add(tile);
                        }
                    }
                }

                {
                    let player_rect: Rect = if self.player_vel.0 != 0.0 { 
                        pick_frame_rect(ctx, Rect::new(0.0, 0.0, 26.0, 8.0), 3, 133.3, time) 
                    } else { 
                        Rect::new(0.0, 0.0, 8.0, 8.0)
                    };
                    let player_bounce = if self.player_vel.0 != 0.0 { 6.0 } else { 1.0 };
                    let player = atlas_drawparam_base(ctx, player_rect)
                        .dest(nalgebra::Point2::new(self.get_player_x(ctx), self.get_player_y(ctx)))
                        .scale(nalgebra::Vector2::new(
                            if self.player_facing == Facing::Left { -1.0 } else { 1.0 } * (5.9) + (2.0* PI*time/4000.0*player_bounce).sin() * 0.15, 
                            6.0 + (2.0 * PI * time / 4000.0 * player_bounce).cos() * 0.3))
                        .offset(nalgebra::Point2::new(0.5, 1.0));
                    self.spritebatch.add(player);
                }

                {
                    let hp_bar = atlas_drawparam_base(ctx, Rect::new(48.0, 27.0, 46.0, 4.0))
                        .dest(nalgebra::Point2::new(80.0, 12.0))
                        .scale(nalgebra::Vector2::new(6.0, 6.0))
                        .offset(nalgebra::Point2::new(0.0, 0.0));
                    let hp_bar_frame = hp_bar.src(atlas_rect(ctx, Rect::new(48.0, 22.0, 46.0, 4.0)));
                    
                    let hp_prog: f32 = self.player_stats.health / self.player_stats.max_health;

                    self.spritebatch.add(hp_bar.scale(nalgebra::Vector2::new(hp_prog * 6.0, 6.0)));
                    self.spritebatch.add(hp_bar_frame);
                    
                    graphics::queue_text(ctx, &self.text_common[2], nalgebra::Point2::new(8.0 - max_width/2.0/text_scalef, 4.0 - max_height/2.0/text_scalef), None);
                    graphics::queue_text(ctx, &self.text_common[3], nalgebra::Point2::new(184.0 - max_width/2.0/text_scalef, 4.0 - max_height/2.0/text_scalef), None);
                }
            },
            _ => {}
        };

        graphics::draw(ctx, &self.spritebatch, graphics::DrawParam::new())?;
        graphics::draw_queued_text(ctx, graphics::DrawParam::new()
            .dest(nalgebra::Point2::new(max_width/2.0, max_height/2.0))
            .scale(nalgebra::Vector2::new(text_scalef, text_scalef))
            .offset(nalgebra::Point2::new(0.5, 0.5)), 
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
    speed: i32,
    tone: i32,
    accessories: [Option<Accessory>; 5]
}

#[derive(PartialEq)]
enum Facing {
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