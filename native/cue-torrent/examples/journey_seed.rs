//! Owned multi-episode peer for the opt-in full UI journey; no public discovery.
use librqbit::{AddTorrent, AddTorrentOptions, ListenerOptions, Session, SessionOptions};
use std::{path::PathBuf, time::Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or_else(|| anyhow::anyhow!("owned output directory required"))?,
    );
    let files = directory.as_path().join("CueTV-fixtures");
    std::fs::create_dir(&files)?;
    for name in ["episode01.mp4", "episode02.mp4", "tail-index.mp4"] {
        std::fs::copy(
            root.join("apple-tv/Resources/PlaybackLab").join(name),
            files.join(name),
        )?;
    }
    let session = Session::new_with_opts(
        directory.as_path().to_owned(),
        SessionOptions {
            dht: None,
            disable_trackers: true,
            disable_local_service_discovery: true,
            listen: Some(ListenerOptions {
                listen_addr: "127.0.0.1:42428".parse()?,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await?;
    let torrent = std::fs::read(root.join("fixtures/torrents/multi-episode.torrent"))?;
    let handle = session
        .add_torrent(
            AddTorrent::from_bytes(torrent),
            Some(AddTorrentOptions {
                overwrite: true,
                ..Default::default()
            }),
        )
        .await?
        .into_handle()
        .ok_or_else(|| anyhow::anyhow!("fixture handle missing"))?;
    handle.wait_until_completed().await?;
    println!("Journey peer ready at 127.0.0.1:42428");
    tokio::time::sleep(Duration::from_secs(600)).await;
    session.stop().await;
    Ok(())
}
