use cue_torrent::PlaybackSession;
use librqbit::{
    AddTorrent, AddTorrentOptions, CreateTorrentOptions, ListenerOptions, Session, SessionOptions,
    create_torrent, limits::LimitsConfig, spawn_utils::BlockingSpawner,
};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    num::NonZeroU32,
    time::Duration,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn uncached_seek_and_stop_interrupts_pending_body() {
    tokio::time::timeout(Duration::from_secs(45), async {
        let seed_dir = tempfile::tempdir().unwrap();
        let file = seed_dir.path().join("large.bin");
        let mut writer = std::fs::File::create(&file).unwrap();
        for piece in 0..1024u32 {
            writer.write_all(&vec![(piece % 251) as u8; 65536]).unwrap();
        }
        drop(writer);
        let metainfo = create_torrent(
            &file,
            CreateTorrentOptions {
                piece_length: Some(65536),
                ..Default::default()
            },
            &BlockingSpawner::new(2),
        )
        .await
        .unwrap();
        let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = reservation.local_addr().unwrap();
        drop(reservation);
        let seed = Session::new_with_opts(
            seed_dir.path().to_owned(),
            SessionOptions {
                dht: None,
                disable_trackers: true,
                disable_local_service_discovery: true,
                ratelimits: LimitsConfig {
                    upload_bps: NonZeroU32::new(65536),
                    download_bps: None,
                },
                listen: Some(ListenerOptions {
                    listen_addr: address,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let handle = seed
            .add_torrent(
                AddTorrent::from_bytes(metainfo.as_bytes().unwrap()),
                Some(AddTorrentOptions {
                    overwrite: true,
                    ..Default::default()
                }),
            )
            .await
            .unwrap()
            .into_handle()
            .unwrap();
        handle.wait_until_completed().await.unwrap();
        let download = tempfile::tempdir().unwrap();
        let playback = PlaybackSession::start(
            metainfo.as_bytes().unwrap().to_vec(),
            0,
            download.path().to_owned(),
            vec![address],
        )
        .await
        .unwrap();
        let client = reqwest::Client::new();
        for piece in [1023u64, 512, 0] {
            assert!(
                playback.downloaded_bytes() < 16 * 1024 * 1024,
                "fixture must remain incomplete"
            );
            let offset = piece * 65536;
            if piece != 0 {
                let mut stored = std::fs::File::open(download.path().join("large.bin")).unwrap();
                stored.seek(SeekFrom::Start(offset)).unwrap();
                let mut before = vec![0; 65536];
                stored.read_exact(&mut before).unwrap();
                assert_ne!(
                    before,
                    vec![(piece % 251) as u8; 65536],
                    "requested piece must not already be fully cached"
                );
            }
            let response = client
                .get(&playback.url)
                .header("Range", format!("bytes={offset}-{}", offset + 1023))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), 206);
            let bytes = tokio::time::timeout(Duration::from_secs(15), response.bytes())
                .await
                .expect("an uncached range must not wait behind a full peer request backlog")
                .unwrap();
            assert_eq!(bytes.as_ref(), &vec![(piece % 251) as u8; 1024]);
        }
        let response = client
            .get(&playback.url)
            .header("Range", "bytes=16777216-")
            .send()
            .await
            .unwrap();
        let body = tokio::spawn(async move { response.bytes().await });
        tokio::time::timeout(Duration::from_secs(2), playback.stop())
            .await
            .expect("stop deadline");
        let result = tokio::time::timeout(Duration::from_secs(2), body)
            .await
            .expect("blocked HTTP body must terminate")
            .unwrap();
        assert!(
            result.is_err(),
            "an incomplete response must not report success"
        );
        seed.stop().await;
    })
    .await
    .expect("seek proof deadline");
}
