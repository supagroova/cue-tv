//! Embedded byte transport. The HTTP adapter binds only to loopback.
mod ffi;
use anyhow::{Context, Result, bail};
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::StreamExt;
use librqbit::{AddTorrent, AddTorrentOptions, DhtSessionConfig, Magnet, Session, SessionOptions};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::{io::ReaderStream, sync::CancellationToken};

/// A single byte range, inclusive. Multipart ranges are deliberately rejected.
pub fn byte_range(value: Option<&str>, len: u64) -> Result<(u64, u64, bool)> {
    if len == 0 {
        bail!("empty resource");
    }
    let Some(value) = value else {
        return Ok((0, len - 1, false));
    };
    let rest = value.strip_prefix("bytes=").context("range unit")?;
    let (start, end) = rest.split_once('-').context("range syntax")?;
    if start.is_empty() {
        let suffix: u64 = end.parse()?;
        if suffix == 0 {
            bail!("empty suffix");
        }
        return Ok((len.saturating_sub(suffix), len - 1, true));
    }
    let start: u64 = start.parse()?;
    let end = if end.is_empty() {
        len - 1
    } else {
        end.parse::<u64>()?.min(len - 1)
    };
    if start >= len || end < start {
        bail!("unsatisfiable range");
    }
    Ok((start, end, true))
}

#[derive(Clone)]
struct StreamState {
    torrent: Arc<librqbit::ManagedTorrent>,
    index: usize,
    len: u64,
    cancel: CancellationToken,
}

async fn serve(State(state): State<StreamState>, method: Method, headers: HeaderMap) -> Response {
    let range = match headers.get("range") {
        Some(h) => match h.to_str() {
            Ok(s) => Some(s),
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        None => None,
    };
    let (start, end, partial) = match byte_range(range, state.len) {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [("content-range", format!("bytes */{}", state.len))],
            )
                .into_response();
        }
    };
    let count = end - start + 1;
    let mut response = Response::builder()
        .status(if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header("accept-ranges", "bytes")
        .header("content-length", count)
        .header("content-type", "video/mp4")
        .header("cache-control", "no-store");
    if partial {
        response = response.header(
            "content-range",
            format!("bytes {start}-{end}/{}", state.len),
        );
    }
    if method == Method::HEAD {
        return response.body(Body::empty()).unwrap();
    }
    let result = async {
        let mut stream = state.torrent.clone().stream(state.index).await?;
        stream.seek(std::io::SeekFrom::Start(start)).await?;
        Ok::<_, anyhow::Error>(stream)
    };
    let stream = tokio::select! {
        _ = state.cancel.cancelled() => return StatusCode::GONE.into_response(),
        r = tokio::time::timeout(Duration::from_secs(30), result) => match r {
            Ok(Ok(s)) => s,
            _ => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    };
    response
        .body(Body::from_stream(
            ReaderStream::new(stream.take(count)).take_until(state.cancel.cancelled_owned()),
        ))
        .unwrap()
}

/// Owns both swarm and HTTP server. Call stop before discarding to await teardown.
pub struct PlaybackSession {
    pub url: String,
    session: Arc<Session>,
    torrent: Arc<librqbit::ManagedTorrent>,
    cancel: CancellationToken,
    server: tokio::task::JoinHandle<()>,
}

pub enum TorrentSource {
    Bytes(Vec<u8>),
    Magnet(String),
}

impl PlaybackSession {
    pub async fn start(
        torrent: Vec<u8>,
        index: usize,
        output: PathBuf,
        initial_peers: Vec<SocketAddr>,
    ) -> Result<Self> {
        Self::start_source(
            TorrentSource::Bytes(torrent),
            index,
            output,
            initial_peers,
            false,
        )
        .await
    }

    pub async fn start_source(
        source: TorrentSource,
        index: usize,
        output: PathBuf,
        initial_peers: Vec<SocketAddr>,
        discovery: bool,
    ) -> Result<Self> {
        let source = match source {
            TorrentSource::Bytes(bytes) => AddTorrent::from_bytes(bytes),
            TorrentSource::Magnet(value) => {
                if value.len() > 16384 || !value.starts_with("magnet:?") {
                    bail!("invalid magnet");
                }
                let magnet = Magnet::parse(&value)?;
                if magnet.as_id20().is_none() {
                    bail!("v1 torrent hash required");
                }
                AddTorrent::from_url(value)
            }
        };
        let cancel = CancellationToken::new();
        let guard = cancel.clone().drop_guard();
        let session = Session::new_with_opts(
            output,
            SessionOptions {
                dht: discovery.then(|| DhtSessionConfig {
                    persistence: None,
                    ..Default::default()
                }),
                persistence: None,
                disable_trackers: !discovery,
                disable_local_service_discovery: true,
                cancellation_token: Some(cancel.clone()),
                peer_limit: Some(30),
                runtime_worker_threads: Some(2),
                ..Default::default()
            },
        )
        .await?;
        let setup = async {
            let handle = session
                .add_torrent(
                    source,
                    Some(AddTorrentOptions {
                        only_files: Some(vec![index]),
                        initial_peers: Some(initial_peers),
                        ..Default::default()
                    }),
                )
                .await?
                .into_handle()
                .context("torrent handle")?;
            handle.wait_until_initialized().await?;
            let len = handle
                .with_metadata(|m| m.file_infos.get(index).map(|f| f.len))?
                .context("file index")?;
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let path = format!("/{}/stream.mp4", uuid::Uuid::new_v4());
            let router = Router::new()
                .route(&path, get(serve))
                .with_state(StreamState {
                    torrent: handle.clone(),
                    index,
                    len,
                    cancel: cancel.clone(),
                });
            let stopped = cancel.clone();
            let server = tokio::spawn(async move {
                let _ = axum::serve(listener, router)
                    .with_graceful_shutdown(stopped.cancelled_owned())
                    .await;
            });
            Ok::<_, anyhow::Error>((format!("http://{address}{path}"), server, handle))
        }
        .await;
        match setup {
            Ok((url, server, torrent)) => {
                guard.disarm();
                Ok(Self {
                    url,
                    session,
                    torrent,
                    cancel,
                    server,
                })
            }
            Err(e) => {
                cancel.cancel();
                session.stop().await;
                Err(e)
            }
        }
    }
    pub fn downloaded_bytes(&self) -> u64 {
        self.torrent.stats().progress_bytes
    }

    pub async fn stop(self) {
        self.cancel.cancel();
        self.session.stop().await;
        self.server.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_are_bounded_and_reject_multipart() {
        assert_eq!(byte_range(None, 10).unwrap(), (0, 9, false));
        assert_eq!(byte_range(Some("bytes=3-100"), 10).unwrap(), (3, 9, true));
        assert_eq!(byte_range(Some("bytes=7-"), 10).unwrap(), (7, 9, true));
        assert_eq!(byte_range(Some("bytes=-3"), 10).unwrap(), (7, 9, true));
        for value in [
            "bytes=10-",
            "bytes=4-2",
            "bytes=-0",
            "bytes=0-1,4-5",
            "items=0-1",
            "bytes=18446744073709551616-",
        ] {
            assert!(byte_range(Some(value), 10).is_err(), "{value}");
        }
    }
}

impl Drop for PlaybackSession {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.server.abort();
    }
}
