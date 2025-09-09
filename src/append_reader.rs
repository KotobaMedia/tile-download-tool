use std::{collections::HashSet, path::Path, sync::Arc};

use anyhow::{Context, Result};
use flume::Sender;
use futures_util::TryStreamExt;
use pmtiles::{AsyncPmTilesReader, MmapBackend};

use crate::{tile::Tile, writer::WriteTileMsg};

pub type PmTilesReader = Arc<AsyncPmTilesReader<MmapBackend>>;

pub struct AppendReader {
    reader: PmTilesReader,
}

impl AppendReader {
    pub async fn new(input: &Path) -> Result<Self> {
        let reader = AsyncPmTilesReader::new_with_path(input)
            .await
            .with_context(|| {
                format!(
                    "Failed to create PMTiles reader when trying to append to {}. Does the file exist?",
                    input.display()
                )
            })?;
        Ok(Self {
            reader: Arc::new(reader),
        })
    }

    pub async fn get_tiles(&self) -> Result<HashSet<Tile>> {
        let entries = self
            .reader
            .clone()
            .entries()
            .try_collect::<Vec<_>>()
            .await?;
        let tiles = entries
            .iter()
            .flat_map(|e| e.iter_coords())
            .map(|c| c.into())
            .collect::<HashSet<_>>();
        Ok(tiles)
    }

    pub async fn read_tiles(&self, output_tx: Sender<WriteTileMsg>) -> Result<usize> {
        let entries = self
            .reader
            .clone()
            .entries()
            .try_collect::<Vec<_>>()
            .await?;
        let tile_iter = entries.iter().flat_map(|e| e.iter_coords());
        let mut last_index = 0;
        for (index, tile) in tile_iter.enumerate() {
            let data = self.reader.clone().get_tile(tile).await?;
            let msg = WriteTileMsg {
                index,
                tile: tile.into(),
                data: data.map(|d| d.to_vec()),
            };
            output_tx.send(msg)?;
            last_index = index;
        }
        Ok(last_index + 1)
    }
}
