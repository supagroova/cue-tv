use cue_core::{
    environment::{Environment, NativeEnv},
    player::PlaybackEvent,
    session::CoreSession,
};
use stremio_core::{
    constants::*,
    runtime::Env,
    types::{library::LibraryBucket, profile::Profile},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn checkpoint_ack_requires_durable_write_and_retry_survives_reopen() {
    tokio::time::timeout(std::time::Duration::from_secs(10), roundtrip(false))
        .await
        .unwrap();
}
#[tokio::test]
async fn lost_checkpoint_and_close_acks_can_be_reconciled() {
    tokio::time::timeout(std::time::Duration::from_secs(10), roundtrip(true))
        .await
        .unwrap();
}
async fn roundtrip(lose_receipts: bool) {
    let directory = std::env::temp_dir().join(format!(
        "cue-checkpoint-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/manifest.json", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for attempt in 0..2 {
            for (path, body) in [
                (
                    "/catalog/movie/owned.json",
                    r#"{"metas":[{"id":"film","type":"movie","name":"Owned"}]}"#,
                ),
                (
                    "/meta/movie/film.json",
                    r#"{"meta":{"id":"film","type":"movie","name":"Owned"}}"#,
                ),
                (
                    "/stream/movie/film.json",
                    r#"{"streams":[{"infoHash":"0123456789abcdef0123456789abcdef01234567","fileIdx":0}]}"#,
                ),
            ] {
                let body = if attempt == 0 {
                    body.to_owned()
                } else {
                    body.replace(
                        "0123456789abcdef0123456789abcdef01234567",
                        "fedcba9876543210fedcba9876543210fedcba98",
                    )
                };
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0; 4096];
                let count = socket.read(&mut request).await.unwrap();
                assert!(
                    std::str::from_utf8(&request[..count])
                        .unwrap()
                        .starts_with(&format!("GET {path} "))
                );
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
            }
        }
    });
    let env = Environment::persistent(&directory, &[7; 32]).unwrap();
    let mut profile = Profile::default();
    profile.addons = vec![serde_json::from_value(serde_json::json!({"transportUrl":base,"manifest":{"id":"owned.checkpoint","version":"1.0.0","name":"Owned","description":"Owned fixture","resources":["catalog","meta","stream"],"types":["movie"],"catalogs":[{"id":"owned","type":"movie"}]}})).unwrap()];
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
            .await
            .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
    })
    .await;
    let mut session = CoreSession::guest(env.clone()).await.unwrap();
    session
        .load_catalog(0, session.snapshot().revision)
        .await
        .unwrap();
    session.settled().await.unwrap();
    session
        .load_details(0, session.snapshot().revision)
        .await
        .unwrap();
    session.settled().await.unwrap();
    session
        .load_sources(None, session.snapshot().revision)
        .await
        .unwrap();
    session.settled().await.unwrap();
    let prepared = session
        .prepare_torrent(0, session.snapshot().revision)
        .await
        .unwrap();
    let token = prepared.playback_token;
    session.start_playback(token).await.unwrap();
    session
        .playback_event(
            token,
            PlaybackEvent::TimeChanged {
                time: 400,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    // Repeating an admitted start must preserve the current playback clock.
    session.start_playback(token).await.unwrap();
    let after_start_retry = session
        .playback_event(token, PlaybackEvent::Paused(false))
        .await
        .unwrap();
    assert_eq!(after_start_retry.time, 400);
    assert_eq!(after_start_retry.time_watched, 400);
    assert_eq!(session.start_playback(u64::MAX).await, Err("playback_busy"));
    // Hold the real sequential writer to prove acknowledgement is not admission.
    let (release, blocked) = tokio::sync::oneshot::channel();
    let (entered, started) = tokio::sync::oneshot::channel();
    env.scope(async {
        NativeEnv::exec_sequential(async move {
            let _ = entered.send(());
            let _ = blocked.await;
        });
    })
    .await;
    started.await.unwrap();
    let mut release = Some(release);
    {
        let save = session.checkpoint_playback(token);
        tokio::pin!(save);
        tokio::select! {
            result = &mut save => panic!("Checkpoint acknowledged before write: {result:?}"),
            _ = async { while session.snapshot().pending_effects == 0 { tokio::task::yield_now().await; } } => {}
        }
        assert_eq!(
            session.checkpoint_playback(token).await,
            Err("checkpoint_busy")
        );
        assert_eq!(session.retire_playback(token).await, Err("checkpoint_busy"));
        assert_eq!(session.logout().await, Err("account_busy"));
        session
            .playback_event(
                token,
                PlaybackEvent::TimeChanged {
                    time: 450,
                    duration: 1000,
                },
            )
            .await
            .unwrap();
        if !lose_receipts {
            release.take().unwrap().send(()).unwrap();
            save.await.unwrap();
        }
    }
    if let Some(release) = release {
        release.send(()).unwrap();
        session.settled().await.unwrap();
    }
    env.scope(async {
        let bucket = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bucket.items["film"].state.time_offset, 400);
        assert_eq!(bucket.items["film"].state.time_watched, 400);
    })
    .await;
    // The first receipt belongs to the captured 400ms state. A following save
    // persists newer progress, never the delayed previous snapshot.
    session.checkpoint_playback(token).await.unwrap();
    let unavailable = directory.with_extension("unavailable");
    std::fs::rename(&directory, &unavailable).unwrap();
    session
        .playback_event(
            token,
            PlaybackEvent::TimeChanged {
                time: 500,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        session.checkpoint_playback(token).await,
        Err("checkpoint_write_failed")
    );
    env.scope(async {
        let bucket = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(bucket.items["film"].state.time_offset, 450);
    })
    .await;
    assert_eq!(
        session.close_playback(token).await,
        Err("checkpoint_write_failed")
    );
    std::fs::rename(&unavailable, &directory).unwrap();
    session
        .playback_event(
            token,
            PlaybackEvent::TimeChanged {
                time: 600,
                duration: 1000,
            },
        )
        .await
        .unwrap();
    session.checkpoint_playback(token).await.unwrap();
    let (release_close, blocked_close) = tokio::sync::oneshot::channel();
    let (entered_close, started_close) = tokio::sync::oneshot::channel();
    env.scope(async {
        NativeEnv::exec_sequential(async move {
            let _ = entered_close.send(());
            let _ = blocked_close.await;
        });
    })
    .await;
    started_close.await.unwrap();
    let mut release_close = Some(release_close);
    {
        let close = session.close_playback(token);
        tokio::pin!(close);
        tokio::select! {
            result = &mut close => panic!("Close acknowledged before write: {result:?}"),
            _ = async { while session.snapshot().pending_effects == 0 { tokio::task::yield_now().await; } } => {}
        }
        assert_eq!(
            session
                .playback_event(
                    token,
                    PlaybackEvent::TimeChanged {
                        time: 900,
                        duration: 1000
                    }
                )
                .await,
            Err("playback_closing")
        );
        assert_eq!(session.close_playback(token).await, Err("checkpoint_busy"));
        if !lose_receipts {
            release_close.take().unwrap().send(()).unwrap();
            close.await.unwrap();
        }
        // Otherwise drop the waiting caller while the actual writer is blocked.
    }
    if let Some(release_close) = release_close {
        release_close.send(()).unwrap();
        session.settled().await.unwrap();
    }
    assert_eq!(
        session
            .playback_event(
                token,
                PlaybackEvent::TimeChanged {
                    time: 900,
                    duration: 1000
                }
            )
            .await,
        Err("stale_playback")
    );
    let saved = std::fs::read(directory.join("state.bin")).unwrap();
    assert_eq!(session.close_playback(token).await, Ok(()));
    assert_eq!(std::fs::read(directory.join("state.bin")).unwrap(), saved);
    assert_eq!(
        session.close_playback(u64::MAX).await,
        Err("stale_playback")
    );
    // A duplicate close must not retire a newer owner, even for the same title.
    let next = session
        .prepare_torrent(0, session.snapshot().revision)
        .await
        .unwrap();
    assert_ne!(next.playback_token, token);
    session.start_playback(next.playback_token).await.unwrap();
    assert_eq!(session.close_playback(token).await, Ok(()));
    assert!(
        session
            .playback_event(next.playback_token, PlaybackEvent::Paused(true))
            .await
            .is_ok()
    );
    session.retire_playback(next.playback_token).await.unwrap();
    session.retire_playback(next.playback_token).await.unwrap();
    assert_eq!(
        session.start_playback(next.playback_token).await,
        Err("stale_playback")
    );
    let current = session
        .prepare_torrent(0, session.snapshot().revision)
        .await
        .unwrap();
    session
        .start_playback(current.playback_token)
        .await
        .unwrap();
    session.retire_playback(next.playback_token).await.unwrap();
    assert!(
        session
            .playback_event(current.playback_token, PlaybackEvent::Paused(true))
            .await
            .is_ok()
    );
    session
        .retire_playback(current.playback_token)
        .await
        .unwrap();
    assert_eq!(
        session.retire_playback(next.playback_token).await,
        Err("stale_playback")
    );
    assert_eq!(
        session.checkpoint_playback(token).await,
        Err("stale_playback")
    );
    session.stop().await;
    drop(session);
    drop(env);
    let reopened = Environment::persistent(&directory, &[7; 32]).unwrap();
    reopened
        .scope(async {
            let bucket = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            let item = &bucket.items["film"];
            assert_eq!(item.state.time_offset, 600);
            assert_eq!(item.state.time_watched, 600);
            assert_eq!(item.state.video_id.as_deref(), Some("film"));
            assert!(!item.watched());
        })
        .await;
    let mut resumed = CoreSession::guest(reopened.clone()).await.unwrap();
    resumed
        .load_catalog(0, resumed.snapshot().revision)
        .await
        .unwrap();
    resumed.settled().await.unwrap();
    resumed
        .load_details(0, resumed.snapshot().revision)
        .await
        .unwrap();
    resumed.settled().await.unwrap();
    resumed
        .load_sources(None, resumed.snapshot().revision)
        .await
        .unwrap();
    resumed.settled().await.unwrap();
    let fresh = resumed
        .prepare_torrent(0, resumed.snapshot().revision)
        .await
        .unwrap();
    assert_ne!(fresh.playback_token, token);
    assert!(
        fresh
            .torrent
            .magnet
            .contains("fedcba9876543210fedcba9876543210fedcba98")
    );
    resumed.start_playback(fresh.playback_token).await.unwrap();
    let restored = resumed
        .playback_event(fresh.playback_token, PlaybackEvent::Paused(true))
        .await
        .unwrap();
    assert_eq!(
        restored,
        cue_core::player::Progress {
            time: 600,
            duration: 1000,
            time_watched: 600,
            watched: false
        }
    );
    resumed.retire_playback(fresh.playback_token).await.unwrap();
    resumed.retire_playback(fresh.playback_token).await.unwrap();
    assert_eq!(
        resumed.close_playback(fresh.playback_token).await,
        Err("stale_playback")
    );
    assert_eq!(resumed.close_playback(token).await, Err("stale_playback"));
    let terminal = resumed
        .prepare_torrent(0, resumed.snapshot().revision)
        .await
        .unwrap();
    resumed
        .start_playback(terminal.playback_token)
        .await
        .unwrap();
    resumed
        .close_playback(terminal.playback_token)
        .await
        .unwrap();
    resumed
        .close_playback(terminal.playback_token)
        .await
        .unwrap();
    resumed.logout().await.unwrap();
    assert_eq!(
        resumed.retire_playback(fresh.playback_token).await,
        Err("stale_playback")
    );
    assert_eq!(
        resumed.close_playback(terminal.playback_token).await,
        Err("stale_playback")
    );
    resumed.stop().await;
    drop(resumed);
    server.await.unwrap();
    reopened.shutdown().await;
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}
