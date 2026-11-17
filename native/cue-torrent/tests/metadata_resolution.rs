use cue_torrent::{Discovery, TorrentSource, metadata::resolve};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn metadata_listing_keeps_original_indexes_without_materializing_media() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join("fixtures/torrents/multi-episode.torrent")).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    let result = resolve(
        TorrentSource::Bytes(bytes.clone()),
        directory.path().to_owned(),
        vec![],
        Discovery::default(),
        cancel.clone(),
    )
    .await
    .unwrap();
    assert!(
        !cancel.is_cancelled(),
        "resolver completion must not cancel its caller"
    );
    assert_eq!(result.files.len(), 3);
    assert_eq!(result.files[2].index, 2);
    assert!(result.files[2].path.ends_with("tail-index.mp4"));
    assert_eq!(result.files[0].index, 0);
    assert_eq!(result.files[1].index, 1);
    assert!(result.files[1].path.ends_with("episode02.mp4"));
    assert_eq!(result.files[1].bytes, 175924);
    assert_eq!(result.torrent_bytes, bytes);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn cancelled_resolution_does_not_create_files() {
    let directory = tempfile::tempdir().unwrap();
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = resolve(
        TorrentSource::Magnet(
            "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567".into(),
        ),
        directory.path().to_owned(),
        vec![],
        Discovery::default(),
        cancel,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn cancellation_joins_a_pending_peer_metadata_request() {
    let directory = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let cancel = CancellationToken::new();
    let task = tokio::spawn(resolve(
        TorrentSource::Magnet(
            "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567".into(),
        ),
        directory.path().to_owned(),
        vec![address],
        Discovery::default(),
        cancel.clone(),
    ));
    let (_held_socket, _) =
        tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
    cancel.cancel();
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn oversized_metadata_is_rejected_before_parsing_or_storage() {
    let directory = tempfile::tempdir().unwrap();
    let result = resolve(
        TorrentSource::Bytes(vec![0; 16 * 1024 * 1024 + 1]),
        directory.path().to_owned(),
        vec![],
        Discovery::default(),
        CancellationToken::new(),
    )
    .await;
    let error = match result {
        Ok(_) => panic!("oversized metadata accepted"),
        Err(error) => error,
    };
    assert_eq!(
        cue_torrent::failure::FailureCode::from_error(&error),
        cue_torrent::failure::FailureCode::SourceTooLarge
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}
