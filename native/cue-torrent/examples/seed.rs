//! Local fixture source for simulator development; never required by the app.
use librqbit::{AddTorrent, AddTorrentOptions, ListenerOptions, Session, SessionOptions};
use std::{path::PathBuf, time::Duration};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let long = std::env::var_os("CUE_LONG_FIXTURE").is_some();
    let session = Session::new_with_opts(
        root.join("apple-tv/Resources/PlaybackLab"),
        SessionOptions {
            ratelimits: librqbit::limits::LimitsConfig {
                upload_bps: if long {
                    std::num::NonZeroU32::new(65536)
                } else {
                    None
                },
                download_bps: None,
            },
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
    if long {
        let directory = root.join(".agent-work/long-playback");
        let torrent = std::fs::read(directory.join("long.torrent"))?;
        let handle = session
            .add_torrent(
                AddTorrent::from_bytes(torrent),
                Some(AddTorrentOptions {
                    overwrite: true,
                    output_folder: Some(directory.to_string_lossy().into_owned()),
                    ..Default::default()
                }),
            )
            .await?
            .into_handle()
            .unwrap();
        handle.wait_until_completed().await?;
    }
    println!("Fixture peer ready at 127.0.0.1:42424");
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}
