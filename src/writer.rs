use std::{collections::BTreeMap, fs::File, path::PathBuf};

use anyhow::{Context, Result};
use flume::Receiver;
use pmtiles::{PmTilesStreamWriter, PmTilesWriter, TileType};

use crate::{
    metadata::Metadata,
    progress::{self, ProgressSender},
    tile::Tile,
    tile_list::TileListMeta,
};

fn str_to_tile_type(s: &str) -> TileType {
    match s {
        "png" => TileType::Png,
        "jpg" | "jpeg" => TileType::Jpeg,
        "webp" => TileType::Webp,
        "mvt" => TileType::Mvt,
        _ => TileType::Png, // default to png
    }
}

pub struct Writer {
    output: PathBuf,

    out_pmt: PmTilesStreamWriter<File>,
    progress_tx: ProgressSender,
}

impl Writer {
    pub fn new(
        output: PathBuf,
        force: bool,
        ext: &str,
        metadata: Metadata,
        tile_list_meta: TileListMeta,
        progress_tx: ProgressSender,
    ) -> Result<Self> {
        let tile_type = str_to_tile_type(ext);

        // Open output according to `force` semantics:
        // - force = true  -> create if missing, overwrite if exists (truncate)
        // - force = false -> create only, fail if already exists
        let out_pmt_f = if force {
            File::options()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&output)
        } else {
            File::options()
                .create_new(true)
                .write(true)
                .open(&output)
        }
        .context("Failed to open output file. Hint: try specifying --force if you want to overwrite an existing file.")?;
        let mut out_pmt = PmTilesWriter::new(tile_type)
            .metadata(serde_json::to_string(&metadata)?.as_str())
            .min_zoom(tile_list_meta.min_zoom)
            .max_zoom(tile_list_meta.max_zoom);
        if let Some((lon, lat)) = tile_list_meta.center {
            out_pmt = out_pmt.center(lon, lat);
        }
        if let Some((west, south, east, north)) = tile_list_meta.bounds {
            out_pmt = out_pmt.bounds(west, south, east, north);
        }
        let out_pmt = out_pmt.create(out_pmt_f)?;

        Ok(Self {
            output,
            out_pmt,
            progress_tx,
        })
    }

    pub fn write(mut self, tile_rx: Receiver<(usize, Tile, Vec<u8>)>) -> Result<()> {
        let mut next = 0usize;
        // reorder buffer
        // TODO: use a more efficient structure
        let mut buf = BTreeMap::new();
        for (index, tile, data) in tile_rx {
            buf.insert(index, (tile, data));
            while let Some((tile, data)) = buf.remove(&next) {
                self.out_pmt.add_tile(*tile, &data)?;
                self.progress_tx
                    .send(progress::ProgressMsg::Written(tile))?;
                next += 1;
            }
        }

        self.progress_tx.send(progress::ProgressMsg::Log(
            "Finished writing tiles, finalizing archive...".to_string(),
        ))?;
        self.out_pmt.finalize()?;

        self.progress_tx.send(progress::ProgressMsg::Log(format!(
            "Finished writing {} tiles to {}.",
            next + 1,
            self.output.display()
        )))?;

        self.progress_tx.send(progress::ProgressMsg::Finished())?;

        Ok(())
    }
}
