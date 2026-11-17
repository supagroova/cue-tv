//! Resolve file choices without starting media transfer or creating torrent storage.
use crate::{Discovery, TorrentSource, failure::FailureCode, storage_budget::StorageBudget};
use anyhow::{Context, Result, bail};
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, DhtSessionConfig, Magnet, Session,
    SessionOptions,
};
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use tokio_util::sync::CancellationToken;

#[derive(serde::Serialize)]
pub struct FileChoice {
    pub index: usize,
    pub path: String,
    pub bytes: u64,
    pub padding: bool,
}
pub struct Resolved {
    pub files: Vec<FileChoice>,
    pub torrent_bytes: Vec<u8>,
    pub seen_peers: Vec<SocketAddr>,
}
pub async fn resolve(
    source: TorrentSource,
    output: PathBuf,
    initial_peers: Vec<SocketAddr>,
    discovery: Discovery,
    cancelled: CancellationToken,
) -> Result<Resolved> {
    if cancelled.is_cancelled() {
        bail!("metadata_cancelled");
    }
    let budget = StorageBudget::default();
    let source = match source {
        TorrentSource::Bytes(bytes) => {
            if bytes.len() as u64 > budget.metadata_bytes {
                bail!(FailureCode::SourceTooLarge);
            }
            AddTorrent::from_bytes(bytes)
        }
        TorrentSource::Magnet(value) => {
            if value.len() > 16384 || !value.starts_with("magnet:?") {
                bail!(FailureCode::InvalidMetadata);
            }
            if Magnet::parse(&value)
                .context(FailureCode::InvalidMetadata)?
                .as_id20()
                .is_none()
            {
                bail!(FailureCode::UnsupportedFormat);
            }
            AddTorrent::from_url(value)
        }
    };
    let cancel = cancelled.child_token();
    let _cancel_on_drop = cancel.clone().drop_guard();
    let session = Session::new_with_opts(
        output,
        SessionOptions {
            dht: discovery.dht.then(|| DhtSessionConfig {
                bootstrap_addrs: discovery.bootstrap,
                persistence: None,
                ..Default::default()
            }),
            persistence: None,
            disable_trackers: !discovery.trackers,
            disable_local_service_discovery: true,
            cancellation_token: Some(cancel.clone()),
            peer_limit: Some(30),
            runtime_worker_threads: Some(2),
            ..Default::default()
        },
    )
    .await?;
    let result = tokio::select! { biased;
        _ = cancel.cancelled() => Err(anyhow::anyhow!("metadata_cancelled")),
        result = tokio::time::timeout(Duration::from_secs(30), session.add_torrent(source, Some(AddTorrentOptions { list_only: true, initial_peers: Some(initial_peers), ..Default::default() }))) => {
            match result {
                Ok(result) => result.context(FailureCode::InvalidMetadata),
                Err(_) => Err(anyhow::anyhow!("metadata_timeout")),
            }
        }
    };
    cancel.cancel();
    session.stop().await;
    let AddTorrentResponse::ListOnly(metadata) = result? else {
        bail!(FailureCode::InvalidMetadata);
    };
    if metadata.torrent_bytes.len() as u64 > budget.metadata_bytes {
        bail!(FailureCode::SourceTooLarge);
    }
    let mut files = Vec::new();
    for (index, file) in metadata.info.iter_file_details().enumerate() {
        if index >= budget.max_files {
            bail!(FailureCode::SourceTooLarge);
        }
        if file
            .filename
            .iter_components()
            .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('\0'))
        {
            bail!(FailureCode::InvalidMetadata);
        }
        let path = file.filename.to_string();
        if path.len() > 4096 || file.attrs().symlink || file.symlink_path.is_some() {
            bail!(FailureCode::InvalidMetadata);
        }
        files.push(FileChoice {
            index,
            path,
            bytes: file.len,
            padding: file.attrs().padding,
        });
    }
    Ok(Resolved {
        files,
        torrent_bytes: metadata.torrent_bytes.to_vec(),
        seen_peers: metadata.seen_peers,
    })
}
