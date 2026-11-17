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
impl Resolved {
    /// Validate an explicit torrent file index before admitting playback.
    /// This does not replace write-time disk and boundary-piece admission checks.
    pub fn selected_file(&self, index: usize) -> Result<&FileChoice> {
        let file = self
            .files
            .iter()
            .find(|file| file.index == index)
            .context(FailureCode::InvalidMetadata)?;
        if file.padding || file.bytes == 0 {
            bail!(FailureCode::UnsupportedFormat);
        }
        if file.bytes > StorageBudget::default().cap_bytes {
            bail!(FailureCode::SourceTooLarge);
        }
        Ok(file)
    }
}

/// Owns discovery until its result is consumed or shutdown is acknowledged.
/// Dropping requests cancellation; callers needing a teardown guarantee must await stop.
pub struct ResolutionSession {
    cancel: CancellationToken,
    task: Option<tokio::task::JoinHandle<Result<Resolved>>>,
}
impl ResolutionSession {
    pub fn start(
        source: TorrentSource,
        output: PathBuf,
        peers: Vec<SocketAddr>,
        discovery: Discovery,
    ) -> Self {
        let cancel = CancellationToken::new();
        let task = tokio::spawn(resolve(source, output, peers, discovery, cancel.clone()));
        Self {
            cancel,
            task: Some(task),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.task.as_ref().is_none_or(|task| task.is_finished())
    }

    pub async fn finish(mut self) -> Result<Resolved> {
        self.task
            .take()
            .expect("owned resolution task")
            .await
            .map_err(|_| anyhow::anyhow!("metadata_task_failed"))?
    }

    pub async fn stop(mut self) -> Result<()> {
        self.cancel.cancel();
        // A cancelled resolution is expected; a failed worker join is not an acknowledgement.
        let _ = self
            .task
            .take()
            .expect("owned resolution task")
            .await
            .map_err(|_| anyhow::anyhow!("metadata_task_failed"))?;
        Ok(())
    }
}
impl Drop for ResolutionSession {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
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
