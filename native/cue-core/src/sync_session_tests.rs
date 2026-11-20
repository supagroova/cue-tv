use crate::{
    environment::{Environment, NativeEnv},
    session::CoreSession,
    sync_journal::Journal,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use stremio_core::{
    constants::*,
    runtime::Env,
    types::{library::LibraryBucket, profile::Profile},
};

#[tokio::test]
async fn restored_pending_edit_uploads_and_acknowledges_without_user_interaction() {
    let env = stored_pending().await;
    let requests = Arc::new(AtomicUsize::new(0));
    let observed = requests.clone();
    *env.test_http.lock().unwrap() = Some(Arc::new(move |parts, bytes| {
        assert!(parts.uri.path().ends_with("/datastorePut"));
        let request: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(request["changes"][0]["state"]["timeOffset"], 100);
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(br#"{"result":{"success":true}}"#.to_vec())
    }));
    let mut session = CoreSession::restore_account(env.clone(), "owner-a")
        .await
        .unwrap();
    let mut cleared = false;
    for _ in 0..200 {
        cleared = env
            .scope(async {
                NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                    .await
                    .unwrap()
                    .unwrap()
                    .revision("title")
                    .is_none()
            })
            .await;
        if cleared {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(
        cleared,
        "Restored pending edits were never durably acknowledged"
    );
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    session.stop().await;
}

async fn stored_pending() -> Arc<Environment> {
    seed_pending(Environment::new().unwrap()).await
}

async fn seed_pending(env: Arc<Environment>) -> Arc<Environment> {
    let mut profile = Profile::default();
    profile.auth = Some(serde_json::from_value(serde_json::json!({
        "key":"synthetic-token", "user":{"_id":"owner-a","email":"fixture@example.test",
        "lastModified":"2026-01-01T00:00:00Z","dateRegistered":"2026-01-01T00:00:00Z",
        "premium_expire":null,"gdpr_consent":{"tos":true,"privacy":true,"marketing":false,"from":null}}
    })).unwrap());
    let item = serde_json::from_value(
        serde_json::json!({"_id":"title","type":"movie","name":"Fixture",
        "removed":false,"temp":false,"_mtime":"2026-01-01T00:00:00Z","state":{
        "timeWatched":0,"timeOffset":100,"overallTimeWatched":0,"timesWatched":0,
        "flaggedWatched":0,"duration":1000}}),
    )
    .unwrap();
    let library = LibraryBucket::new(Some("owner-a".parse().unwrap()), vec![item]);
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
            .await
            .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
        let journal = Journal::new("owner-a")
            .unwrap()
            .with_intent("owner-a", "movie", "title")
            .unwrap();
        crate::checkpoint::write_with_journal(&library, &journal)
            .unwrap()
            .await
            .unwrap();
    })
    .await;
    env
}

#[tokio::test]
async fn rejected_upload_keeps_intent_and_does_not_retry_without_reauthentication() {
    let env = stored_pending().await;
    let requests = Arc::new(AtomicUsize::new(0));
    let observed = requests.clone();
    *env.test_http.lock().unwrap() = Some(Arc::new(move |parts, _| {
        assert!(parts.uri.path().ends_with("/datastorePut"));
        observed.fetch_add(1, Ordering::SeqCst);
        Ok(br#"{"error":{"code":1,"message":"synthetic-token rejected"}}"#.to_vec())
    }));
    let mut session = CoreSession::restore_account(env.clone(), "owner-a")
        .await
        .unwrap();
    for _ in 0..200 {
        if session.snapshot().error == Some("account_sync_failed") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(session.snapshot().error, Some("account_sync_failed"));
    session.set_interface_language("fra").await.unwrap();
    assert_eq!(
        session.snapshot().sync.as_ref().unwrap().state,
        "reauthentication_required"
    );
    assert_eq!(session.snapshot().sync.as_ref().unwrap().pending_titles, 1);
    assert_eq!(session.retry_sync().await, Err("reauthentication_required"));

    tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    env.scope(async {
        let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(journal.revision("title"), Some(1));
    })
    .await;
    assert!(
        !serde_json::to_string(&session.snapshot())
            .unwrap()
            .contains("synthetic-token")
    );
    session.stop().await;
}

#[tokio::test]
async fn older_upload_reply_cannot_clear_a_new_edit_admitted_during_http() {
    use futures::FutureExt;
    use std::sync::Mutex;
    use tokio::sync::Notify;
    let env = stored_pending().await;
    let observed = Arc::new(Mutex::new(Vec::new()));
    let first_started = Arc::new(Notify::new());
    let second_started = Arc::new(Notify::new());
    let release_first = Arc::new(Notify::new());
    let release_second = Arc::new(Notify::new());
    let (positions, start1, start2, release1, release2) = (
        observed.clone(),
        first_started.clone(),
        second_started.clone(),
        release_first.clone(),
        release_second.clone(),
    );
    *env.test_http_async.lock().unwrap() = Some(Arc::new(move |parts, bytes| {
        assert!(parts.uri.path().ends_with("/datastorePut"));
        let request: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let index = {
            let mut positions = positions.lock().unwrap();
            positions.push(
                request["changes"][0]["state"]["timeOffset"]
                    .as_u64()
                    .unwrap(),
            );
            positions.len()
        };
        let (started, release) = if index == 1 {
            (start1.clone(), release1.clone())
        } else {
            (start2.clone(), release2.clone())
        };
        async move {
            started.notify_one();
            release.notified().await;
            Ok(br#"{"result":{"success":true}}"#.to_vec())
        }
        .boxed()
    }));
    let mut session = CoreSession::restore_account(env.clone(), "owner-a")
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), first_started.notified())
        .await
        .unwrap();
    let mut item = env
        .scope(async {
            NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap()
                .items["title"]
                .clone()
        })
        .await;
    item.state.time_offset = 200;
    session.test_library_item(item).await.unwrap();
    release_first.notify_one();
    tokio::time::timeout(std::time::Duration::from_secs(3), second_started.notified())
        .await
        .unwrap();
    env.scope(async {
        assert_eq!(
            NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                .await
                .unwrap()
                .unwrap()
                .revision("title"),
            Some(2)
        );
        assert_eq!(
            NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap()
                .items["title"]
                .state
                .time_offset,
            200
        );
    })
    .await;
    release_second.notify_one();
    let mut pending = true;
    for _ in 0..200 {
        pending = env
            .scope(async {
                NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                    .await
                    .unwrap()
                    .unwrap()
                    .has_pending()
            })
            .await;
        if !pending {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(!pending);
    assert_eq!(*observed.lock().unwrap(), vec![100, 200]);
    session.stop().await;
}

struct NotifyOnDrop(Arc<tokio::sync::Notify>);
impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

#[tokio::test]
async fn logout_cancels_suspended_upload_and_keeps_intent_until_account_cleanup() {
    use futures::FutureExt;
    let env = stored_pending().await;
    let started = Arc::new(tokio::sync::Notify::new());
    let cancelled = Arc::new(tokio::sync::Notify::new());
    let requests = Arc::new(AtomicUsize::new(0));
    let (start, cancel, count) = (started.clone(), cancelled.clone(), requests.clone());
    *env.test_http_async.lock().unwrap() = Some(Arc::new(move |parts, _| {
        if parts.uri.path().ends_with("/datastorePut") {
            count.fetch_add(1, Ordering::SeqCst);
            let (start, cancel) = (start.clone(), cancel.clone());
            async move {
                let _drop = NotifyOnDrop(cancel);
                start.notify_one();
                futures::future::pending().await
            }
            .boxed()
        } else {
            assert!(parts.uri.path().ends_with("/logout"));
            async { Ok(br#"{"result":{"success":true}}"#.to_vec()) }.boxed()
        }
    }));
    let mut session = CoreSession::restore_account(env.clone(), "owner-a")
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), started.notified())
        .await
        .unwrap();
    session.logout().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), cancelled.notified())
        .await
        .unwrap();
    for _ in 0..200 {
        if session.account_identity().is_none() && session.snapshot().pending_effects == 0 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert!(session.account_identity().is_none());
    env.scope(async {
        assert_eq!(
            NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                .await
                .unwrap()
                .unwrap()
                .revision("title"),
            Some(1)
        );
    })
    .await;
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    session.stop().await;
}

#[tokio::test]
async fn full_sync_journal_rejects_manual_watched_before_details_change() {
    let env = stored_pending().await;
    env.scope(async {
        let mut profile = NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        profile.addons = vec![serde_json::from_value(serde_json::json!({
            "transportUrl":"https://fixture.example.test/manifest.json",
            "manifest":{"id":"org.cue.capacity","version":"1.0.0","name":"Owned fixture",
            "description":"Owned fixture","types":["movie"],"resources":["catalog","meta","stream"],
            "catalogs":[{"id":"owned","type":"movie","name":"Owned"}]}
        })).unwrap()];
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
        let mut library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        let template = library.items.get("title").unwrap().clone();
        let mut journal = Journal::new("owner-a").unwrap();
        for index in 0..256 {
            let mut item = template.clone();
            item.id = format!("pending-{index}");
            journal = journal.with_intent("owner-a", "movie", &item.id).unwrap();
            library.merge_items(vec![item]);
        }
        crate::checkpoint::write_with_journal(&library, &journal)
            .unwrap()
            .await
            .unwrap();
    })
    .await;
    *env.test_http.lock().unwrap() = Some(Arc::new(|parts, _| {
        let path = parts.uri.path();
        let body = if path.ends_with("/datastorePut") {
            r#"{"error":{"code":1,"message":"fixture rejection"}}"#
        } else if path.ends_with("/datastoreGet") {
            r#"{"result":[]}"#
        } else if path.contains("/catalog/") {
            r#"{"metas":[{"id":"title","type":"movie","name":"Fixture"}]}"#
        } else if path.contains("/stream/") {
            r#"{"streams":[{"url":"https://fixture.example.test/owned.mp4"}]}"#
        } else if path.contains("/meta/") {
            r#"{"meta":{"id":"title","type":"movie","name":"Fixture"}}"#
        } else {
            panic!("Unexpected fixture request: {path}")
        };
        Ok(body.as_bytes().to_vec())
    }));
    let mut session = CoreSession::restore_account(env.clone(), "owner-a")
        .await
        .unwrap();
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
    assert_eq!(
        session.snapshot().details.as_ref().unwrap().watched,
        Some(false)
    );
    session
        .load_sources(None, session.snapshot().revision)
        .await
        .unwrap();
    session.settled().await.unwrap();
    let source = session
        .prepare_https(0, session.snapshot().revision)
        .await
        .unwrap();
    session.start_playback(source.playback_token).await.unwrap();
    let result = session
        .set_details_watched(None, true, session.snapshot().revision)
        .await;
    let progress = session
        .playback_event(
            source.playback_token,
            crate::player::PlaybackEvent::Paused(true),
        )
        .await
        .unwrap();
    assert!(
        !progress.watched,
        "Rejected watched intent changed the playback owner"
    );
    assert_eq!(
        session.snapshot().details.as_ref().unwrap().watched,
        Some(false)
    );
    assert_eq!(result, Err("library_write_failed"));
    session.stop().await;
}

#[tokio::test]
async fn explicit_retry_persists_failed_local_edit_before_upload() {
    let directory = std::env::temp_dir().join(format!(
        "cue-sync-retry-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap()
    ));
    std::fs::create_dir(&directory).unwrap();
    let unavailable = directory.with_extension("unavailable");
    let env = seed_pending(Environment::persistent(&directory, &[31; 32]).unwrap()).await;
    let sent = Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = sent.clone();
    *env.test_http.lock().unwrap() = Some(Arc::new(move |parts, bytes| {
        assert!(parts.uri.path().ends_with("/datastorePut"));
        let body: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        observed
            .lock()
            .unwrap()
            .push(body["changes"][0]["state"]["timeOffset"].as_u64().unwrap());
        Ok(br#"{"result":{"success":true}}"#.to_vec())
    }));
    let mut session = CoreSession::restore_account(env.clone(), "owner-a")
        .await
        .unwrap();
    for _ in 0..200 {
        if session
            .snapshot()
            .sync
            .as_ref()
            .is_some_and(|sync| sync.state == "idle")
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(session.snapshot().sync.as_ref().unwrap().state, "idle");
    let mut item = env
        .scope(async {
            NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap()
                .items["title"]
                .clone()
        })
        .await;
    item.state.time_offset = 200;
    std::fs::rename(&directory, &unavailable).unwrap();
    let _ = session.test_library_item(item).await;
    session.settled().await.unwrap();
    for _ in 0..200 {
        if session
            .snapshot()
            .sync
            .as_ref()
            .is_some_and(|sync| sync.state == "waiting")
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(session.snapshot().sync.as_ref().unwrap().pending_titles, 1);
    assert_eq!(*sent.lock().unwrap(), vec![100]);
    assert_eq!(session.retry_sync().await, Err("account_sync_save_failed"));
    assert_eq!(session.snapshot().sync.as_ref().unwrap().pending_titles, 1);
    assert_eq!(*sent.lock().unwrap(), vec![100]);
    std::fs::rename(&unavailable, &directory).unwrap();
    session.retry_sync().await.unwrap();
    for _ in 0..200 {
        if session
            .snapshot()
            .sync
            .as_ref()
            .is_some_and(|sync| sync.state == "idle")
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(
        *sent.lock().unwrap(),
        vec![100, 200],
        "Retry never uploaded the unsaved edit"
    );
    assert_eq!(session.snapshot().sync.as_ref().unwrap().state, "idle");
    env.scope(async {
        let library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(library.items["title"].state.time_offset, 200);
        let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        assert!(!journal.has_pending());
    })
    .await;
    session.stop().await;
    drop(session);
    drop(env);
    let reopened = Environment::persistent(&directory, &[31; 32]).unwrap();
    reopened
        .scope(async {
            let library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(library.items["title"].state.time_offset, 200);
            let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            assert!(!journal.has_pending());
        })
        .await;
    reopened.shutdown().await;
    drop(reopened);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn retry_writer_barrier_holds_receipt_and_newer_edit_until_save_finishes() {
    use futures::poll;
    use std::sync::atomic::AtomicBool;
    use stremio_core::runtime::EnvError;
    let env = stored_pending().await;
    let online = Arc::new(AtomicBool::new(false));
    let network = online.clone();
    *env.test_http.lock().unwrap() = Some(Arc::new(move |parts, _| {
        assert!(parts.uri.path().ends_with("/datastorePut"));
        if !network.load(Ordering::SeqCst) {
            return Err(EnvError::Fetch("fixture offline".into()));
        }
        Ok(br#"{"result":{"success":true}}"#.to_vec())
    }));
    let mut session = CoreSession::restore_account(env.clone(), "owner-a")
        .await
        .unwrap();
    for _ in 0..200 {
        if session.snapshot().error == Some("account_sync_failed") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(session.snapshot().sync.as_ref().unwrap().state, "waiting");
    let mut item = env
        .scope(async {
            NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap()
                .items["title"]
                .clone()
        })
        .await;
    item.state.time_offset = 200;
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
    {
        let retry = session.retry_sync();
        tokio::pin!(retry);
        assert!(poll!(&mut retry).is_pending());
        let edit = session.test_library_item(item);
        tokio::pin!(edit);
        assert!(poll!(&mut edit).is_pending());
        tokio::select! {
            result = &mut retry => panic!("Retry acknowledged before storage receipt: {result:?}"),
            result = &mut edit => panic!("Newer mutation crossed the retry write barrier: {result:?}"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(30)) => {},
        }
        online.store(true, Ordering::SeqCst);
        release.send(()).unwrap();
        retry.await.unwrap();
        edit.await.unwrap();
    }
    for _ in 0..200 {
        if session
            .snapshot()
            .sync
            .as_ref()
            .is_some_and(|sync| sync.state == "idle")
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(session.snapshot().sync.as_ref().unwrap().state, "idle");
    env.scope(async {
        let library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(library.items["title"].state.time_offset, 200);
        let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        assert!(!journal.has_pending());
    })
    .await;
    session.stop().await;
}
