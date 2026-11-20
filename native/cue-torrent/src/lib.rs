//! Embedded byte transport. The HTTP adapter binds only to loopback.
pub mod failure;
mod ffi;
pub mod metadata;
mod metadata_ffi;
use failure::FailureCode;
pub mod storage_budget;
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
use librqbit::storage::StorageFactoryExt;
use librqbit::{AddTorrent, AddTorrentOptions, DhtSessionConfig, Magnet, Session, SessionOptions};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use storage_budget::{BudgetFactory, DiskProbe, StorageBudget};
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
    content_type: &'static str,
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
        .header("content-type", state.content_type)
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
struct StorageLease(BudgetFactory);
impl Drop for StorageLease {
    fn drop(&mut self) {
        self.0.close();
    }
}

pub struct PlaybackSession {
    storage: StorageLease,
    pub url: String,
    session: Arc<Session>,
    torrent: Arc<librqbit::ManagedTorrent>,
    cancel: CancellationToken,
    server: tokio::task::JoinHandle<()>,
}

#[derive(Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct Discovery {
    pub trackers: bool,
    pub dht: bool,
    pub bootstrap: Option<Vec<String>>,
}

pub enum TorrentSource {
    Bytes(Vec<u8>),
    Magnet(String),
}

impl PlaybackSession {
    /// Consume metadata and discovered peers after an explicit file choice.
    /// Reuses the resolved torrent rather than repeating magnet discovery.
    pub async fn start_resolved(
        resolved: metadata::Resolved,
        index: usize,
        output: PathBuf,
        discovery: Discovery,
        cache_root: Option<PathBuf>,
    ) -> Result<Self> {
        resolved.selected_file(index)?;
        Self::start_budgeted_in_root(
            TorrentSource::Bytes(resolved.torrent_bytes),
            index,
            output,
            resolved.seen_peers,
            discovery,
            StorageBudget::default(),
            None,
            cache_root,
        )
        .await
    }

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
            Discovery::default(),
        )
        .await
    }

    pub async fn start_source(
        source: TorrentSource,
        index: usize,
        output: PathBuf,
        initial_peers: Vec<SocketAddr>,
        discovery: Discovery,
    ) -> Result<Self> {
        Self::start_budgeted(
            source,
            index,
            output,
            initial_peers,
            discovery,
            StorageBudget::default(),
            None,
        )
        .await
    }

    pub async fn start_budgeted(
        source: TorrentSource,
        index: usize,
        output: PathBuf,
        initial_peers: Vec<SocketAddr>,
        discovery: Discovery,
        budget: StorageBudget,
        probe: Option<DiskProbe>,
    ) -> Result<Self> {
        Self::start_budgeted_in_root(
            source,
            index,
            output,
            initial_peers,
            discovery,
            budget,
            probe,
            None,
        )
        .await
    }

    pub async fn start_budgeted_in_root(
        source: TorrentSource,
        index: usize,
        output: PathBuf,
        initial_peers: Vec<SocketAddr>,
        discovery: Discovery,
        budget: StorageBudget,
        probe: Option<DiskProbe>,
        cache_root: Option<PathBuf>,
    ) -> Result<Self> {
        let accounting_root = if let Some(root) = cache_root {
            if !root.is_absolute()
                || !output.is_absolute()
                || std::fs::symlink_metadata(&root)?.file_type().is_symlink()
                || std::fs::symlink_metadata(&output)?.file_type().is_symlink()
            {
                bail!("invalid cache root");
            }
            let canonical_root = root.canonicalize()?;
            let canonical_output = output.canonicalize()?;
            if canonical_output.parent() != Some(canonical_root.as_path()) {
                bail!("cache root must be output parent");
            }
            canonical_root
        } else {
            output.clone()
        };
        let storage = BudgetFactory::new(accounting_root, budget, probe);
        let storage_lease = StorageLease(storage.clone());
        let source = match source {
            TorrentSource::Bytes(bytes) => AddTorrent::from_bytes(bytes),
            TorrentSource::Magnet(value) => {
                if value.len() > 16384 || !value.starts_with("magnet:?") {
                    bail!(FailureCode::InvalidMetadata);
                }
                let magnet = Magnet::parse(&value).context(FailureCode::InvalidMetadata)?;
                if magnet.as_id20().is_none() {
                    bail!(FailureCode::UnsupportedFormat);
                }
                AddTorrent::from_url(value)
            }
        };
        let cancel = CancellationToken::new();
        let guard = cancel.clone().drop_guard();
        let session = Session::new_with_opts(
            output,
            SessionOptions {
                dht: discovery.dht.then(|| DhtSessionConfig {
                    bootstrap_addrs: discovery.bootstrap,
                    persistence: None,
                    ..Default::default()
                }),
                persistence: None,
                default_storage_factory: Some(storage.boxed()),
                disable_trackers: !discovery.trackers,
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
                        // Bound queued bytes ahead of a new seek (16 * 16 KiB per peer).
                        // The upstream default of 128 can strand a seek behind 2 MiB.
                        peer_opts: Some(librqbit::PeerConnectionOptions {
                            max_request_window: Some(16),
                            ..Default::default()
                        }),
                        only_files: Some(vec![index]),
                        initial_peers: Some(initial_peers),
                        ..Default::default()
                    }),
                )
                .await
                .context(FailureCode::InvalidMetadata)?
                .into_handle()
                .context(FailureCode::InvalidMetadata)?;
            handle.wait_until_initialized().await?;
            if storage_lease.0.has_failed() {
                bail!("storage policy failed during initialization");
            }
            let (len, content_type, extension) = handle
                .with_metadata(|m| {
                    m.file_infos.get(index).map(|f| {
                        let (mime, ext) =
                            match f.relative_filename.extension().and_then(|s| s.to_str()) {
                                Some(ext) if ext.eq_ignore_ascii_case("mkv") => {
                                    ("video/x-matroska", "mkv")
                                }
                                Some(ext)
                                    if ext.eq_ignore_ascii_case("mp4")
                                        || ext.eq_ignore_ascii_case("m4v") =>
                                {
                                    ("video/mp4", "mp4")
                                }
                                _ => ("application/octet-stream", "bin"),
                            };
                        (f.len, mime, ext)
                    })
                })?
                .context(FailureCode::InvalidMetadata)?;
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            let path = format!("/{}/stream.{extension}", uuid::Uuid::new_v4());
            let router = Router::new()
                .route(&path, get(serve))
                .with_state(StreamState {
                    torrent: handle.clone(),
                    index,
                    len,
                    content_type,
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
                    storage: storage_lease,
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
                if storage_lease.0.has_failed() {
                    return Err(storage_lease
                        .0
                        .limit_failure()
                        .unwrap_or(FailureCode::Read)
                        .into());
                }
                Err(e)
            }
        }
    }
    pub fn snapshot(&self) -> serde_json::Value {
        if self.storage.0.has_failed() {
            return self
                .storage
                .0
                .limit_failure()
                .unwrap_or(FailureCode::Read)
                .snapshot();
        }
        let stats = self.torrent.stats();
        if stats.error.is_some() {
            return FailureCode::Read.snapshot();
        }
        let files = self.torrent.with_metadata(|m| m.file_infos.iter().enumerate().map(|(index, file)| {
            serde_json::json!({"index": index, "path": file.relative_filename.to_string_lossy(), "bytes": file.len})
        }).collect::<Vec<_>>()).unwrap_or_default();
        // This projection is safe for presentation; never expose upstream errors,
        // per-peer addresses, filenames or source URLs through diagnostics.
        let diagnostics = serde_json::json!({
            "cache_allocated_bytes": self.storage.0.cache_allocated_bytes(),
            "engine_state": stats.state.to_string(),
            "live_peers": stats.live.as_ref().map(|live| live.snapshot.peer_stats.live),
            "download_bytes_per_second": stats.live.as_ref().map(|live| live.download_speed.as_bytes()),
            "verified_bytes": stats.progress_bytes,
            "total_bytes": stats.total_bytes,
        });
        serde_json::json!({"state": "ready", "url": self.url,
            "verified_bytes": stats.progress_bytes, "total_bytes": stats.total_bytes,
            "files": files, "diagnostics": diagnostics})
    }

    pub fn downloaded_bytes(&self) -> u64 {
        self.torrent.stats().progress_bytes
    }

    pub async fn stop(self) {
        self.storage.0.close();
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
        self.storage.0.close();
        self.cancel.cancel();
        self.server.abort();
    }
}
