use cue_core::playback_source::torrent;
use stremio_core::types::resource::Stream;

#[test]
fn torrent_handoff_preserves_unknown_file_and_encodes_trackers() {
    let stream: Stream = serde_json::from_value(serde_json::json!({
        "infoHash":"0123456789abcdef0123456789abcdef01234567",
        "sources":["tracker:https://example.test/announce?token=a&pass=b"]
    }))
    .unwrap();
    let request = torrent(&stream).unwrap();
    assert_eq!(request.file_index, None);
    let url = reqwest::Url::parse(&request.magnet).unwrap();
    let pairs: Vec<_> = url.query_pairs().collect();
    assert_eq!(
        pairs.iter().find(|(key, _)| key == "xt").unwrap().1,
        "urn:btih:0123456789abcdef0123456789abcdef01234567"
    );
    assert_eq!(
        pairs.iter().find(|(key, _)| key == "tr").unwrap().1,
        "https://example.test/announce?token=a&pass=b"
    );
    assert_eq!(pairs.len(), 2);
}

#[test]
fn unsupported_sources_are_not_converted_to_torrents() {
    let stream: Stream =
        serde_json::from_value(serde_json::json!({"externalUrl":"https://example.test/player"}))
            .unwrap();
    assert!(torrent(&stream).is_err());
}

#[test]
fn raw_magnets_require_an_unambiguous_v1_hash() {
    for hash in ["not-a-hash", "", "0123"] {
        let stream: Stream = serde_json::from_value(
            serde_json::json!({"url":format!("magnet:?xt=urn:btih:{hash}")}),
        )
        .unwrap();
        assert!(torrent(&stream).is_err());
    }
    let stream: Stream = serde_json::from_value(
        serde_json::json!({"url":"magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567"}),
    )
    .unwrap();
    assert!(torrent(&stream).is_ok());
}

#[test]
fn explicit_file_selection_and_dht_hints_do_not_become_trackers() {
    let stream: Stream = serde_json::from_value(serde_json::json!({
        "infoHash":"0123456789abcdef0123456789abcdef01234567", "fileIdx":7,
        "fileMustInclude":["episode-03"],
        "sources":["dht:0123456789abcdef0123456789abcdef01234567","tracker:udp://example.test:80/announce"]
    })).unwrap();
    let request = torrent(&stream).unwrap();
    assert_eq!(request.file_index, Some(7));
    assert_eq!(request.file_must_include, ["episode-03"]);
    let url = reqwest::Url::parse(&request.magnet).unwrap();
    let trackers: Vec<_> = url
        .query_pairs()
        .filter(|(key, _)| key == "tr")
        .map(|(_, value)| value.into_owned())
        .collect();
    assert_eq!(trackers, ["udp://example.test:80/announce"]);
}

#[test]
fn duplicate_hashes_and_non_network_trackers_are_rejected() {
    for url in [
        "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&xt=urn:btih:0123456789abcdef0123456789abcdef01234567",
        "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&tr=file:///private/token",
    ] {
        let stream: Stream = serde_json::from_value(serde_json::json!({"url":url})).unwrap();
        assert!(torrent(&stream).is_err());
    }
}
