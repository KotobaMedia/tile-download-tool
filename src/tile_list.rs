use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    str::FromStr,
};

use crate::{tile::Tile, tile_list_format::compile_tile_format};
use anyhow::Result;

pub struct TileListMeta {
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub center: Option<(f32, f32)>,
    pub bounds: Option<(f32, f32, f32, f32)>,
}

impl TileListMeta {
    pub fn new(min_zoom: u8, max_zoom: u8, tiles: &[Tile]) -> Self {
        let bounds = if tiles.is_empty() {
            None
        } else {
            // Only consider tiles at minimum zoom since child tiles are contained within parent tiles
            let min_zoom_tiles: Vec<_> = tiles.iter().filter(|tile| tile.z() == min_zoom).collect();
            let mut min_lon = f32::INFINITY;
            let mut min_lat = f32::INFINITY;
            let mut max_lon = f32::NEG_INFINITY;
            let mut max_lat = f32::NEG_INFINITY;
            for tile in min_zoom_tiles {
                let (lon_min, lat_min, lon_max, lat_max) = tile.bounds();
                min_lon = min_lon.min(lon_min);
                min_lat = min_lat.min(lat_min);
                max_lon = max_lon.max(lon_max);
                max_lat = max_lat.max(lat_max);
            }
            Some((min_lon, min_lat, max_lon, max_lat))
        };
        let center = bounds.map(|(min_lon, min_lat, max_lon, max_lat)| {
            ((min_lon + max_lon) / 2.0, (min_lat + max_lat) / 2.0)
        });
        Self {
            min_zoom,
            max_zoom,
            center,
            bounds,
        }
    }
}

pub struct TileList {
    pub tiles: Vec<Tile>,
    pub meta: TileListMeta,
}

impl TileList {
    pub fn parse_from_file(path: &str, format: &str) -> Result<Self> {
        let format_re = compile_tile_format(format)?;
        let reader = BufReader::new(File::open(path)?);
        let mut tiles = Vec::new();

        let mut min_zoom = 32;
        let mut max_zoom = 0;
        for line in reader.lines() {
            let line = line?;
            if let Some(caps) = format_re.captures(&line) {
                let tile = Tile::from_captures(&caps);
                min_zoom = min_zoom.min(tile.z());
                max_zoom = max_zoom.max(tile.z());
                tiles.push(tile);
            }
        }

        tiles.sort_by_key(|a| a.to_id());
        let meta = TileListMeta::new(min_zoom, max_zoom, &tiles);
        Ok(TileList { tiles, meta })
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
        tiles.sort_by_key(|a| a.to_id());
        let meta = TileListMeta::new(min, max, &tiles);
        Self { tiles, meta }
    }

    pub fn filter_zooms(&mut self, min: u8, max: u8) {
        self.tiles.retain(|tile| tile.z() >= min && tile.z() <= max);
        self.meta = TileListMeta::new(min, max, &self.tiles);
    }

    pub fn filter_bbox(&mut self, bbox: SimpleBBox) {
        self.tiles.retain(|tile| {
            let (tile_min_lon, tile_min_lat, tile_max_lon, tile_max_lat) = tile.bounds();
            !(tile_max_lon < bbox.min_x()
                || tile_min_lon > bbox.max_x()
                || tile_max_lat < bbox.min_y()
                || tile_min_lat > bbox.max_y())
        });
        if let Some((min_lon, min_lat, max_lon, max_lat)) = self.meta.bounds {
            let new_min_lon = min_lon.max(bbox.min_x());
            let new_min_lat = min_lat.max(bbox.min_y());
            let new_max_lon = max_lon.min(bbox.max_x());
            let new_max_lat = max_lat.min(bbox.max_y());
            self.meta.bounds = Some((new_min_lon, new_min_lat, new_max_lon, new_max_lat));
            self.meta.center = Some((
                (new_min_lon + new_max_lon) / 2.0,
                (new_min_lat + new_max_lat) / 2.0,
            ));
        }
    }

    pub fn remove_existing(&mut self, existing: &HashSet<Tile>) {
        self.tiles.retain(|tile| !existing.contains(tile));
        let (min_zoom, max_zoom) = self.tiles.iter().fold((32u8, 0u8), |(min_z, max_z), t| {
            (min_z.min(t.z()), max_z.max(t.z()))
        });
        self.meta = TileListMeta::new(min_zoom, max_zoom, &self.tiles);
    }
}

pub struct SimpleBBox(f32, f32, f32, f32);

impl SimpleBBox {
    pub fn min_x(&self) -> f32 {
        self.0
    }

    pub fn min_y(&self) -> f32 {
        self.1
    }

    pub fn max_x(&self) -> f32 {
        self.2
    }

    pub fn max_y(&self) -> f32 {
        self.3
    }
}

impl FromStr for SimpleBBox {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() != 4 {
            anyhow::bail!("Invalid bbox format. Expected format: min_x,min_y,max_x,max_y");
        }
        let min_x: f32 = parts[0].parse()?;
        let min_y: f32 = parts[1].parse()?;
        let max_x: f32 = parts[2].parse()?;
        let max_y: f32 = parts[3].parse()?;
        Ok(SimpleBBox(min_x, min_y, max_x, max_y))
    }
}
