use cue_torrent::{Discovery, PlaybackSession, TorrentSource};
use librqbit::{AddTorrent, AddTorrentOptions, ListenerOptions, Session, SessionOptions};
use std::{path::PathBuf, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn peer_bytes_ranges_and_teardown() {
    check_fixture("single", 0, false, "direct").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn magnet_resolves_metadata_from_peer() {
    check_fixture("single", 0, true, "direct").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multifile_selects_second_episode() {
    check_fixture("multi-episode", 1, true, "direct").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tracker_discovers_peer_without_initial_addresses() {
    check_fixture("single", 0, true, "http").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn udp_tracker_discovers_peer_with_dht_disabled() {
    check_fixture("single", 0, true, "udp").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dht_discovers_peer_with_trackers_disabled() {
    check_fixture("single", 0, true, "dht").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mkv_peer_bytes_and_content_type() {
    check_fixture("mkv", 0, false, "direct").await;
}

async fn check_fixture(name: &str, index: usize, magnet: bool, protocol: &str) {
    if std::env::var_os("RUST_LOG").is_some() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    }
    let tracker = protocol != "direct";
    tokio::time::timeout(Duration::from_secs(45), async {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let torrent =
            std::fs::read(root.join(format!("fixtures/torrents/{name}.torrent"))).unwrap();
        let filename = if name == "mkv" {
            "episode01.mkv".to_owned()
        } else {
            format!("episode0{}.mp4", index + 1)
        };
        let bytes =
            std::fs::read(root.join("apple-tv/Resources/PlaybackLab").join(&filename)).unwrap();
        let seed_dir = tempfile::tempdir().unwrap();
        let seed_files = if name == "multi-episode" {
            seed_dir.path().join("CueTV-fixtures")
        } else {
            seed_dir.path().to_owned()
        };
        std::fs::create_dir_all(&seed_files).unwrap();
        for file in [
            "episode01.mp4",
            "episode02.mp4",
            "tail-index.mp4",
            "episode01.mkv",
        ] {
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
        let udp = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let udp_address = udp.local_addr().unwrap();
        let udp_requests = announcements.clone();
        let udp_task = tokio::spawn(async move {
            let mut packet = [0u8; 2048];
            loop {
                let (length, sender) = udp.recv_from(&mut packet).await.unwrap();
                if length < 16 {
                    continue;
                }
                let action = u32::from_be_bytes(packet[8..12].try_into().unwrap());
                let mut reply = action.to_be_bytes().to_vec();
                reply.extend_from_slice(&packet[12..16]);
                match action {
                    0 => reply.extend_from_slice(&1u64.to_be_bytes()),
                    1 if length >= 98 => {
                        udp_requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        reply.extend_from_slice(&60u32.to_be_bytes());
                        reply.extend_from_slice(&0u32.to_be_bytes());
                        reply.extend_from_slice(&1u32.to_be_bytes());
                        reply.extend_from_slice(&[127, 0, 0, 1]);
                        reply.extend_from_slice(&address.port().to_be_bytes());
                    }
                    _ => continue,
                }
                udp.send_to(&reply, sender).await.unwrap();
            }
        });
        // KRPC fixture: return a routing node, then the peer only via get_peers.
        let dht_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let dht_address = dht_socket.local_addr().unwrap();
        let dht_queries = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let query_count = dht_queries.clone();
        let dht_task = tokio::spawn(async move {
            #[derive(serde::Deserialize)]
            struct Query {
                t: bencode::ByteBufOwned,
                q: bencode::ByteBufOwned,
            }
            let mut packet = [0u8; 4096];
            loop {
                let (length, sender) = dht_socket.recv_from(&mut packet).await.unwrap();
                let Ok(query) = bencode::from_bytes::<Query>(&packet[..length]) else {
                    continue;
                };
                let mut reply = b"d1:rd2:id20:01234567890123456789".to_vec();
                if query.q.as_ref() == b"get_peers" {
                    query_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    reply.extend_from_slice(b"5:token4:test6:valuesl6:");
                    reply.extend_from_slice(&[127, 0, 0, 1]);
                    reply.extend_from_slice(&address.port().to_be_bytes());
                    reply.push(b'e');
                } else {
                    reply.extend_from_slice(b"5:nodes26:01234567890123456789");
                    reply.extend_from_slice(&[127, 0, 0, 1]);
                    reply.extend_from_slice(&dht_address.port().to_be_bytes());
                }
                reply.extend_from_slice(format!("e1:t{}:", query.t.as_ref().len()).as_bytes());
                reply.extend_from_slice(query.t.as_ref());
                reply.extend_from_slice(b"1:y1:re");
                dht_socket.send_to(&reply, sender).await.unwrap();
            }
        });
        let tracker_url = if protocol == "udp" {
            format!("udp://{udp_address}/announce")
        } else {
            format!("http://{tracker_address}/announce")
        };
        let download = tempfile::tempdir().unwrap();
        let playback = PlaybackSession::start_source(
            if magnet {
                TorrentSource::Magnet(format!(
                    "magnet:?xt=urn:btih:{}&tr={tracker_url}",
                    seed.info_hash().as_string()
                ))
            } else {
                TorrentSource::Bytes(torrent)
            },
            index,
            download.path().to_owned(),
            if tracker { vec![] } else { vec![address] },
            Discovery {
                trackers: tracker && protocol != "dht",
                dht: protocol == "dht",
                bootstrap: Some(vec![dht_address.to_string()]),
            },
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
        assert_eq!(
            head.headers()["content-type"],
            if name == "mkv" {
                "video/x-matroska"
            } else {
                "video/mp4"
            }
        );
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
        if tracker && protocol != "dht" {
            assert!(announcements.load(std::sync::atomic::Ordering::Relaxed) > 0);
        }
        tracker_task.abort();
        udp_task.abort();
        if protocol == "dht" {
            assert!(dht_queries.load(std::sync::atomic::Ordering::Relaxed) > 0);
        }
        dht_task.abort();
        seeder.stop().await;
    })
    .await
    .expect("peer stream must complete within deadline");
}
