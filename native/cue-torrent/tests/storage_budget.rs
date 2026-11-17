use cue_torrent::{
    Discovery, PlaybackSession, TorrentSource,
    storage_budget::{DiskUsage, StorageBudget},
};
use std::{path::PathBuf, sync::Arc};

#[tokio::test]
async fn oversized_torrent_is_rejected_before_creating_files() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let torrent = std::fs::read(root.join("fixtures/torrents/single.torrent")).unwrap();
    let output = tempfile::tempdir().unwrap();
    let budget = StorageBudget {
        cap_bytes: 100_000,
        reserve_bytes: 0,
        metadata_bytes: 4096,
        max_files: 8,
    };
    let result = PlaybackSession::start_budgeted(
        TorrentSource::Bytes(torrent),
        0,
        output.path().to_owned(),
        vec![],
        Discovery::default(),
        budget,
        None,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn reserve_is_preserved_without_filling_real_disk() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let torrent = std::fs::read(root.join("fixtures/torrents/single.torrent")).unwrap();
    let output = tempfile::tempdir().unwrap();
    let budget = StorageBudget {
        cap_bytes: 1_000_000,
        reserve_bytes: 800_000,
        metadata_bytes: 4096,
        max_files: 8,
    };
    let disk = Arc::new(|_: &std::path::Path| {
        Ok(DiskUsage {
            available: 900_000,
            allocated: 0,
            metadata_allocated: 0,
            block_size: 4096,
        })
    });
    let result = PlaybackSession::start_budgeted(
        TorrentSource::Bytes(torrent),
        0,
        output.path().to_owned(),
        vec![],
        Discovery::default(),
        budget,
        Some(disk),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disk_exhaustion_after_admission_blocks_later_writes() {
    use librqbit::{AddTorrent, AddTorrentOptions, ListenerOptions, Session, SessionOptions};
    use std::{
        os::unix::fs::MetadataExt,
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let torrent = std::fs::read(root.join("fixtures/torrents/single.torrent")).unwrap();
        let seed_dir = tempfile::tempdir().unwrap();
        std::fs::copy(
            root.join("apple-tv/Resources/PlaybackLab/episode01.mp4"),
            seed_dir.path().join("episode01.mp4"),
        )
        .unwrap();
        let reserve = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = reserve.local_addr().unwrap();
        drop(reserve);
        let seed = Session::new_with_opts(
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
        seed.add_torrent(
            AddTorrent::from_bytes(torrent.clone()),
            Some(AddTorrentOptions {
                overwrite: true,
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_handle()
        .unwrap()
        .wait_until_completed()
        .await
        .unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let disk = Arc::new(move |path: &std::path::Path| {
            let call = observed.fetch_add(1, Ordering::SeqCst);
            let allocated = std::fs::metadata(path.join("episode01.mp4"))
                .map(|m| m.blocks() * 512)
                .unwrap_or(0);
            Ok(DiskUsage {
                available: if call < 3 { 1_000_000 } else { 0 },
                allocated,
                metadata_allocated: 0,
                block_size: 4096,
            })
        });
        let output = tempfile::tempdir().unwrap();
        let budget = StorageBudget {
            cap_bytes: 1_000_000,
            reserve_bytes: 0,
            metadata_bytes: 4096,
            max_files: 8,
        };
        let session = PlaybackSession::start_budgeted(
            TorrentSource::Bytes(torrent),
            0,
            output.path().to_owned(),
            vec![address],
            Discovery::default(),
            budget,
            Some(disk),
        )
        .await
        .unwrap();
        while session.snapshot()["state"] != "failed" {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            calls.load(Ordering::SeqCst) >= 4,
            "must observe write-time failure after admission"
        );
        let allocated = std::fs::metadata(output.path().join("episode01.mp4"))
            .unwrap()
            .blocks()
            * 512;
        assert!(
            allocated <= 176128,
            "allocation must stay within the admitted file bound: {allocated}"
        );
        let stored = std::fs::read(output.path().join("episode01.mp4")).unwrap();
        assert!(
            stored[..16384].iter().any(|b| *b != 0),
            "first permitted block was written"
        );
        assert!(
            stored[16384..].iter().all(|b| *b == 0),
            "blocked writes must not change file content"
        );
        assert!(session.downloaded_bytes() < 175917);
        session.stop().await;
        seed.stop().await;
    })
    .await
    .expect("quota failure must be reported promptly");
}

#[tokio::test]
async fn unselected_files_and_boundary_storage_count_toward_budget() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let torrent = std::fs::read(root.join("fixtures/torrents/multi-episode.torrent")).unwrap();
    let output = tempfile::tempdir().unwrap();
    // The selected episode is 175,924 bytes, but the complete torrent exceeds 500 KB.
    let budget = StorageBudget {
        cap_bytes: 250_000,
        reserve_bytes: 0,
        metadata_bytes: 4096,
        max_files: 8,
    };
    assert!(
        PlaybackSession::start_budgeted(
            TorrentSource::Bytes(torrent),
            1,
            output.path().to_owned(),
            vec![],
            Discovery::default(),
            budget,
            None
        )
        .await
        .is_err()
    );
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn existing_directory_allocation_counts_before_file_creation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let torrent = std::fs::read(root.join("fixtures/torrents/single.torrent")).unwrap();
    let output = tempfile::tempdir().unwrap();
    let disk = Arc::new(|_: &std::path::Path| {
        Ok(DiskUsage {
            available: 1_000_000,
            allocated: 0,
            metadata_allocated: 100_000,
            block_size: 4096,
        })
    });
    let result = PlaybackSession::start_budgeted(
        TorrentSource::Bytes(torrent),
        0,
        output.path().to_owned(),
        vec![],
        Discovery::default(),
        StorageBudget {
            cap_bytes: 250_000,
            reserve_bytes: 0,
            metadata_bytes: 4096,
            max_files: 8,
        },
        Some(disk),
    )
    .await;
    if let Ok(session) = result {
        session.stop().await;
    }
    assert_eq!(
        std::fs::read_dir(output.path()).unwrap().count(),
        0,
        "over-budget directory metadata must reject admission before creating files"
    );
}

#[tokio::test]
async fn closed_storage_rejects_late_initialization_without_creating_files() {
    use cue_torrent::storage_budget::BudgetFactory;
    use librqbit::{AddTorrent, Session, SessionOptions, storage::StorageFactoryExt};
    let output = tempfile::tempdir().unwrap();
    let factory = BudgetFactory::new(output.path().to_owned(), StorageBudget::default(), None);
    factory.close();
    let session = Session::new_with_opts(
        output.path().to_owned(),
        SessionOptions {
            default_storage_factory: Some(factory.boxed()),
            dht: None,
            disable_trackers: true,
            disable_local_service_discovery: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let torrent = std::fs::read(root.join("fixtures/torrents/single.torrent")).unwrap();
    let result = session
        .add_torrent(AddTorrent::from_bytes(torrent), None)
        .await;
    assert!(
        result.is_err(),
        "closed storage must reject late initialization"
    );
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
    session.stop().await;
}
