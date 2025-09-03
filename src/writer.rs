use std::{fs::File, path::PathBuf};

use anyhow::{Context, Result};
use flume::Receiver;
use pmtiles::{PmTilesStreamWriter, PmTilesWriter, TileType};

use crate::{
    progress::{self, ProgressSender},
    tile::Tile,
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
        _name: Option<String>,
        _description: Option<String>,
        _attribution: Option<String>,
        progress_tx: ProgressSender,
    ) -> Result<Self> {
        let tile_type = str_to_tile_type(ext);

        let out_pmt_f = File::options()
            .create_new(!force)
            .write(true)
            .open(&output)
            .context("Failed to open output file. Hint: try specifying --force if you want to overwrite an existing file.")?;
        out_pmt_f.set_len(0)?;
        let out_pmt = PmTilesWriter::new(tile_type);
        let out_pmt = out_pmt.create(out_pmt_f)?;

        Ok(Self {
            output,
            out_pmt,
            progress_tx,
        })
    }

    pub fn write(mut self, tile_rx: Receiver<(Tile, Vec<u8>)>) -> Result<()> {
        for (tile, data) in tile_rx {
            self.out_pmt.add_tile(*tile, &data)?;
            self.progress_tx
                .send(progress::ProgressMsg::TileWritten(tile))?;
        }
        println!("Finished writing tiles, finalizing archive...");
        self.out_pmt.finalize()?;
        println!("Finished writing to {}.", self.output.display());
        Ok(())
    }
}
