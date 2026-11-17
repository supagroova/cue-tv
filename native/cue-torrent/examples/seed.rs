//! Local fixture source for simulator development; never required by the app.
use librqbit::{AddTorrent, AddTorrentOptions, ListenerOptions, Session, SessionOptions};
use std::{path::PathBuf, time::Duration};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let session = Session::new_with_opts(
        root.join("apple-tv/Resources/PlaybackLab"),
        SessionOptions {
            dht: None,
            disable_trackers: true,
            disable_local_service_discovery: true,
            listen: Some(ListenerOptions {
                listen_addr: "127.0.0.1:42424".parse()?,
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await?;
    let torrent = std::fs::read(root.join("fixtures/torrents/single.torrent"))?;
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
        .unwrap();
    handle.wait_until_completed().await?;
    println!("Fixture peer ready at 127.0.0.1:42424");
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
