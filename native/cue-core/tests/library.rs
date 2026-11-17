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
        membership_roundtrip(false, false),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn selected_library_removal_persists_a_tombstone() {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        membership_roundtrip(true, false),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn failed_library_write_remains_failed_until_retry_is_durable() {
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        membership_roundtrip(false, true),
    )
    .await
    .unwrap();
}

async fn membership_roundtrip(remove: bool, fail_write: bool) {
    let directory = std::env::temp_dir().join(format!(
        "cue-library-{}-{}",
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
    profile.addons = vec![serde_json::from_value(serde_json::json!({"transportUrl":base,"manifest":{"id":"org.cue.library","version":"1.0.0","name":"Owned fixture","description":"Owned fixture","types":["movie"],"resources":["catalog","meta"],"catalogs":[{"id":"owned","type":"movie","name":"Owned"}]}})).unwrap()];
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
    })
    .await;
    let ciphertext = std::fs::read(directory.join("state.bin")).unwrap();
    assert!(!String::from_utf8_lossy(&ciphertext).contains("owned-film"));
    restored.stop().await;
    drop(restored);
    drop(env);
    std::fs::remove_dir_all(directory).unwrap();
}
