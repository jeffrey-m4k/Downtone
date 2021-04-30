use std::path;
use std::io::Read;
use std::f32::{consts::PI};
use ggez::{Context, GameResult};
use ggez::graphics::{Rect, Color};
use ggez::filesystem;
use ggez::nalgebra::Vector2;
use crate::{CameraView, MainState, clamp};


pub const LEVEL_WIDTH: f32 = 16.0;

pub struct Level {
    pub tiles: Vec<Vec<LevelTile>>,
    pub lightmap: Vec<Vec<u8>>,
    pub last_update: f32,
    pub color: Color
}

impl Level {
    pub fn push_piece(&mut self, ctx: &mut Context, piece: &LevelPiece) {
        assert!(piece.data.len() > 0);
        let vec_h = piece.data.len() as usize;
        let vec_w = piece.get_width(ctx) as usize;
        let data = &piece.data;
        for i in 0..vec_h {
            let mut temp_vec: Vec<LevelTile> = vec!();
            for n in 0..vec_w {
                let level_tile = type_to_tile(ctx, data[i][n]);
                temp_vec.push(level_tile);
            }
            let size = temp_vec.len();
            self.tiles.push(temp_vec);
            self.lightmap.push(vec![15; size]);
        }
    }

    pub fn get_tile(&self, _ctx: &mut Context, x: usize, y: usize) -> Option<LevelTile> {
        if !(self.tiles.len() > y) || !(self.tiles[y].len() > x) { 
            None 
        } else {
            Some(self.tiles[y][x])
        }
    }

    pub fn comp_tile(&self, ctx: &mut Context, x: usize, y: usize, match_type: &TileType) -> bool {
        let tile = self.get_tile(ctx, x, y);
        match tile {
            Some(t) => { &(t.tile_type) == match_type },
            None => { false }
        }
    }

    pub fn height(&self) -> usize {
        self.tiles.len()
    }

    pub fn width(&self) -> usize {
        if self.height() == 0 { 0 } else { self.tiles[0].len() }
    }

    pub fn init_textures(&mut self, ctx: &mut Context) {
        for i in 0..self.height() {
            for n in 0..self.width() {
                self.init_tile_texture(ctx, i, n);
            }
        }
    }

    fn init_tile_texture(&mut self, ctx: &mut Context, x: usize, y: usize) {
        let tile = self.tiles[x][y];
        let x_max = self.height() as usize-1;
        let y_max = self.width() as usize-1;

        let atlas_region = TILE_REGIONS[tile.tile_type as usize];
        assert!(atlas_region.w == 71.0 && atlas_region.h == 17.0, "Invalid atlas region for tile!");
        let adjacent: [bool; 4] = [
            if x>0 { self.comp_tile(ctx, y, x-1, &tile.tile_type) } else { true },
            if y<y_max { self.comp_tile(ctx, y+1, x, &tile.tile_type) } else { true },
            if x<x_max { self.comp_tile(ctx, y, x+1, &tile.tile_type) } else { true },
            if y>0 { self.comp_tile(ctx, y-1, x, &tile.tile_type) } else { true },
        ];
        let tex_id = match adjacent {
            [true, true, true, true] => { 8 },
            [true, true, true, false] => { 3 },
            [true, true, false, true] => { 11 },
            [true, false, true, true] => { 12 },
            [false, true, true, true] => { 4 },
            [true, true, false, false] => { 9 },
            [true, false, true, false] => { 5 },
            [false, true, true, false] => { 1 },
            [true, false, false, true] => { 10 },
            [false, true, false, true] => { 13 },
            [false, false, true, true] => { 2 },
            [true, false, false, false] => { 14 },
            [false, true, false, false] => { 6 },
            [false, false, true, false] => { 7 },
            [false, false, false, true] => { 15 },
            _ => 0
        };
        let tex = get_tile_texture_rect(ctx, atlas_region, tex_id);
        self.tiles[x][y].tile_texture = Some(tex);
    }

    pub fn update_lightmap(&mut self, ctx: &mut Context, camera: &CameraView, screen_size: Vector2<f32>, player_pos: Vector2<f32>) {
        let screen_y_tiles = screen_to_lvl_y(ctx, screen_size.y);
        let y_min = clamp(screen_to_lvl_y(ctx, camera.scroll.y) as i8 - screen_y_tiles as i8, 0, self.height() as i8 - 1) as usize;
        let y_max = clamp(y_min + 3 * screen_y_tiles as usize, y_min, self.height() - 1);
        let player_pos = screen_to_lvl_coords(ctx, player_pos.x, player_pos.y, screen_size.x);

        for i in 0..self.width() {
            for n in y_min..=y_max {
                self.update_light(ctx, i, n, player_pos);
            }
        }
    }

    fn update_light(&mut self, _ctx: &mut Context, x: usize, y: usize, player_pos: Vector2<f32>) {

        let mut light = 60i8;

        //let target_x = player_pos.x as usize;
        //let target_y = player_pos.y as usize;
        
        let dist = ((player_pos.y - y as f32).powf(2.0) + (player_pos.x - x as f32).powf(2.0)).sqrt() * 8.0;
        light = clamp(light - dist as i8, 12, 60);
        self.lightmap[y][x] = light as u8;

        // 👇👇👇 too hard :( 
        /*let mut cur_x = x as f32;
        let mut cur_y = y as f32;
        while (cur_x != target_x || cur_y != target_y) && light > 3 {
            //let dist = ((target_y - cur_y as usize).pow(2) as f32 + (target_x - cur_x).pow(2) as f32).sqrt();
            let angle = fit_angle(cur_y.atan2(cur_x) - target_y.atan2(target_x));
            println!("Tracing tile: ({}, {}) | Angle: {}", cur_x, cur_y, angle / (2.0 * PI) * 360.0);
            match angle {
                a if a >= PI/4.0 && a <= PI*3.0/4.0 => { cur_y -= 1.0; },
                a if a >= PI*3.0/4.0 && a <= PI*5.0/4.0 => { cur_x -= 1.0; },
                a if a >= PI*5.0/4.0 && a <= PI*7.0/4.0 => { cur_y += 1.0; },
                _ => { cur_x += 1.0 }
            }
            light -= 1;
            if self.get_tile(ctx, cur_x as usize, cur_y as usize).unwrap().collide {
                light = 0;
            }
        }
        self.lightmap[y][x] = light;*/
    }
}

pub fn screen_to_lvl_coords(ctx: &mut Context, x: f32, y: f32, screen_w: f32) -> Vector2<f32> {
    let x_offset = 6.0 * (screen_w / 6.0 / TILE_DIMS - LEVEL_WIDTH) / 2.0;
    Vector2::new((x + x_offset) / TILE_DIMS / 6.0, screen_to_lvl_y(ctx, y))
}

fn screen_to_lvl_y(_ctx: &mut Context, y: f32) -> f32 {
    y / TILE_DIMS / 6.0
}

fn fit_angle(theta: f32) -> f32 {
    let theta = theta % (2.0 * PI);
    if theta < 0.0 {
        2.0 * PI + theta
    } else {
        theta
    }
}

#[derive(Clone, Debug)]
pub struct LevelPiece {
    pub data: Vec<Vec<TileType>>
}

impl LevelPiece {
    pub fn get_width(&self, _ctx: &mut Context) -> f32 {
        if self.data.len() == 0 { 0.0 }
        else { self.data[0].len() as f32 }
    }
}

pub fn piece_from_string(string: String) -> GameResult<LevelPiece> {
    let mut rows = string.split('~');

    let mut data: Vec<Vec<TileType>> = vec!();
    let mut row_index: usize = 0;

    loop {
        let s = rows.next();
        match s {
            Some(row) => {
                data.push(vec!());
                let mut blocks = row.trim().split('_');
                loop {
                    let b = blocks.next();
                    match b  {
                        Some(block) => {
                            let mut comp = block.split(':');

                            let tile = comp.next().unwrap().parse::<usize>().unwrap();
                            let count = comp.next().unwrap().parse::<usize>().unwrap();
                            for _ in 0..count {
                                data[row_index].push(TILES[tile].tile_type);
                            }
                        },
                        _ => { break; },
                    }
                };
                row_index += 1;
            },
            _ => { break; },
        }
    };
    Ok(LevelPiece {
        data: data
    })
}

pub fn piece_from_dntp<P: AsRef<path::Path>>(ctx: &mut Context, path: P) -> GameResult<LevelPiece> {
    let mut dntp = String::new();
    let mut f = filesystem::open(ctx, path)?;
    f.read_to_string(&mut dntp)?;

    piece_from_string(dntp)
}

#[derive(Copy, Clone, Debug)]
pub struct LevelTile {
    pub tile_type: TileType,
    pub tile_texture: Option<Rect>,
    pub collide: bool
}

pub struct Generator {
    pub pieces: Vec<LevelPiece>,
    pub colors: [Color; 4]
}

pub const TILE_DIMS: f32 = 8.0;
const TILE_ROW_SIZE: f32 = 8.0;

pub fn get_tile_texture_rect(_ctx: &mut Context, region: Rect, index: usize) -> Rect {
    let col: f32 = (index as u32 % TILE_ROW_SIZE as u32) as f32; 
    let row: f32 = (index as u32 / TILE_ROW_SIZE as u32) as f32;
    Rect::new(region.x + (TILE_DIMS + 1.0) * col, region.y + (TILE_DIMS + 1.0) * row, TILE_DIMS, TILE_DIMS)
}

pub fn get_tile_drawn_size(_ctx: &mut Context, scale: f32) -> f32 {
    TILE_DIMS * 6.0 / scale
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum TileType {
    Brick = 0,
    Wood = 1,
    Metal = 2,
    Air = 3
}

fn type_to_tile(_ctx: &mut Context, t_type: TileType) -> LevelTile {
    TILES[t_type as usize]
}

const TILES: [LevelTile; 4] = [
    LevelTile {
        tile_type: TileType::Brick,
        tile_texture: None,
        collide: true
    },
    LevelTile {
        tile_type: TileType::Wood,
        tile_texture: None,
        collide: true
    },
    LevelTile {
        tile_type: TileType::Metal,
        tile_texture: None,
        collide: true
    },
    LevelTile {
        tile_type: TileType::Air,
        tile_texture: None,
        collide: false
    },
];

const TILE_REGIONS: [Rect; 4] = [
    Rect::new(0.0, 111.0, 71.0, 17.0),
    Rect::new(0.0, 111.0, 71.0, 17.0),
    Rect::new(0.0, 111.0, 71.0, 17.0),
    Rect::new(0.0, 93.0, 71.0, 17.0)
];