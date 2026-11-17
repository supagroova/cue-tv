use cue_core::{
    environment::{Environment, NativeEnv},
    session::CoreSession,
};
use stremio_core::{
    constants::*,
    runtime::Env,
    types::{library::LibraryBucket, profile::Profile},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn selected_library_membership_is_revision_guarded_and_survives_encrypted_reopen() {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        membership_roundtrip(false, false, None, false),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn selected_library_removal_persists_a_tombstone() {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        membership_roundtrip(true, false, None, false),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn failed_library_write_remains_failed_until_retry_is_durable() {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        membership_roundtrip(false, true, None, false),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn film_watched_changes_are_idempotent_durable_and_do_not_add_membership() {
    for watched in [true, false] {
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            membership_roundtrip(false, false, Some(watched), false),
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn episode_watched_changes_preserve_other_episodes_and_survive_reopen() {
    for watched in [true, false] {
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            membership_roundtrip(false, false, Some(watched), true),
        )
        .await
        .unwrap();
    }
}

async fn membership_roundtrip(remove: bool, fail_write: bool, watched: Option<bool>, series: bool) {
    let kind = if series { "series" } else { "movie" };
    static NEXT_DIRECTORY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT_DIRECTORY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "cue-library-{}-{}-{sequence}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!(
        "http://{}/private/manifest.json",
        listener.local_addr().unwrap()
    );
    let server = tokio::spawn(async move {
        for (path, body) in [
            (
                "/private/catalog/movie/owned.json",
                r#"{"metas":[{"id":"owned-film","type":"movie","name":"Owned film"}]}"#,
            ),
            (
                "/private/meta/movie/owned-film.json",
                r#"{"meta":{"id":"owned-film","type":"movie","name":"Owned film"}}"#,
            ),
        ] {
            let path = path.replace("movie", kind);
            let body = if series && path.contains("/meta/") {
                r#"{"meta":{"id":"owned-film","type":"series","name":"Owned film","videos":[{"id":"owned-film:1:1","title":"First","season":1,"episode":1},{"id":"owned-film:1:2","title":"Second","season":1,"episode":2}]}}"#.to_owned()
            } else {
                body.replace("movie", kind)
            };
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let count = socket.read(&mut request).await.unwrap();
            assert!(
                std::str::from_utf8(&request[..count])
                    .unwrap()
                    .starts_with(&format!("GET {path} "))
            );
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    let env = Environment::persistent(&directory, &[9; 32]).unwrap();
    let mut profile = Profile::default();
    profile.addons = vec![serde_json::from_value(serde_json::json!({"transportUrl":base,"manifest":{"id":"org.cue.library","version":"1.0.0","name":"Owned fixture","description":"Owned fixture","types":[kind],"resources":["catalog","meta"],"catalogs":[{"id":"owned","type":kind,"name":"Owned"}]}})).unwrap()];
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
    assert_eq!(
        session
            .set_details_library(true, session.snapshot().revision)
            .await,
        Err("details_not_ready")
    );
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
    server.await.unwrap();
    assert_eq!(session.snapshot().details_in_library, Some(false));
    if let Some(watched) = watched {
        let video_id = series.then(|| "owned-film:1:2".to_owned());
        assert_eq!(
            session
                .set_details_watched(
                    Some("missing-video".into()),
                    true,
                    session.snapshot().revision
                )
                .await,
            Err("invalid_video")
        );
        let unavailable = directory.with_extension("unavailable");
        std::fs::rename(&directory, &unavailable).unwrap();
        for _ in 0..2 {
            session
                .set_details_watched(video_id.clone(), true, session.snapshot().revision)
                .await
                .unwrap();
            assert_eq!(
                session.settled().await.unwrap().error,
                Some("core_effect_failed")
            );
        }
        std::fs::rename(unavailable, &directory).unwrap();
        let initial = session.snapshot().revision;
        session
            .set_details_watched(video_id.clone(), true, initial)
            .await
            .unwrap();
        assert!(session.settled().await.unwrap().error.is_none());
        assert_eq!(session.snapshot().details_in_library, Some(false));
        let details = session.snapshot().details.unwrap();
        if series {
            assert_eq!(details.videos[0].watched, Some(false));
            assert_eq!(details.videos[1].watched, Some(true));
        } else {
            assert_eq!(details.watched, Some(true));
        }
        assert_eq!(
            session
                .set_details_watched(video_id.clone(), false, initial)
                .await,
            Err("stale_details")
        );
        // Reasserting true is an idempotent retry, not a second completed viewing.
        session
            .set_details_watched(video_id.clone(), watched, session.snapshot().revision)
            .await
            .unwrap();
        assert!(session.settled().await.unwrap().error.is_none());
        let details = session.snapshot().details.unwrap();
        if series {
            assert_eq!(details.videos[0].watched, Some(false));
            assert_eq!(details.videos[1].watched, Some(watched));
        } else {
            assert_eq!(details.watched, Some(watched));
        }
        assert_eq!(session.snapshot().details_in_library, Some(false));
        // Check disk before the subsequent membership action can persist anything.
        env.scope(async {
            let library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            let item = library.items.get("owned-film").unwrap();
            assert!(item.removed);
            assert!(item.temp);
            assert_watched(item, series, watched);
        })
        .await;
    }
    let stale = session.snapshot().revision;
    if fail_write {
        let unavailable = directory.with_extension("unavailable");
        std::fs::rename(&directory, &unavailable).unwrap();
        session.set_details_library(true, stale).await.unwrap();
        assert_eq!(
            session.settled().await.unwrap().error,
            Some("core_effect_failed")
        );
        // Retrying an already-updated in-memory item must attempt persistence again.
        session
            .set_details_library(true, session.snapshot().revision)
            .await
            .unwrap();
        assert_eq!(
            session.settled().await.unwrap().error,
            Some("core_effect_failed")
        );
        std::fs::rename(unavailable, &directory).unwrap();
    }
    session
        .set_details_library(true, session.snapshot().revision)
        .await
        .unwrap();
    let added = session.settled().await.unwrap();
    assert!(added.error.is_none());
    assert_eq!(added.details_in_library, Some(true));
    assert_eq!(
        session.set_details_library(false, stale).await,
        Err("stale_details")
    );
    assert_eq!(session.snapshot().details_in_library, Some(true));
    if remove {
        session
            .set_details_library(false, session.snapshot().revision)
            .await
            .unwrap();
        let removed = session.settled().await.unwrap();
        assert!(removed.error.is_none());
        assert_eq!(removed.details_in_library, Some(false));
    }
    session.stop().await;
    drop(session);
    drop(env);
    let env = Environment::persistent(&directory, &[9; 32]).unwrap();
    let mut restored = CoreSession::guest(env.clone()).await.unwrap();
    env.scope(async {
        let library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        let item = library.items.get("owned-film").unwrap();
        assert_eq!(item.removed, remove);
        assert!(!item.temp);
        assert_eq!(item.name, "Owned film");
        if let Some(watched) = watched {
            assert_watched(item, series, watched);
        }
    })
    .await;
    let ciphertext = std::fs::read(directory.join("state.bin")).unwrap();
    assert!(!String::from_utf8_lossy(&ciphertext).contains("owned-film"));
    restored.stop().await;
    drop(restored);
    drop(env);
    std::fs::remove_dir_all(directory).unwrap();
}

fn assert_watched(item: &stremio_core::types::library::LibraryItem, series: bool, watched: bool) {
    if series {
        let videos: Vec<stremio_core::types::resource::Video> =
            serde_json::from_value(serde_json::json!([
                {"id":"owned-film:1:1","title":"First","season":1,"episode":1},
                {"id":"owned-film:1:2","title":"Second","season":1,"episode":2}
            ]))
            .unwrap();
        let bits = item.state.watched_bitfield(&videos);
        assert!(!bits.get_video("owned-film:1:1"));
        assert_eq!(bits.get_video("owned-film:1:2"), watched);
    } else {
        assert_eq!(item.state.times_watched, if watched { 1 } else { 0 });
    }
}
