use std::path;
use std::io::Read;
use ggez::{Context, GameResult};
use ggez::graphics::{Rect, Color};
use ggez::filesystem;
use crate::MainState;


pub const LEVEL_WIDTH: f32 = 16.0;

pub struct Level {
    pub tiles: Vec<Vec<LevelTile>>,
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
            self.tiles.push(temp_vec);
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
        let tex_id = if adjacent[0] {
            if adjacent[1] {
                if adjacent[2] {
                    if adjacent[3] { 8 } // 0,1,2,3
                    else { 3 } // 0,1,2
                } 
                else if adjacent[3] { 11 } // 0,1,3
                else { 9 } // 0,1
            } 
            else if adjacent[2] {
                if adjacent[3] { 12 } // 0,2,3
                else { 5 } // 0,2
            }
            else if adjacent[3] { 10 } // 0,3
            else { 14 } // 0
        }
        else if adjacent[1] { 
            if adjacent[2] {
                if adjacent[3] { 4 } // 1,2,3
                else { 1 } // 1,2
            }
            else if adjacent[3] { 13 } // 1,3
            else { 6 } // 1
        }
        else if adjacent[2] {
            if adjacent[3] { 2 } // 2,3
            else { 7 } // 2
        } 
        else if adjacent[3] { 15 } // 3
        else { 0 }; // none
        //println!("({:?}, {:?}): {:?} | {:?}", x, y, tex_id, adjacent);
        let tex = get_tile_texture_rect(ctx, atlas_region, tex_id);
        self.tiles[x][y].tile_texture = Some(tex);
    }
}

pub fn screen_to_lvl_coords(_ctx: &mut Context, x: f32, y: f32, screen_w: f32, _screen_h: f32) -> (f32, f32) {
    let _x_cap = TILE_DIMS * 6.0 / screen_w;
    let x_offset = 6.0 * (screen_w / 6.0 / TILE_DIMS - LEVEL_WIDTH) / 2.0;
    ((x + x_offset) / TILE_DIMS / 6.0, y / TILE_DIMS / 6.0)
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

pub fn piece_from_dntp<P: AsRef<path::Path>>(ctx: &mut Context, path: P) -> GameResult<LevelPiece> {
    let mut dntp = String::new();
    let mut f = filesystem::open(ctx, path)?;
    f.read_to_string(&mut dntp)?;

    let mut rows = dntp.split('~');

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