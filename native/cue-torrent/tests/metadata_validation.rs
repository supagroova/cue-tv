use cue_torrent::PlaybackSession;
use std::path::PathBuf;

#[tokio::test]
async fn invalid_index_rejects_before_materializing_files() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join("fixtures/torrents/single.torrent")).unwrap();
    let output = tempfile::tempdir().unwrap();
    assert!(
        PlaybackSession::start(bytes, 99, output.path().to_owned(), vec![])
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn empty_or_parent_paths_reject_before_materializing_files() {
    // One-byte file, with the SHA-1 of "x"; only the path varies.
    for name in [".", "..", "/escape", "a/b", "a\\b"] {
        let mut bytes = format!(
            "d4:infod6:lengthi1e4:name{}:{}12:piece lengthi16384e6:pieces20:",
            name.len(),
            name
        )
        .into_bytes();
        bytes.extend_from_slice(&[
            0x11, 0xf6, 0xad, 0x8e, 0xc5, 0x2a, 0x29, 0x84, 0xab, 0xaa, 0xfd, 0x7c, 0x3b, 0x51,
            0x65, 0x03, 0x78, 0x5c, 0x20, 0x72,
        ]);
        bytes.extend_from_slice(b"ee");
        let output = tempfile::tempdir().unwrap();
        assert!(
            PlaybackSession::start(bytes, 0, output.path().to_owned(), vec![])
                .await
                .is_err(),
            "{name}"
        );
        assert_eq!(
            std::fs::read_dir(output.path()).unwrap().count(),
            0,
            "{name}"
        );
    }
}
