use cue_core::subtitles::choices;
use stremio_core::types::resource::Stream;

#[test]
fn subtitles_keep_signed_urls_private_and_bound_unsupported_tracks() {
    let stream: Stream = serde_json::from_value(serde_json::json!({
        "url":"https://video.example/movie.mp4", "subtitles":[
            {"id":"english", "lang":"eng", "url":"https://captions.example/a.vtt?token=owned"},
            {"id":"local", "lang":"eng", "url":"file:///private/a.vtt"},
            {"id":"credentials", "lang":"eng", "url":"https://user:password@captions.example/a.vtt"},
            {"id":"fragment", "lang":"eng", "url":"https://captions.example/a.vtt#secret"},
            {"id":"font-dependent", "lang":"eng", "url":"https://captions.example/a.ass", "fonts":["https://captions.example/font.ttf"]}
        ]
    })).unwrap();
    let json = serde_json::to_value(choices(&stream.subtitles)).unwrap();
    assert_eq!(
        json,
        serde_json::json!([{ "id":"english", "language":"eng", "label":null, "url":"https://captions.example/a.vtt?token=owned" }])
    );
    let mut tracks = stream.subtitles.clone();
    tracks = std::iter::repeat_n(tracks.remove(0), 100).collect();
    assert_eq!(choices(&tracks).len(), 32);
}

#[test]
fn discovery_plans_only_compatible_installed_providers_and_bounds_fanout() {
    use cue_core::subtitles::plan;
    use stremio_core::types::addon::Descriptor;
    let provider = |resource: &str, kind: &str, prefix: &str| -> Descriptor {
        serde_json::from_value(serde_json::json!({
            "transportUrl":"https://captions.example/manifest.json",
            "manifest":{"id":"owned.subtitles","name":"Owned","version":"1.0.0",
                "description":"Owned fixture","resources":[{"name":resource,"types":[kind],"idPrefixes":[prefix]}],
                "types":[kind],"catalogs":[]}
        })).unwrap()
    };
    let compatible = provider("subtitles", "series", "tt");
    let addons = vec![
        provider("stream", "series", "tt"),
        provider("subtitles", "movie", "tt"),
        provider("subtitles", "series", "other"),
        compatible.clone(),
    ];
    let requests = plan(&addons, "series", "tt123:2:3").unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path.resource, "subtitles");
    assert_eq!(requests[0].path.r#type, "series");
    assert_eq!(requests[0].path.id, "tt123:2:3");
    assert!(
        requests[0].path.extra.is_empty(),
        "Do not invent video hashes or sizes"
    );
    assert_eq!(
        plan(&vec![compatible; 100], "series", "tt123:2:3")
            .unwrap()
            .len(),
        32
    );
    for invalid in ["".to_owned(), "tt123\n".to_owned(), "x".repeat(2049)] {
        assert!(plan(&addons, "series", &invalid).is_err());
    }
}

#[tokio::test]
async fn discovery_isolates_provider_failure_and_deduplicates_private_results() {
    use cue_core::{environment::Environment, subtitles::discover};
    use stremio_core::types::addon::{ResourcePath, ResourceRequest};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..3 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = [0; 4096];
            let count = socket.read(&mut bytes).await.unwrap();
            let request = std::str::from_utf8(&bytes[..count]).unwrap();
            assert!(
                request.contains("/subtitles/series/tt123%3A2%3A3.json ")
                    || request.contains("/subtitles/series/tt123:2:3.json ")
            );
            let body = if request.starts_with("GET /bad/") {
                "invalid json"
            } else {
                r#"{"subtitles":[{"id":"owned","lang":"eng","url":"https://captions.example/a.vtt?private=token"}]}"#
            };
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });
    let requests = ["good", "bad", "duplicate"].map(|name| ResourceRequest {
        base: format!("http://{address}/{name}/manifest.json")
            .parse()
            .unwrap(),
        path: ResourcePath::without_extra("subtitles", "series", "tt123:2:3"),
    });
    let env = Environment::new().unwrap();
    let result = discover(
        env.clone(),
        requests.to_vec(),
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(result.tracks.len(), 1);
    assert_eq!(
        result.tracks[0].url,
        "https://captions.example/a.vtt?private=token"
    );
    assert_eq!(result.failed_providers, 1);
    server.await.unwrap();
    env.shutdown().await;
}

#[tokio::test]
async fn cancel_discovery_drops_a_blocked_http_request() {
    use cue_core::{environment::Environment, subtitles::discover};
    use stremio_core::types::addon::{ResourcePath, ResourceRequest};
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (sent, received) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = [0; 4096];
        assert!(socket.read(&mut bytes).await.unwrap() > 0);
        sent.send(()).unwrap();
        assert_eq!(socket.read(&mut bytes).await.unwrap(), 0);
    });
    let env = Environment::new().unwrap();
    let cancel = tokio_util::sync::CancellationToken::new();
    let query = tokio::spawn(discover(
        env.clone(),
        vec![ResourceRequest {
            base: format!("http://{address}/manifest.json").parse().unwrap(),
            path: ResourcePath::without_extra("subtitles", "series", "tt123:2:3"),
        }],
        cancel.clone(),
    ));
    received.await.unwrap();
    cancel.cancel();
    let result = tokio::time::timeout(std::time::Duration::from_secs(1), query)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(result, Err("subtitle_discovery_cancelled")));
    tokio::time::timeout(std::time::Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
    env.shutdown().await;
}

#[tokio::test]
async fn discovery_deadline_bounds_fanout_and_counts_unfinished_providers() {
    use cue_core::{environment::Environment, subtitles::discover};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use stremio_core::types::addon::{ResourcePath, ResourceRequest};
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let observed = accepted.clone();
    let server = tokio::spawn(async move {
        let mut sockets = tokio::task::JoinSet::new();
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            observed.fetch_add(1, Ordering::SeqCst);
            sockets.spawn(async move {
                let mut bytes = [0; 4096];
                let _ = socket.read(&mut bytes).await;
                let _ = socket.read(&mut bytes).await;
            });
        }
    });
    let request = ResourceRequest {
        base: format!("http://{address}/manifest.json").parse().unwrap(),
        path: ResourcePath::without_extra("subtitles", "series", "tt123:2:3"),
    };
    let env = Environment::new().unwrap();
    let query = tokio::spawn(discover(
        env.clone(),
        vec![request; 12],
        tokio_util::sync::CancellationToken::new(),
    ));
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        while accepted.load(Ordering::SeqCst) < 4 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        4,
        "Only four blocked providers may start concurrently"
    );
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), query)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(result.tracks.is_empty());
    assert_eq!(
        result.failed_providers, 12,
        "Queued and active providers must all count on deadline"
    );
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        8,
        "Total deadline stops the second batch before the third starts"
    );
    server.abort();
    let _ = server.await;
    env.shutdown().await;
}

#[tokio::test]
async fn session_rejects_subtitles_from_a_previous_selection_of_the_same_video() {
    use cue_core::{
        environment::{Environment, NativeEnv},
        session::CoreSession,
    };
    use stremio_core::{
        constants::{PROFILE_STORAGE_KEY, SCHEMA_VERSION, SCHEMA_VERSION_STORAGE_KEY},
        runtime::Env,
        types::profile::Profile,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (received, waiting) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let mut received = Some(received);
        let mut released = Some(released);
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = [0; 4096];
            let count = socket.read(&mut bytes).await.unwrap();
            let path = std::str::from_utf8(&bytes[..count])
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap();
            let body = match path {
                "/catalog/movie/owned.json" => {
                    r#"{"metas":[{"id":"owned","type":"movie","name":"Owned"}]}"#
                }
                "/meta/movie/owned.json" => {
                    r#"{"meta":{"id":"owned","type":"movie","name":"Owned","videos":[]}}"#
                }
                "/stream/movie/owned.json" => {
                    r#"{"streams":[{"url":"https://fixture.test/video.mp4"}]}"#
                }
                "/subtitles/movie/owned.json" => {
                    received.take().unwrap().send(()).unwrap();
                    released.take().unwrap().await.unwrap();
                    r#"{"subtitles":[{"id":"old-generation","lang":"eng","url":"https://fixture.test/old.vtt"}]}"#
                }
                other => panic!("Unexpected fixture resource: {other}"),
            };
            let _ = socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .as_bytes(),
                )
                .await;
        }
    });
    let environment = Environment::new().unwrap();
    let mut profile = Profile::default();
    profile.addons = vec![serde_json::from_value(serde_json::json!({
        "transportUrl":format!("http://{address}/manifest.json"),
        "manifest":{"id":"owned","name":"Owned","version":"1.0.0","description":"Owned fixture",
            "resources":["catalog","meta","stream","subtitles"],"types":["movie"],"catalogs":[{"id":"owned","type":"movie"}]}
    })).unwrap()];
    environment
        .scope(async {
            NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
                .await
                .unwrap();
            NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
                .await
                .unwrap();
        })
        .await;
    let mut session = CoreSession::guest(environment).await.unwrap();
    session
        .load_catalog(0, session.snapshot().revision)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    session
        .load_details(0, session.snapshot().revision)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    session
        .load_sources(None, session.snapshot().revision)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    let (result, ()) = tokio::join!(
        session.discover_subtitles(
            "owned",
            "owned",
            "movie",
            tokio_util::sync::CancellationToken::new()
        ),
        async {
            tokio::time::timeout(std::time::Duration::from_secs(2), waiting)
                .await
                .unwrap()
                .unwrap();
            // Same title/type/video: only the generation distinguishes this selection.
            session
                .load_sources(None, session.snapshot().revision)
                .await
                .unwrap();
            release.send(()).unwrap();
        }
    );
    assert!(matches!(result, Err("stale_subtitle_identity")));
    session.stop().await;
    server.abort();
    let _ = server.await;
}
