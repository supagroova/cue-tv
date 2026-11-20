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
    let env = Environment::new().unwrap();
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
