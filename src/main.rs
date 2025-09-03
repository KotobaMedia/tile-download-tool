use anyhow::Result;
use clap::Parser;
use tokio::task::JoinSet;

use crate::{downloader::Downloader, metadata::Metadata, progress::Progress, writer::Writer};

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
    let cli = cli::Cli::parse();

    let tile_list = if let Some(tile_list_path) = &cli.tile_list {
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

    println!("Found {} tiles to download.", tile_list.tiles.len());

    let mut js = JoinSet::new();
    // Create a channel for downloaded tile data
    // ballpark estimate, one tile is 100KB -- at 4096 tiles, that gives us ~400MB inflight, max
    // Writing isn't hard so this is a worst case scenario
    let (tile_tx, tile_rx) = flume::bounded(4096);
    // The channel for progress updates
    let (progress_tx, progress_rx) = flume::bounded(4096);

    let metadata = Metadata::new(&cli);
    let writer = Writer::new(
        cli.output.clone(),
        cli.force,
        "png", // TODO: detect from URL
        metadata,
        tile_list.meta,
        progress_tx.clone(),
    )?;
    let progress = Progress::new(tile_list.tiles.len() as u64);
    let mut downloader = Downloader::new(&cli.url, tile_list.tiles, cli.concurrency, progress_tx);

    js.spawn(async move { downloader.download(tile_tx).await });
    js.spawn_blocking(move || writer.write(tile_rx));
    js.spawn_blocking(move || progress.run(progress_rx));

    while let Some(res) = js.join_next().await {
        res??;
    }

    Ok(())
}
