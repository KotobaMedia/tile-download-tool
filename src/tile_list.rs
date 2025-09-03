use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{tile::Tile, tile_list_format::compile_tile_format};
use anyhow::Result;

pub struct TileList {
    pub tiles: Vec<Tile>,
}

impl TileList {
    pub fn parse_from_file(path: &str, format: &str) -> Result<Self> {
        let format_re = compile_tile_format(format)?;
        let reader = BufReader::new(File::open(path)?);
        let mut tiles = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if let Some(caps) = format_re.captures(&line) {
                let tile = Tile::from_captures(&caps);
                tiles.push(tile);
            }
        }

        tiles.sort_by(|a, b| a.to_id().cmp(&b.to_id()));

        Ok(TileList { tiles })
    }

    pub fn from_zoom_range(min: u8, max: u8) -> Self {
        let mut tiles = Vec::new();
        for z in min..=max {
            for x in 0..(1 << z) {
                for y in 0..(1 << z) {
                    tiles.push(Tile::new(z, x, y));
                }
            }
        }
        Self { tiles }
    }

    pub fn filter_zooms(&mut self, min: u8, max: u8) {
        self.tiles.retain(|tile| tile.z() >= min && tile.z() <= max);
    }
}
