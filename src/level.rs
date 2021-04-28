use std::path;
use std::io::Read;
use ggez::{Context, GameResult};
use ggez::graphics::Rect;
use ggez::filesystem;

pub struct Level {
    pieces: Vec<LevelPiece>
}

pub struct LevelPiece {
    pub data: Vec<Vec<TileType>>
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

pub struct LevelTile {
    tile_type: TileType,
    atlas_region: Rect,
    tile_texture: Option<Rect>,
    collide: bool
}

pub struct Generator {
    pub pieces: Vec<LevelPiece>
}

const TILE_DIMS: f32 = 8.0;
const TILE_ROW_SIZE: f32 = 8.0;

pub fn init_tile_texture(ctx: &mut Context, tile: LevelTile, piece: LevelPiece, x: usize, y: usize) -> Rect {
    assert!(tile.atlas_region.w == 71.0 && tile.atlas_region.h == 17.0, "Invalid atlas region for tile!");
    let adjacent: [bool; 4] = [
        comp_tile(ctx, &piece, x, y-1, &tile.tile_type),
        comp_tile(ctx, &piece, x+1, y, &tile.tile_type),
        comp_tile(ctx, &piece, x, y+1, &tile.tile_type),
        comp_tile(ctx, &piece, x-1, y, &tile.tile_type)
    ];
    match adjacent.iter().filter(|&x| *x).count() {
        0 => { get_tile_texture_rect(ctx, tile.atlas_region, 0) },
        1 => {
            if adjacent[0] { get_tile_texture_rect(ctx, tile.atlas_region, 14) }
            else if adjacent[1] { get_tile_texture_rect(ctx, tile.atlas_region, 6) }
            else if adjacent[2] { get_tile_texture_rect(ctx, tile.atlas_region, 7) }
            else { get_tile_texture_rect(ctx, tile.atlas_region, 15) }
        },
        2 => {
            if adjacent[0] {
                if adjacent[1] { get_tile_texture_rect(ctx, tile.atlas_region, 9) }
                else if adjacent[2] { get_tile_texture_rect(ctx, tile.atlas_region, 5) }
                else { get_tile_texture_rect(ctx, tile.atlas_region, 10) }
            } else if adjacent[1] {
                if adjacent[2] { get_tile_texture_rect(ctx, tile.atlas_region, 1) }
                else { get_tile_texture_rect(ctx, tile.atlas_region, 13) }
            } else {
                get_tile_texture_rect(ctx, tile.atlas_region, 2)
            }
        },
        3 => {
            if !adjacent[0] { get_tile_texture_rect(ctx, tile.atlas_region, 4) }
            else if !adjacent[1] { get_tile_texture_rect(ctx, tile.atlas_region, 12) }
            else if !adjacent[2] { get_tile_texture_rect(ctx, tile.atlas_region, 3) }
            else { get_tile_texture_rect(ctx, tile.atlas_region, 11) }
        },
        4 => { get_tile_texture_rect(ctx, tile.atlas_region, 8) },
        _ => { panic!("This should never happen") }
    }
}

fn get_tile_texture_rect(_ctx: &mut Context, region: Rect, index: usize) -> Rect {
    let col: f32 = index as f32 % TILE_ROW_SIZE;
    let row: f32 = index as f32 / TILE_ROW_SIZE;
    Rect::new(region.x + (TILE_DIMS + 1.0) * col, region.y + (TILE_DIMS + 1.0) * row, TILE_DIMS, TILE_DIMS)
}

pub fn get_tile(_ctx: &mut Context, piece: &LevelPiece, x: usize, y: usize) -> Option<TileType> {
    let piece_w = piece.data.len();
    if piece_w == 0 { () } // <-- this looks like an eye
    let piece_h = piece.data[0].len();
    if piece_h == 0 { () }
    if x - 1 > piece_w || y - 1 > piece_h { () }

    Some(piece.data[x][y])
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

const TILES: [LevelTile; 4] = [
    LevelTile {
        tile_type: TileType::Brick,
        atlas_region: Rect::new(0.0, 111.0, 71.0, 17.0),
        tile_texture: None,
        collide: true
    },
    LevelTile {
        tile_type: TileType::Wood,
        atlas_region: Rect::new(0.0, 111.0, 71.0, 17.0),
        tile_texture: None,
        collide: true
    },
    LevelTile {
        tile_type: TileType::Metal,
        atlas_region: Rect::new(0.0, 111.0, 71.0, 17.0),
        tile_texture: None,
        collide: true
    },
    LevelTile {
        tile_type: TileType::Air,
        atlas_region: Rect::new(0.0, 111.0, 71.0, 17.0),
        tile_texture: None,
        collide: false
    },
];