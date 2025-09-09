use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinSet;

use crate::{
    downloader::Downloader,
    metadata::Metadata,
    progress::{Progress, ProgressMsg},
    writer::Writer,
};

mod append_reader;
mod cli;
mod downloader;
mod metadata;
mod progress;
mod tile;
mod tile_list;
mod tile_list_format;
mod tile_urls;
mod writer;

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = cli::Cli::parse();

    let mut tile_list = if let Some(tile_list_path) = &cli.tile_list {
        println!("Parsing tile list from {}...", &tile_list_path);
        let mut tile_list =
            tile_list::TileList::parse_from_file(tile_list_path, &cli.tile_list_format)?;
        tile_list.filter_zooms(cli.minimum_zoom, cli.maximum_zoom);
        tile_list
    } else {
        println!(
            "Downloading all tiles from zoom {} to {}...",
            cli.minimum_zoom, cli.maximum_zoom
        );
        tile_list::TileList::from_zoom_range(cli.minimum_zoom, cli.maximum_zoom)
    };
    if let Some(bbox_str) = &cli.bbox {
        println!("Filtering tiles by bounding box {}...", bbox_str);
        tile_list.filter_bbox(bbox_str.parse()?);
    }

    let expected_tile_len = tile_list.tiles.len();
    println!(
        "Expected number of tiles to download: {}",
        expected_tile_len
    );

    let append_reader = if cli.append {
        println!("Reading existing tiles from {}...", cli.output.display());
        let append_reader = append_reader::AppendReader::new(&cli.output).await?;
        let existing_tiles = append_reader.get_tiles().await?;
        tile_list.remove_existing(&existing_tiles);
        println!(
            "Skipping {} tiles already present in the existing PMTiles file.",
            existing_tiles.len()
        );
        // --append implies --force
        cli.force = true;
        Some(append_reader)
    } else {
        None
    };

    let mut js = JoinSet::new();
    // Create a channel for downloaded tile data
    // ballpark estimate, one tile is 100KB -- at 4096 tiles, that gives us ~400MB inflight, max
    // Writing isn't hard so this is a worst case scenario
    let (tile_tx, tile_rx) = flume::bounded(4096);
    // The channel for progress updates
    let (progress_tx, progress_rx) = flume::bounded(4096);
    // Cancellation signal shared with tasks
    let cancel = Arc::new(RwLock::new(false));

    let metadata = Metadata::new(&cli);
    let inferred_ext = tile_urls::infer_tile_format(&cli.url);
    let writer = Writer::new(
        cli.output.clone(),
        cli.force,
        &inferred_ext,
        metadata,
        tile_list.meta,
        progress_tx.clone(),
    )?;
    let progress = Progress::new(expected_tile_len as u64);
    let mut downloader = Downloader::new(
        &cli.url,
        tile_list.tiles,
        cli.concurrency,
        progress_tx.clone(),
        cancel.clone(),
    );

    // Handle Ctrl-C to trigger shutdown
    let progress_tx2 = progress_tx.clone();
    let cancel2 = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = progress_tx2.send(ProgressMsg::Log(
                "Ctrl-C received; cancelling downloads and finalizing...".to_string(),
            ));
            let mut w = cancel2.write().await;
            *w = true;
        }
    });

    // we start the writer and progress first so that they are ready to receive tiles
    js.spawn_blocking(move || writer.write(tile_rx));
    js.spawn_blocking(move || progress.run(progress_rx));

    // preload existing tiles if appending
    let mut download_start_idx = 0usize;
    if let Some(ar) = append_reader {
        let tile_tx = tile_tx.clone();
        download_start_idx = ar.read_tiles(tile_tx).await?;
    }
    // start the downloader after all existing tiles have been queued, so that indexing is correct
    js.spawn(async move { downloader.download(download_start_idx, tile_tx).await });

    // Wait for all tasks to finish; if any failed, remember the first error
    let mut first_err: Option<anyhow::Error> = None;
    while let Some(res) = js.join_next().await {
        match res {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
            Err(join_err) => {
                if first_err.is_none() {
                    first_err = Some(anyhow::anyhow!(join_err));
                }
            }
        }
    }

    if let Some(e) = first_err {
        // Ensure a clean exit after finalization; surface non-zero status by returning Err
        return Err(e);
    }

    println!("All done!");

    Ok(())
}
