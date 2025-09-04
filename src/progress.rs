use anyhow::Result;
use flume::Receiver;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::tile::Tile;

pub enum ProgressMsg {
    Skipped(),

    /// A tile was downloaded. (Tile, byte size)
    Downloaded(Tile, usize),
    Written(Tile),
}

pub type ProgressSender = flume::Sender<ProgressMsg>;

pub struct Progress {
    tile_dl: ProgressBar,
    tile_dl_bytes: ProgressBar,
    tile_written: ProgressBar,
}

impl Progress {
    pub fn new(initial_count: u64) -> Self {
        let m = MultiProgress::new();

        let tile_dl = m.add(ProgressBar::new(initial_count));
        tile_dl.set_style(
            ProgressStyle::with_template(
                "Tile DL {msg} {bar:40.cyan/blue} {pos:>11}/{len:11} ({percent}%) ({eta})",
            )
            .unwrap(),
        );

        let tile_dl_bytes = m.add(ProgressBar::new(0));
        tile_dl_bytes.set_style(
            ProgressStyle::with_template(
                "DLBytes {msg} {bar:40.cyan/blue} {bytes:>11}/{total_bytes:11} ({percent}%) ({eta})",
            )
            .unwrap(),
        );

        let tile_written = m.add(ProgressBar::new(initial_count));
        tile_written.set_style(
            ProgressStyle::with_template(
                "TileOut {msg} {bar:40.cyan/blue} {pos:>11}/{len:11} ({percent}%) ({eta})",
            )
            .unwrap(),
        );

        Self {
            tile_dl,
            tile_dl_bytes,
            tile_written,
        }
    }

    pub fn run(&self, rx: Receiver<ProgressMsg>) -> Result<()> {
        while let Ok(msg) = rx.recv() {
            match msg {
                ProgressMsg::Skipped() => {
                    self.tile_dl.dec_length(1);
                    self.tile_written.dec_length(1);
                }
                ProgressMsg::Downloaded(tile, size) => {
                    self.tile_dl.inc(1);
                    self.tile_dl_bytes.inc(size as u64);

                    let avg_bytes_per_tile =
                        self.tile_dl_bytes.position() as usize / self.tile_dl.position() as usize;
                    self.tile_dl_bytes
                        .set_length(avg_bytes_per_tile as u64 * self.tile_dl.length().unwrap());

                    let tile_str = format!("{:<14}", tile.to_string());
                    self.tile_dl.set_message(tile_str.clone());
                    self.tile_dl_bytes.set_message(tile_str);
                }
                ProgressMsg::Written(tile) => {
                    self.tile_written.inc(1);
                    let tile_str = format!("{:<14}", tile.to_string());
                    self.tile_written.set_message(tile_str);
                }
            }
        }
        Ok(())
    }
}
