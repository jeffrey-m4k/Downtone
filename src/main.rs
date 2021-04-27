use std::path;
use std::env;
use std::f32::consts::PI;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler, KeyCode, KeyMods};
use ggez::graphics;
use ggez::graphics::{Text, TextFragment};
use ggez::nalgebra;
use ggez::timer;
use ggez::audio;
use ggez::audio::SoundSource;
use ggez::input::keyboard;

mod draw;

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
    text_common: [Text; 3],
    player_stats: GameStats,
    player_pos: (f32, f32),
    player_vel: (f32, f32),
    player_facing: Facing
}

impl MainState {
    fn new(ctx: &mut Context) -> GameResult<MainState> {
        let mut atlas: graphics::Image = graphics::Image::new(ctx, "/atlas.png").expect("Could not load texture atlas!");
        atlas.set_filter(graphics::FilterMode::Nearest);
        let batch = graphics::spritebatch::SpriteBatch::new(atlas.clone());

        let mut music = audio::Source::new(ctx, "/audio/menu_loop.ogg")?;
        music.set_repeat(true);

        let font_emulogic =  graphics::Font::new(ctx, "/font/emulogic.ttf").expect("Could not load font!");
        let text_common: [_; 3] = [
            Text::new(TextFragment::new("PRESS ENTER").font(font_emulogic)),
            Text::new(TextFragment::new("a game for the 2020-21 APCSP create task")),
            Text::new(TextFragment::new("HEALTH").font(font_emulogic))
        ];

        let stats = GameStats {
            floor: 0,
            score: 0,
            health: 100,
            max_health: 100,
            attack: 10,
            defense: 5,
            speed: 10,
            tone: 15,
            accessories: [None; 5]
        };

        let state = MainState {
            state: GameState::Menu(MenuState::Main),
            //state: GameState::InGame,
            paused: false,
            spritebatch: batch,
            music_source: music,
            font: font_emulogic,
            text_common: text_common,
            player_stats: stats,
            player_pos: (100.0, 100.0),
            player_vel: (0.0, 0.0),
            player_facing: Facing::Right
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
            },
            _ => {}
        };
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);
        let (max_width, max_height): (f32, f32) = graphics::drawable_size(ctx);
        let time = (timer::duration_to_f64(timer::time_since_start(ctx)) * 1000.0) as f32;
        let text_scalef: f32 = 1.5;

        match &self.state {
            GameState::Menu(state) => match state {
                MenuState::Main => {
                    let cycle_time: f32 = 4000.0;
                    let logo = graphics::DrawParam::new()
                        .src(graphics::Rect::new(48.0/128.0, 0.0, 65.0/128.0, 21.0/128.0))
                        .dest(nalgebra::Point2::new(max_width / 2.0, max_height / 4.0 + 15.0 + (2.0 * PI * time / cycle_time).cos() * 5.0))
                        .scale(nalgebra::Vector2::new(9.0, 9.0))
                        .offset(nalgebra::Point2::new(0.5, 0.5));
                    self.spritebatch.add(logo);

                    let bgr = graphics::Image::new(ctx, "/menu_bgr.png").expect("Could not load image!");
                    let bgr_param = graphics::DrawParam::new()
                        .dest(nalgebra::Point2::new(max_width / 2.0, max_height))
                        .scale(nalgebra::Vector2::new(max_width / bgr.width() as f32, max_height / bgr.height() as f32 + (2.0 * PI * time / cycle_time / 2.0).sin() * 0.25))
                        .offset(nalgebra::Point2::new(0.5, 1.0));
                    graphics::draw(ctx, &bgr, bgr_param)?;

                    if (time + 300.0) % 1263.0 > 631.5 {
                        let text_width = self.text_common[0].width(ctx);
                        graphics::queue_text(ctx, &self.text_common[0], nalgebra::Point2::new(-(text_width as f32) / 2.0, 35.0/* + (2.0 * PI * time / cycle_time / 2.0).sin() * 2.0*/), None);
                        graphics::draw_queued_text(ctx, graphics::DrawParam::new()
                            .dest(nalgebra::Point2::new(max_width/2.0, max_height/2.0))
                            .scale(nalgebra::Vector2::new(2.0, 2.0))
                            .offset(nalgebra::Point2::new(0.5, 0.5)), 
                            None, graphics::FilterMode::Nearest).expect("Failed to draw text!");
                    }
                    
                    let text_width = self.text_common[1].width(ctx);
                    graphics::queue_text(ctx, &self.text_common[1], nalgebra::Point2::new(-(text_width as f32)/2.0, (max_height/2.0 - 80.0)/text_scalef), None);
                },
                _ => {}
            },
            GameState::InGame => {
                let player_rect: graphics::Rect = if self.player_vel.0 != 0.0 { 
                    pick_frame_rect(ctx, graphics::Rect::new(0.0, 0.0, 24.0, 8.0), 3, 128.0, 128.0, 250.0, time) 
                } else { 
                    graphics::Rect::new(0.0, 0.0, 8.0/128.0, 8.0/128.0)
                };
                let player = graphics::DrawParam::new()
                    .src(player_rect)
                    .dest(nalgebra::Point2::new(self.get_player_x(ctx), self.get_player_y(ctx)))
                    .scale(nalgebra::Vector2::new(if self.player_facing == Facing::Left { -8.0 } else { 8.0 }, 8.0))
                    .offset(nalgebra::Point2::new(0.5, 0.5));
                self.spritebatch.add(player);
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

fn pick_frame_rect(_ctx: &mut Context, frame_rect: graphics::Rect, frames: usize, img_width: f32, img_height: f32, interval: f32, cur_time: f32) -> graphics::Rect {
    let anim_length: f32 = interval * frames as f32;
    let frame_index: usize = ((cur_time%anim_length/anim_length)*frames as f32) as usize;
    let frame_width: f32 = frame_rect.w/frames as f32;

    let rect = graphics::Rect::new(
        (frame_rect.x/img_width + frame_index as f32*frame_width)/img_width, 
        (frame_rect.y/img_height)/img_height, 
        frame_width/img_width, 
        frame_rect.h/img_height
    );
    //println!("Rect X: {} | Rect Y: {} | Rect W: {} | Rect H: {} | Frame Number: {}", rect.x, rect.y, rect.w, rect.h, frame_index);

    rect
}

struct GameStats {
    floor: u32,
    score: i32,
    health: i32,
    max_health: i32,
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