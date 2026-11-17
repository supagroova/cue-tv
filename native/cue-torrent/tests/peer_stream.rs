use cue_torrent::{PlaybackSession, TorrentSource};
use librqbit::{AddTorrent, AddTorrentOptions, ListenerOptions, Session, SessionOptions};
use std::{path::PathBuf, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peer_bytes_ranges_and_teardown() {
    check_fixture("single", 0, false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn magnet_resolves_metadata_from_peer() {
    check_fixture("single", 0, true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multifile_selects_second_episode() {
    check_fixture("multi-episode", 1, true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tracker_discovers_peer_without_initial_addresses() {
    check_fixture("single", 0, true, true).await;
}

async fn check_fixture(name: &str, index: usize, magnet: bool, tracker: bool) {
    tokio::time::timeout(Duration::from_secs(45), async {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let torrent =
            std::fs::read(root.join(format!("fixtures/torrents/{name}.torrent"))).unwrap();
        let bytes = std::fs::read(root.join(format!(
            "apple-tv/Resources/PlaybackLab/episode0{}.mp4",
            index + 1
        )))
        .unwrap();
        let seed_dir = tempfile::tempdir().unwrap();
        let seed_files = if name == "multi-episode" {
            seed_dir.path().join("CueTV-fixtures")
        } else {
            seed_dir.path().to_owned()
        };
        std::fs::create_dir_all(&seed_files).unwrap();
        for file in ["episode01.mp4", "episode02.mp4", "tail-index.mp4"] {
            std::fs::copy(
                root.join("apple-tv/Resources/PlaybackLab").join(file),
                seed_files.join(file),
            )
            .unwrap();
        }
        let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = reservation.local_addr().unwrap();
        drop(reservation);
        let seeder = Session::new_with_opts(
            seed_dir.path().to_owned(),
            SessionOptions {
                dht: None,
                disable_trackers: true,
                disable_local_service_discovery: true,
                listen: Some(ListenerOptions {
                    listen_addr: address,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let seed = seeder
            .add_torrent(
                AddTorrent::from_bytes(torrent.clone()),
                Some(AddTorrentOptions {
                    overwrite: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap()
            .into_handle()
            .unwrap();
        seed.wait_until_completed().await.unwrap();
        let tracker_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tracker_address = tracker_listener.local_addr().unwrap();
        let mut reply = b"d8:intervali60e5:peers6:".to_vec();
        reply.extend_from_slice(&[127, 0, 0, 1]);
        reply.extend_from_slice(&address.port().to_be_bytes());
        reply.push(b'e');
        let announcements = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let requests = announcements.clone();
        let tracker_task = tokio::spawn(async move {
            let app = axum::Router::new().route(
                "/announce",
                axum::routing::get(move || {
                    let reply = reply.clone();
                    let requests = requests.clone();
                    async move {
                        requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        reply
                    }
                }),
            );
            axum::serve(tracker_listener, app).await.unwrap();
        });
        let download = tempfile::tempdir().unwrap();
        let playback = PlaybackSession::start_source(
            if magnet {
                TorrentSource::Magnet(format!(
                    "magnet:?xt=urn:btih:{}&tr=http://{tracker_address}/announce",
                    seed.info_hash().as_string()
                ))
            } else {
                TorrentSource::Bytes(torrent)
            },
            index,
            download.path().to_owned(),
            if tracker { vec![] } else { vec![address] },
            tracker,
        )
        .await
        .unwrap();
        let client = reqwest::Client::new();
        let tail = client
            .get(&playback.url)
            .header("Range", "bytes=-1024")
            .send()
            .await
            .unwrap();
        assert_eq!(tail.status(), 206);
        assert_eq!(
            tail.headers()["content-range"],
            format!(
                "bytes {}-{}/{}",
                bytes.len() - 1024,
                bytes.len() - 1,
                bytes.len()
            )
        );
        assert_eq!(
            tail.bytes().await.unwrap().as_ref(),
            &bytes[bytes.len() - 1024..]
        );
        let head = client.head(&playback.url).send().await.unwrap();
        assert_eq!(head.status(), 200);
        assert_eq!(head.headers()["content-length"], bytes.len().to_string());
        let bad = client
            .get(&playback.url)
            .header("Range", "bytes=999999-")
            .send()
            .await
            .unwrap();
        assert_eq!(bad.status(), 416);
        let all = client
            .get(&playback.url)
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(all.as_ref(), bytes.as_slice());
        let url = playback.url.clone();
        playback.stop().await;
        assert!(client.get(url).send().await.is_err());
        if tracker {
            assert!(announcements.load(std::sync::atomic::Ordering::Relaxed) > 0);
        }
        tracker_task.abort();
        seeder.stop().await;
    })
    .await
    .expect("peer stream must complete within deadline");
}
