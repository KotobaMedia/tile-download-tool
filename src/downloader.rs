use anyhow::{Result, anyhow, bail};
use flume::Sender;
use reqwest::{Client, ClientBuilder};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio::time::{Duration, sleep};

use crate::{
    progress::{ProgressMsg, ProgressSender},
    tile::Tile,
    tile_urls::TileUrl,
    writer::WriteTileMsg,
};

pub struct Downloader {
    url_template: String,
    tiles: Vec<Tile>,
    concurrency: usize,
    client: Client,
    progress_tx: ProgressSender,
    cancel: Arc<RwLock<bool>>,
}

impl Downloader {
    pub fn new(
        url_template: &str,
        tiles: Vec<Tile>,
        concurrency: usize,
        progress_tx: ProgressSender,
        cancel: Arc<RwLock<bool>>,
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
            cancel,
        }
    }

    pub async fn download(
        &mut self,
        start_idx: usize,
        output_tx: Sender<WriteTileMsg>,
    ) -> Result<()> {
        let (dlq_tx, dlq_rx) = flume::unbounded();
        let mut tasks = JoinSet::new();

        let tiles = std::mem::take(&mut self.tiles);
        tasks.spawn(async move {
            for (index, tile) in tiles.into_iter().enumerate() {
                if dlq_tx.send_async((start_idx + index, tile)).await.is_err() {
                    break;
                }
            }
            Ok::<_, anyhow::Error>(())
        });

        for _ in 0..self.concurrency {
            let client = self.client.clone();
            let url_template = self.url_template.clone();
            let dlq_rx = dlq_rx.clone();
            let output_tx = output_tx.clone();
            let progress_tx = self.progress_tx.clone();
            let cancel = self.cancel.clone();
            tasks.spawn(async move {
                while let Ok((index, tile)) = dlq_rx.recv_async().await {
                    if *cancel.read().await {
                        break;
                    }

                    let tile_url = TileUrl::from_template(&url_template, tile.clone());

                    let mut msg = WriteTileMsg {
                        index,
                        tile: tile.clone(),
                        data: None,
                    };
                    match download_tile(&client, tile_url).await {
                        Ok(Some(bytes)) => {
                            progress_tx
                                .send_async(ProgressMsg::Downloaded(tile.clone(), bytes.len()))
                                .await?;
                            msg.data = Some(bytes);
                            output_tx.send_async(msg).await?;
                        }
                        Ok(None) => {
                            progress_tx.send_async(ProgressMsg::Skipped()).await?;
                            output_tx.send_async(msg).await?;
                        }
                        Err(e) => {
                            // Log the failure via progress logger, request cancellation, then error out
                            let _ = progress_tx
                                .send_async(ProgressMsg::Log(format!(
                                    "Error downloading tile {}: {}",
                                    tile, e
                                )))
                                .await;
                            {
                                let mut w = cancel.write().await;
                                *w = true;
                            }
                            return Err(e);
                        }
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
    const MAX_ATTEMPTS: usize = 4;
    let url = tile_url.url();

    for attempt in 1..=MAX_ATTEMPTS {
        match attempt_download(client, &url).await {
            Ok(Some(bytes)) => return Ok(Some(bytes)),
            Ok(None) => return Ok(None),
            Err(AttemptError::Fatal(e)) => return Err(e),
            Err(AttemptError::Retryable(e)) => {
                if attempt == MAX_ATTEMPTS {
                    return Err(e);
                }
                let delay_ms = 200u64.saturating_mul(1u64 << (attempt - 1));
                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    bail!("Failed to download tile after retries")
}

enum AttemptError {
    Retryable(anyhow::Error),
    Fatal(anyhow::Error),
}

async fn attempt_download(
    client: &Client,
    url: &str,
) -> std::result::Result<Option<Vec<u8>>, AttemptError> {
    let resp = client
        .get(url.to_string())
        .send()
        .await
        .map_err(|e| AttemptError::Retryable(e.into()))?;

    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::NO_CONTENT {
        return Ok(None);
    }

    if status.is_success() {
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AttemptError::Retryable(e.into()))?;
        return Ok(Some(bytes.to_vec()));
    }

    let retryable = status.is_server_error()
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::REQUEST_TIMEOUT;

    if retryable {
        Err(AttemptError::Retryable(anyhow!("HTTP error: {}", status)))
    } else {
        Err(AttemptError::Fatal(anyhow!(
            "Failed to download tile: {}",
            status
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::Tile;
    use crate::tile_urls::TileUrl;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::time::timeout;

    async fn spawn_scripted_server(
        statuses: Vec<u16>,
        ok_body: &'static [u8],
    ) -> (SocketAddr, Arc<AtomicUsize>) {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let hit = Arc::new(AtomicUsize::new(0));
        let hit_clone = hit.clone();

        tokio::spawn(async move {
            loop {
                // Exit if no connection arrives for a bit to avoid hanging tests
                let Ok(Ok((mut socket, _))) =
                    timeout(Duration::from_millis(3000), listener.accept()).await
                else {
                    break;
                };

                let i = hit_clone.fetch_add(1, Ordering::SeqCst);
                let status = *statuses.get(i).unwrap_or_else(|| statuses.last().unwrap());

                // Read request headers (best-effort)
                let mut buf = vec![0u8; 1024];
                let _ = timeout(Duration::from_millis(200), socket.read(&mut buf)).await;

                let (status_line, body) = match status {
                    200 => ("200 OK", ok_body),
                    204 => ("204 No Content", &b""[..]),
                    400 => ("400 Bad Request", &b""[..]),
                    404 => ("404 Not Found", &b""[..]),
                    408 => ("408 Request Timeout", &b""[..]),
                    429 => ("429 Too Many Requests", &b""[..]),
                    500 => ("500 Internal Server Error", &b""[..]),
                    503 => ("503 Service Unavailable", &b""[..]),
                    _ => ("500 Internal Server Error", &b""[..]),
                };

                let resp = format!(
                    "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    status_line,
                    body.len()
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                if !body.is_empty() {
                    let _ = socket.write_all(body).await;
                }
                let _ = socket.shutdown().await;
            }
        });

        (addr, hit)
    }

    fn make_url(addr: SocketAddr) -> String {
        format!("http://{}/{{z}}/{{x}}/{{y}}", addr)
    }

    #[tokio::test]
    async fn retries_on_500_then_succeeds() {
        let (addr, hit) = spawn_scripted_server(vec![500, 200], b"OK").await;
        let client = reqwest::Client::new();
        let url_template = make_url(addr);
        let tile_url = TileUrl::from_template(&url_template, Tile::new(0, 0, 0));

        let data = download_tile(&client, tile_url)
            .await
            .expect("should not error");
        assert_eq!(data.as_deref(), Some(&b"OK"[..]));
        assert!(
            hit.load(Ordering::SeqCst) >= 2,
            "should attempt at least twice"
        );
    }

    #[tokio::test]
    async fn gives_up_after_4_attempts_on_5xx() {
        let (addr, hit) = spawn_scripted_server(vec![500, 500, 500, 500, 500], b"").await;
        let client = reqwest::Client::new();
        let url_template = make_url(addr);
        let tile_url = TileUrl::from_template(&url_template, Tile::new(0, 0, 0));

        let _err = download_tile(&client, tile_url)
            .await
            .expect_err("should error after retries");
        let attempts = hit.load(Ordering::SeqCst);
        assert!(attempts >= 4, "should try 4 times, got {}", attempts);
        // Optional: ensure not absurdly high
        assert!(attempts <= 5);
    }

    #[tokio::test]
    async fn skips_on_404_without_retry() {
        let (addr, hit) = spawn_scripted_server(vec![404], b"").await;
        let client = reqwest::Client::new();
        let url_template = make_url(addr);
        let tile_url = TileUrl::from_template(&url_template, Tile::new(0, 0, 0));

        let data = download_tile(&client, tile_url)
            .await
            .expect("404 should not error");
        assert!(data.is_none(), "404 should return None");
        assert_eq!(hit.load(Ordering::SeqCst), 1, "should not retry on 404");
    }

    #[tokio::test]
    async fn does_not_retry_on_400() {
        let (addr, hit) = spawn_scripted_server(vec![400, 200], b"OK").await;
        let client = reqwest::Client::new();
        let url_template = make_url(addr);
        let tile_url = TileUrl::from_template(&url_template, Tile::new(0, 0, 0));

        let err = download_tile(&client, tile_url)
            .await
            .expect_err("400 should be fatal");
        let attempts = hit.load(Ordering::SeqCst);
        assert_eq!(
            attempts, 1,
            "should not retry on 400, got {} attempts",
            attempts
        );
        let _ = err; // silence warning in case of different formatting
    }
}
