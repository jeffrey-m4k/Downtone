use std::path;
use std::io::Read;
use ggez::{Context, GameResult};
use ggez::graphics::Rect;
use ggez::filesystem;

const LEVEL_WIDTH: f32 = 16.0;

pub struct Level {
    pub tiles: Vec<Vec<LevelTile>>
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
                let mut level_tile = type_to_tile(ctx, data[i][n]);
                level_tile.init_tile_texture(ctx, piece.clone(), i, n);
                temp_vec.push(level_tile);
            }
            self.tiles.push(temp_vec);
        }
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

impl LevelTile {
    pub fn init_tile_texture(&mut self, ctx: &mut Context, piece: LevelPiece, x: usize, y: usize) {
        let atlas_region = TILE_REGIONS[self.tile_type as usize];
        assert!(atlas_region.w == 71.0 && atlas_region.h == 17.0, "Invalid atlas region for tile!");
        let adjacent: [bool; 4] = [
            if y>0 { comp_tile(ctx, &piece, x, y-1, &self.tile_type) } else { false },
            if x<usize::MAX { comp_tile(ctx, &piece, x+1, y, &self.tile_type) } else { false },
            if y<usize::MAX { comp_tile(ctx, &piece, x, y+1, &self.tile_type) } else { false },
            if x>0 { comp_tile(ctx, &piece, x-1, y, &self.tile_type) } else { false }
        ];
        let tex = match adjacent.iter().filter(|&x| *x).count() {
            0 => { get_tile_texture_rect(ctx, atlas_region, 0) },
            1 => {
                if adjacent[0] { get_tile_texture_rect(ctx, atlas_region, 14) }
                else if adjacent[1] { get_tile_texture_rect(ctx, atlas_region, 6) }
                else if adjacent[2] { get_tile_texture_rect(ctx, atlas_region, 7) }
                else { get_tile_texture_rect(ctx, atlas_region, 15) }
            },
            2 => {
                if adjacent[0] {
                    if adjacent[1] { get_tile_texture_rect(ctx, atlas_region, 9) }
                    else if adjacent[2] { get_tile_texture_rect(ctx, atlas_region, 5) }
                    else { get_tile_texture_rect(ctx, atlas_region, 10) }
                } else if adjacent[1] {
                    if adjacent[2] { get_tile_texture_rect(ctx, atlas_region, 1) }
                    else { get_tile_texture_rect(ctx, atlas_region, 13) }
                } else {
                    get_tile_texture_rect(ctx, atlas_region, 2)
                }
            },
            3 => {
                if !adjacent[0] { get_tile_texture_rect(ctx, atlas_region, 4) }
                else if !adjacent[1] { get_tile_texture_rect(ctx, atlas_region, 12) }
                else if !adjacent[2] { get_tile_texture_rect(ctx, atlas_region, 3) }
                else { get_tile_texture_rect(ctx, atlas_region, 11) }
            },
            4 => { get_tile_texture_rect(ctx, atlas_region, 8) },
            _ => { panic!("This should never happen") }
        };
        self.tile_texture = Some(tex);
    }
}

pub struct Generator {
    pub pieces: Vec<LevelPiece>
}

pub const TILE_DIMS: f32 = 8.0;
const TILE_ROW_SIZE: f32 = 8.0;

pub fn get_tile_texture_rect(_ctx: &mut Context, region: Rect, index: usize) -> Rect {
    let col: f32 = index as f32 % TILE_ROW_SIZE;
    let row: f32 = index as f32 / TILE_ROW_SIZE;
    Rect::new(region.x + (TILE_DIMS + 1.0) * col, region.y + (TILE_DIMS + 1.0) * row, TILE_DIMS, TILE_DIMS)
}

pub fn get_tile(_ctx: &mut Context, piece: &LevelPiece, x: usize, y: usize) -> Option<TileType> {
    let piece_h = piece.data.len();
    if piece_h == 0 { return None } // <-- this looks like an eye
    let piece_w = piece.data[0].len();
    if piece_w == 0 { return None }
    if x >= piece_w || y >= piece_h { return None }

    //println!("Some({:?}.data[{:?}][{:?}]) | piece_h = {:?}, piece_w = {:?}", piece, x, y, piece_h, piece_w);
    Some(piece.data[y][x])
}

pub fn comp_tile(ctx: &mut Context, piece: &LevelPiece, x: usize, y: usize, match_type: &TileType) -> bool {
    let tile = get_tile(ctx, piece, x, y);
    match tile {
        Some(t_type) => { &t_type == match_type },
        None => { false }
    }
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