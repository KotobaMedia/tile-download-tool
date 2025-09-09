use std::{collections::BTreeMap, fs::File, path::PathBuf};

use anyhow::{Context, Result};
use flume::Receiver;
use pmtiles::{PmTilesStreamWriter, PmTilesWriter, TileType};
use tempfile::NamedTempFile;

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
    force: bool,
    output: PathBuf,

    out_pmt: PmTilesStreamWriter<File>,
    out_pmt_f: NamedTempFile,
    progress_tx: ProgressSender,
}

pub struct WriteTileMsg {
    pub index: usize,
    pub tile: Tile,
    /// None if the tile was not found / no data
    pub data: Option<Vec<u8>>,
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

        if !force && output.exists() {
            return Err(anyhow::anyhow!(
                "Output file {} already exists. Use --force to overwrite.",
                output.display()
            ));
        }

        let out_pmt_f = NamedTempFile::new_in(
            output
                .parent()
                .context("Output path must have a parent directory")?,
        )?;

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
        let out_pmt = out_pmt.create(out_pmt_f.reopen()?)?;

        Ok(Self {
            force,
            output,
            out_pmt,
            out_pmt_f,
            progress_tx,
        })
    }

    pub fn write(mut self, tile_rx: Receiver<WriteTileMsg>) -> Result<()> {
        let mut next = 0usize;
        // reorder buffer
        // TODO: use a more efficient structure
        let mut buf = BTreeMap::new();
        for msg in tile_rx {
            let WriteTileMsg { index, tile, data } = msg;
            buf.insert(index, (tile, data));
            while let Some((tile, data)) = buf.remove(&next) {
                if let Some(data) = data {
                    self.out_pmt.add_tile(*tile, &data)?;
                    self.progress_tx
                        .send(progress::ProgressMsg::Written(tile))?;
                }
                next += 1;
            }
        }

        self.progress_tx.send(progress::ProgressMsg::Log(
            "Finished writing tiles, finalizing archive...".to_string(),
        ))?;
        self.out_pmt.finalize()?;
        if self.force {
            self.out_pmt_f.persist(&self.output)?;
        } else {
            self.out_pmt_f.persist_noclobber(&self.output)?;
        }

        self.progress_tx.send(progress::ProgressMsg::Log(format!(
            "Finished writing {} tiles to {}.",
            next + 1,
            self.output.display()
        )))?;

        self.progress_tx.send(progress::ProgressMsg::Finished())?;

        Ok(())
    }
}
