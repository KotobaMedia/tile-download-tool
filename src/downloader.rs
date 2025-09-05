use anyhow::{Result, bail};
use flume::Sender;
use reqwest::{Client, ClientBuilder};
use tokio::task::JoinSet;

use crate::{
    progress::{ProgressMsg, ProgressSender},
    tile::Tile,
    tile_urls::TileUrl,
};

pub struct Downloader {
    url_template: String,
    tiles: Vec<Tile>,
    concurrency: usize,
    client: Client,
    progress_tx: ProgressSender,
}

impl Downloader {
    pub fn new(
        url_template: &str,
        tiles: Vec<Tile>,
        concurrency: usize,
        progress_tx: ProgressSender,
    ) -> Self {
        let client = ClientBuilder::new()
            .user_agent(format!(
                "tile-download-tool/{} (+https://github.com/KotobaMedia/tile-download-tool)",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .unwrap();

        Self {
            url_template: url_template.to_string(),
            tiles,
            concurrency,
            client,
            progress_tx,
        }
    }

    pub async fn download(&mut self, output_tx: Sender<(usize, Tile, Vec<u8>)>) -> Result<()> {
        let (dlq_tx, dlq_rx) = flume::unbounded();
        let mut tasks = JoinSet::new();

        let tiles = std::mem::take(&mut self.tiles);
        tasks.spawn(async move {
            for (index, tile) in tiles.into_iter().enumerate() {
                dlq_tx.send_async((index, tile)).await?;
            }
            Ok::<_, anyhow::Error>(())
        });

        for _ in 0..self.concurrency {
            let client = self.client.clone();
            let url_template = self.url_template.clone();
            let dlq_rx = dlq_rx.clone();
            let output_tx = output_tx.clone();
            let progress_tx = self.progress_tx.clone();
            tasks.spawn(async move {
                while let Ok((index, tile)) = dlq_rx.recv_async().await {
                    let tile_url = TileUrl::from_template(&url_template, tile.clone());

                    if let Some(bytes) = download_tile(&client, tile_url).await? {
                        progress_tx
                            .send_async(ProgressMsg::Downloaded(tile.clone(), bytes.len()))
                            .await?;
                        output_tx.send_async((index, tile, bytes)).await?;
                    } else {
                        progress_tx.send_async(ProgressMsg::Skipped()).await?;
                    }
                }
                Ok::<_, anyhow::Error>(())
            });
        }

        while let Some(res) = tasks.join_next().await {
            res??;
        }

        self.progress_tx
            .send(ProgressMsg::Log("All downloads complete.".to_string()))?;

        Ok(())
    }
}

async fn download_tile(client: &Client, tile_url: TileUrl) -> Result<Option<Vec<u8>>> {
    let resp = client.get(tile_url.url()).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND
        || resp.status() == reqwest::StatusCode::NO_CONTENT
    {
        return Ok(None);
    }
    if !resp.status().is_success() {
        bail!("Failed to download tile: {}", resp.status());
    }
    let bytes = resp.bytes().await?;
    Ok(Some(bytes.to_vec()))
}
