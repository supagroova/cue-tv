use cue_core::catalog::{Catalog, view};
use stremio_core::models::{
    catalog_with_filters::Selected,
    common::{Loadable, ResourceError, ResourceLoadable},
};

#[test]
fn an_empty_catalogue_is_a_successful_empty_result() {
    let selected: Selected = serde_json::from_value(serde_json::json!({"request":{"base":"https://example.test/manifest.json","path":{"resource":"catalog","type":"movie","id":"fixture","extra":[]}}})).unwrap();
    let mut catalog = Catalog::default();
    catalog.catalog.push(ResourceLoadable {
        request: selected.request.clone(),
        content: Some(Loadable::Err(ResourceError::EmptyContent)),
    });
    catalog.selected = Some(selected);
    let state = view(&catalog, 0).unwrap();
    assert_eq!(state.status, "ready");
    assert!(state.items.is_empty());
    assert!(!state.has_more);
}

#[tokio::test]
async fn all_installed_catalogues_are_reachable_through_bounded_choice_windows() {
    use cue_core::{
        environment::{Environment, NativeEnv},
        session::CoreSession,
    };
    use stremio_core::{
        constants::{PROFILE_STORAGE_KEY, SCHEMA_VERSION, SCHEMA_VERSION_STORAGE_KEY},
        runtime::Env,
        types::profile::Profile,
    };
    let env = Environment::new().unwrap();
    let mut profile = Profile::default();
    profile.addons.truncate(1);
    let template = profile.addons[0].manifest.catalogs[0].clone();
    profile.addons[0].manifest.catalogs = (0..35)
        .map(|index| {
            let mut catalog = template.clone();
            catalog.id = format!("catalog-{index}");
            catalog.name = Some(format!("Catalogue {index}"));
            catalog
        })
        .collect();
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
            .await
            .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
    })
    .await;
    let mut session = CoreSession::guest(env).await.unwrap();
    let first = session.snapshot();
    assert_eq!(first.catalog_total, 35);
    assert_eq!(first.catalogs.len(), 16);
    session
        .show_catalog_choices(16, first.revision)
        .await
        .unwrap();
    let second = session.snapshot();
    assert_eq!(second.catalog_choices_offset, 16);
    assert_eq!(second.catalogs[0].index, 16);
    assert_eq!(second.catalogs[0].name, "Catalogue 16");
    session
        .show_catalog_choices(32, second.revision)
        .await
        .unwrap();
    let last = session.snapshot();
    assert_eq!(last.catalogs.len(), 3);
    assert_eq!(last.catalogs[2].index, 34);
    assert!(
        session
            .show_catalog_choices(35, last.revision)
            .await
            .is_err()
    );
    assert!(
        session
            .show_catalog_choices(0, first.revision)
            .await
            .is_err()
    );
    session
        .show_catalog_choices(0, last.revision)
        .await
        .unwrap();
    assert_eq!(session.snapshot().catalogs[0].index, 0);
    session.stop().await;
}

#[tokio::test]
async fn content_windows_fetch_remote_pages_without_gaps_or_duplicates() {
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
    let (first_received, first_wait) = tokio::sync::oneshot::channel();
    let (release_fresh, fresh_wait) = tokio::sync::oneshot::channel();
    let (source_received, source_wait) = tokio::sync::oneshot::channel();
    let (release_source, source_release) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        for (request_path, range) in [
            ("/catalog/movie/fixture.json", 0..27),
            ("/catalog/movie/fixture/skip=27.json", 0..0),
            ("/catalog/movie/fixture/skip=27.json", 25..30),
            ("/catalog/movie/fixture/skip=32.json", 30..30),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let count = socket.read(&mut request).await.unwrap();
            assert!(
                std::str::from_utf8(&request[..count])
                    .unwrap()
                    .starts_with(&format!("GET {request_path} "))
            );
            let metas: Vec<_> = range.map(|n| serde_json::json!({"id":format!("film-{n}"),"name":format!("Film {n}"),"type":"movie"})).collect();
            let failed = request_path.ends_with("skip=27.json") && metas.is_empty();
            let status = if failed {
                "503 Service Unavailable"
            } else {
                "200 OK"
            };
            let body = serde_json::json!({"metas":metas}).to_string();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        let count = socket.read(&mut request).await.unwrap();
        assert!(
            std::str::from_utf8(&request[..count])
                .unwrap()
                .starts_with("GET /meta/movie/film-0.json ")
        );
        first_received.send(()).unwrap();
        let mut old_socket = socket;
        let (mut socket, _) = listener.accept().await.unwrap();
        let count = socket.read(&mut request).await.unwrap();
        assert!(
            std::str::from_utf8(&request[..count])
                .unwrap()
                .starts_with("GET /meta/movie/film-0.json ")
        );
        let body = r#"{"meta":{"id":"film-0","type":"movie","name":"Obsolete film"}}"#;
        old_socket
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
        fresh_wait.await.unwrap();
        let body = r#"{"meta":{"id":"film-0","type":"movie","name":"Detailed film","description":"Owned metadata fixture"}}"#;
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
        let (mut socket, _) = listener.accept().await.unwrap();
        let count = socket.read(&mut request).await.unwrap();
        assert!(
            std::str::from_utf8(&request[..count])
                .unwrap()
                .starts_with("GET /stream/movie/film-0.json ")
        );
        source_received.send(()).unwrap();
        let mut old_source = socket;
        let (mut socket, _) = listener.accept().await.unwrap();
        let count = socket.read(&mut request).await.unwrap();
        assert!(
            std::str::from_utf8(&request[..count])
                .unwrap()
                .starts_with("GET /stream/movie/film-0.json ")
        );
        let old_body = r#"{"streams":[{"externalUrl":"https://example.test/obsolete","name":"Obsolete source"}]}"#;
        old_source
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    old_body.len(),
                    old_body
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        source_release.await.unwrap();
        let body = r#"{"streams":[{"infoHash":"0123456789abcdef0123456789abcdef01234567","name":"Owned torrent","fileIdx":0},{"externalUrl":"https://example.test/private-token","name":"External player"}]}"#;
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
    });
    let env = Environment::new().unwrap();
    let mut profile = Profile::default();
    profile.addons=vec![serde_json::from_value(serde_json::json!({"transportUrl":format!("http://{address}/manifest.json"),"manifest":{"id":"fixture","name":"Fixture","version":"1.0.0","description":"Fixture","resources":["catalog","meta","stream"],"types":["movie"],"catalogs":[{"id":"fixture","type":"movie","extra":[{"name":"skip"}]}]}})).unwrap()];
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
            .await
            .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
    })
    .await;
    let mut session = CoreSession::guest(env).await.unwrap();
    session
        .load_catalog(0, session.snapshot().revision)
        .await
        .unwrap();
    for (offset, ids) in [
        (0, (0..24).collect::<Vec<_>>()),
        (24, (24..27).collect()),
        (27, (27..30).collect()),
        (30, vec![]),
        (0, (0..24).collect()),
    ] {
        if offset != 0
            || session
                .snapshot()
                .catalog
                .as_ref()
                .is_some_and(|v| v.offset != 0)
        {
            session
                .show_catalog_items(offset, session.snapshot().revision)
                .await
                .unwrap();
        }
        if offset == 27 {
            let failed = tokio::time::timeout(std::time::Duration::from_secs(2), session.settled())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(failed.catalog.unwrap().status, "failed");
            session
                .show_catalog_items(27, failed.revision)
                .await
                .unwrap();
        }
        let state = tokio::time::timeout(std::time::Duration::from_secs(2), session.settled())
            .await
            .unwrap()
            .unwrap();
        let view = state.catalog.unwrap();
        assert_eq!(view.status, "ready");
        assert_eq!(view.offset, offset);
        assert_eq!(
            view.items.iter().map(|i| i.id.clone()).collect::<Vec<_>>(),
            ids.into_iter()
                .map(|i| format!("film-{i}"))
                .collect::<Vec<_>>()
        );
        if offset == 30 {
            assert!(!view.has_more);
        }
    }
    assert!(
        session
            .show_catalog_items(31, session.snapshot().revision)
            .await
            .is_err()
    );
    session
        .load_details(0, session.snapshot().revision)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), first_wait)
        .await
        .unwrap()
        .unwrap();
    session
        .load_details(0, session.snapshot().revision)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if session.snapshot().pending_effects == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(session.snapshot().details.unwrap().status, "loading");
    release_fresh.send(()).unwrap();
    let state = tokio::time::timeout(std::time::Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    let details = state.details.unwrap();
    assert_eq!(details.status, "ready");
    assert_eq!(details.id, "film-0");
    assert_eq!(details.name.as_deref(), Some("Detailed film"));
    assert_eq!(
        details.description.as_deref(),
        Some("Owned metadata fixture")
    );
    session
        .load_sources(None, session.snapshot().revision)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), source_wait)
        .await
        .unwrap()
        .unwrap();
    session
        .load_sources(None, session.snapshot().revision)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if session.snapshot().pending_effects == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(session.snapshot().sources.unwrap().status, "loading");
    release_source.send(()).unwrap();
    let state = tokio::time::timeout(std::time::Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    let sources = state.sources.unwrap();
    assert_eq!(sources.status, "ready");
    assert_eq!(sources.video_id, "film-0");
    assert_eq!(sources.total, 2);
    assert_eq!(sources.items[0].kind, "torrent");
    assert_eq!(sources.items[0].name.as_deref(), Some("Owned torrent"));
    assert_eq!(sources.items[0].size_bytes, None);
    assert_eq!(sources.items[1].kind, "unsupported");
    assert!(
        !serde_json::to_string(&sources)
            .unwrap()
            .contains("private-token")
    );
    let revision = session.snapshot().revision;
    let playback = session.prepare_torrent(0, revision).await.unwrap();
    assert_ne!(playback.playback_token, 0);
    let replacement = session.prepare_torrent(0, revision).await.unwrap();
    assert_ne!(playback.playback_token, replacement.playback_token);
    assert_eq!(
        session.claim_playback(playback.playback_token).await,
        Err("stale_playback")
    );
    let chosen = session
        .claim_playback(replacement.playback_token)
        .await
        .unwrap();
    assert_eq!(chosen.meta_request.unwrap().path.id, "film-0");
    assert_eq!(chosen.stream_request.unwrap().path.id, "film-0");
    assert_eq!(
        cue_core::playback_source::torrent(&chosen.stream)
            .unwrap()
            .file_index,
        Some(0)
    );
    assert_eq!(
        session.claim_playback(replacement.playback_token).await,
        Err("stale_playback")
    );

    use cue_core::player::{PlaybackEvent, Progress};
    let playing = session.prepare_torrent(0, revision).await.unwrap();
    session
        .start_playback(playing.playback_token)
        .await
        .unwrap();
    let seek = session
        .playback_event(
            playing.playback_token,
            PlaybackEvent::Seek {
                time: 800,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        seek,
        Progress {
            time: 800,
            duration: 1000,
            time_watched: 0,
            watched: false
        }
    );
    let played = session
        .playback_event(
            playing.playback_token,
            PlaybackEvent::TimeChanged {
                time: 900,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    assert_eq!(played.time_watched, 100);
    // Queue retry after an unknown receipt repeats the exact absolute sample.
    // Neither repetition nor repeated pause/seek may add viewed time.
    for event in [
        PlaybackEvent::TimeChanged {
            time: 900,
            duration: 1000,
        },
        PlaybackEvent::TimeChanged {
            time: 900,
            duration: 1000,
        },
        PlaybackEvent::Paused(true),
        PlaybackEvent::Paused(true),
        PlaybackEvent::Paused(false),
        PlaybackEvent::Paused(false),
        PlaybackEvent::Seek {
            time: 900,
            duration: 1000,
        },
        PlaybackEvent::Seek {
            time: 900,
            duration: 1000,
        },
    ] {
        assert_eq!(
            session
                .playback_event(playing.playback_token, event)
                .await
                .unwrap(),
            Progress {
                time: 900,
                duration: 1000,
                time_watched: 100,
                watched: false
            }
        );
    }

    assert!(
        !played.watched,
        "Seeking to credits must not count skipped content"
    );
    session
        .playback_event(
            playing.playback_token,
            PlaybackEvent::Seek {
                time: 100,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    let threshold = session
        .playback_event(
            playing.playback_token,
            PlaybackEvent::TimeChanged {
                time: 700,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    assert_eq!(threshold.time_watched, 700);
    assert!(
        !threshold.watched,
        "Core threshold is strictly greater than 70 percent"
    );
    assert!(
        session
            .playback_event(
                playing.playback_token,
                PlaybackEvent::TimeChanged {
                    time: 701,
                    duration: 1000
                }
            )
            .await
            .unwrap()
            .watched
    );
    session
        .playback_event(playing.playback_token, PlaybackEvent::Paused(true))
        .await
        .unwrap();
    let paused = session
        .playback_event(
            playing.playback_token,
            PlaybackEvent::TimeChanged {
                time: 999,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    assert_eq!(paused.time, 701);
    assert_eq!(paused.time_watched, 701);
    session
        .set_details_watched(None, false, session.snapshot().revision)
        .await
        .unwrap();
    assert!(session.settled().await.unwrap().error.is_none());
    session
        .playback_event(playing.playback_token, PlaybackEvent::Paused(false))
        .await
        .unwrap();
    let overridden = session
        .playback_event(
            playing.playback_token,
            PlaybackEvent::TimeChanged {
                time: 800,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    assert!(
        !overridden.watched,
        "Manual unwatched must win over later threshold progress"
    );
    assert_eq!(overridden.time_watched, 800);
    session
        .set_details_watched(None, true, session.snapshot().revision)
        .await
        .unwrap();
    assert!(session.settled().await.unwrap().error.is_none());
    let marked = session
        .playback_event(
            playing.playback_token,
            PlaybackEvent::TimeChanged {
                time: 900,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    assert!(marked.watched);
    assert_eq!(
        marked.time_watched, 900,
        "Manual watched must not reset the decoder position and count it again"
    );
    session
        .set_details_watched(None, false, session.snapshot().revision)
        .await
        .unwrap();
    assert!(session.settled().await.unwrap().error.is_none());
    let revision = session.snapshot().revision;
    let next_owner = session.prepare_torrent(0, revision).await.unwrap();
    assert_eq!(
        session.start_playback(next_owner.playback_token).await,
        Err("playback_busy")
    );
    assert_eq!(
        session.retire_playback(next_owner.playback_token).await,
        Err("stale_playback")
    );
    session
        .retire_playback(playing.playback_token)
        .await
        .unwrap();
    assert_eq!(
        session
            .playback_event(
                playing.playback_token,
                PlaybackEvent::Seek {
                    time: 0,
                    duration: 1000
                }
            )
            .await,
        Err("stale_playback")
    );
    session
        .start_playback(next_owner.playback_token)
        .await
        .unwrap();
    assert_eq!(
        session
            .playback_event(
                playing.playback_token,
                PlaybackEvent::Seek {
                    time: 0,
                    duration: 1000
                }
            )
            .await,
        Err("stale_playback")
    );
    let unknown = session
        .playback_event(
            next_owner.playback_token,
            PlaybackEvent::TimeChanged {
                time: 1000,
                duration: 0,
            },
        )
        .await
        .unwrap();
    assert_eq!(unknown.time, 1000);
    assert!(
        !unknown.watched,
        "Unknown duration must not automatically complete"
    );
    assert_eq!(
        session
            .playback_event(
                next_owner.playback_token,
                PlaybackEvent::TimeChanged {
                    time: 500,
                    duration: 0
                }
            )
            .await,
        Err("playback_requires_seek")
    );
    let clamped = session
        .playback_event(
            next_owner.playback_token,
            PlaybackEvent::Seek {
                time: 2000,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    assert_eq!(clamped.time, 1000);
    assert!(!clamped.watched);
    session
        .retire_playback(next_owner.playback_token)
        .await
        .unwrap();
    let cancellation = tokio_util::sync::CancellationToken::new();
    let subtitles = session
        .discover_subtitles("film-0", "film-0", "movie", cancellation.clone())
        .await
        .unwrap();
    assert!(
        subtitles.tracks.is_empty(),
        "No installed subtitle resource should produce an empty success"
    );
    assert!(matches!(
        session
            .discover_subtitles("film-0", "different-episode", "movie", cancellation.clone())
            .await,
        Err("stale_subtitle_identity")
    ));
    assert!(matches!(
        session
            .discover_subtitles("different-title", "film-0", "movie", cancellation.clone())
            .await,
        Err("stale_subtitle_identity")
    ));
    assert!(matches!(
        session
            .discover_subtitles("film-0", "film-0", "series", cancellation)
            .await,
        Err("stale_subtitle_identity")
    ));

    assert_eq!(playback.torrent.file_index, Some(0));
    assert_eq!(playback.meta_id, "film-0");
    assert_eq!(playback.video_id, "film-0");
    assert_eq!(playback.kind, "movie");
    assert!(
        playback
            .torrent
            .magnet
            .contains("0123456789abcdef0123456789abcdef01234567")
    );
    assert!(session.prepare_torrent(1, revision).await.is_err());
    assert!(session.prepare_torrent(2, revision).await.is_err());
    session.show_sources(1, revision).await.unwrap();
    assert!(session.prepare_torrent(0, revision).await.is_err());
    assert!(
        session
            .prepare_torrent(0, session.snapshot().revision)
            .await
            .is_err()
    );
    session
        .show_sources(0, session.snapshot().revision)
        .await
        .unwrap();
    let retiring = session
        .prepare_torrent(0, session.snapshot().revision)
        .await
        .unwrap();
    session
        .load_details(0, session.snapshot().revision)
        .await
        .unwrap();
    assert_eq!(
        session.claim_playback(retiring.playback_token).await,
        Err("stale_playback")
    );
    server.await.unwrap();
    session.stop().await;
}

#[tokio::test]
async fn search_uses_declared_catalog_extra_and_rejects_unbounded_queries() {
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
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = vec![0; 8192];
        let count = socket.read(&mut buffer).await.unwrap();
        let request = String::from_utf8_lossy(&buffer[..count]);
        assert!(request.contains("search=blue%20sky"), "{request}");
        let body = r#"{"metas":[{"id":"search-result","type":"movie","name":"Blue sky"}]}"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });
    let env = Environment::new().unwrap();
    let mut profile = Profile::default();
    profile.addons = vec![serde_json::from_value(serde_json::json!({"transportUrl":format!("http://{address}/manifest.json"),"manifest":{"id":"search.fixture","name":"Search","version":"1.0.0","description":"Owned fixture","resources":["catalog"],"types":["movie"],"catalogs":[{"id":"search","type":"movie","extra":[{"name":"search","isRequired":true}]}]}})).unwrap()];
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
            .await
            .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
    })
    .await;
    let mut session = CoreSession::guest(env).await.unwrap();
    assert!(
        session
            .load_catalog(0, session.snapshot().revision)
            .await
            .is_err()
    );
    assert!(
        session
            .search_catalog(0, &"x".repeat(257), session.snapshot().revision)
            .await
            .is_err()
    );
    session
        .search_catalog(0, "blue sky", session.snapshot().revision)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if session
                .snapshot()
                .catalog
                .as_ref()
                .is_some_and(|v| v.items.first().is_some_and(|i| i.id == "search-result"))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    server.await.unwrap();
    session.stop().await;
}
