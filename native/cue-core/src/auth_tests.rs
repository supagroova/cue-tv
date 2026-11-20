use crate::{environment::Environment, session::CoreSession};
use serde_json::json;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[tokio::test]
async fn rejected_login_keeps_guest_identity_and_redacts_server_errors() {
    let env = Environment::new().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let observed = requests.clone();
    *env.test_http.lock().unwrap() = Some(Arc::new(move |parts, body| {
        observed.lock().unwrap().push((
            parts.uri.to_string(),
            parts.method.clone(),
            serde_json::from_slice::<serde_json::Value>(body).unwrap(),
        ));
        Ok(serde_json::to_vec(&json!({"error":{"code":1,"message":"synthetic-password fixture@example.test rejected"}})).unwrap())
    }));
    let mut session = CoreSession::new_account(env.clone()).await.unwrap();
    session
        .login("fixture@example.test", "synthetic-password")
        .await
        .unwrap();
    for _ in 0..200 {
        if session.snapshot().error.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let state = session.snapshot();
    assert!(!state.authenticated);
    assert_eq!(state.error, Some("core_effect_failed"));
    let published = serde_json::to_string(&state).unwrap();
    assert!(!published.contains("synthetic-password"));
    assert!(!published.contains("fixture@example.test"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].1, http::Method::POST);
    assert!(requests[0].0.starts_with("https://"));
    assert_eq!(requests[0].2["email"], "fixture@example.test");
    assert_eq!(requests[0].2["password"], "synthetic-password");
    drop(requests);
    session.stop().await;
}

#[tokio::test]
async fn successful_login_persists_identity_and_rejects_account_replacement() {
    use crate::environment::NativeEnv;
    use stremio_core::{constants::PROFILE_STORAGE_KEY, runtime::Env, types::profile::Profile};
    let directory = std::env::temp_dir().join(format!(
        "cue-login-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let key = [91; 32];
    let env = Environment::persistent(&directory, &key).unwrap();
    *env.test_http.lock().unwrap() = Some(Arc::new(move |parts, _| {
        let result = if parts.uri.path().ends_with("/login") {
            json!({"result":{"authKey":"synthetic-token", "user":{
                "_id":"fixture-user", "email":"fixture@example.test",
                "lastModified":"2026-01-01T00:00:00Z", "dateRegistered":"2026-01-01T00:00:00Z",
                "premium_expire":null, "gdpr_consent":{"tos":true,"privacy":true,"marketing":false,"from":null}
            }}})
        } else if parts.uri.path().ends_with("/addonCollectionGet") {
            json!({"result":{"addons":[],"lastModified":"2026-01-01T00:00:00Z"}})
        } else if parts.uri.path().contains("datastore") {
            json!({"result":[]})
        } else {
            json!({"error":{"code":1,"message":"Fixture rejects unrelated requests"}})
        };
        Ok(serde_json::to_vec(&result).unwrap())
    }));
    let mut session = CoreSession::new_account(env.clone()).await.unwrap();
    assert_eq!(
        session.login("", "password").await,
        Err("invalid_credentials")
    );
    session
        .login("fixture@example.test", "synthetic-password")
        .await
        .unwrap();
    for _ in 0..200 {
        if session.snapshot().authenticated && session.snapshot().pending_effects == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(session.snapshot().authenticated);
    assert_eq!(session.account_identity().as_deref(), Some("fixture-user"));
    assert_eq!(
        session
            .login("other@example.test", "different-password")
            .await,
        Err("account_busy")
    );
    let profile = env
        .scope(async { NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY).await })
        .await
        .unwrap()
        .unwrap();
    let auth = profile.auth.unwrap();
    assert_eq!(
        serde_json::to_value(&auth.user.id).unwrap(),
        json!("fixture-user")
    );
    let published = serde_json::to_string(&session.snapshot()).unwrap();
    for secret in [
        "fixture-user",
        "synthetic-token",
        "fixture@example.test",
        "synthetic-password",
    ] {
        assert!(!published.contains(secret));
    }
    session.stop().await;
    assert_eq!(session.account_identity(), None);
    drop(session);
    drop(env);
    let ciphertext = std::fs::read(directory.join("state.bin")).unwrap();
    for secret in [
        "fixture-user",
        "synthetic-token",
        "fixture@example.test",
        "synthetic-password",
    ] {
        assert!(
            !ciphertext
                .windows(secret.len())
                .any(|bytes| bytes == secret.as_bytes())
        );
    }
    let restored_env = Environment::persistent(&directory, &key).unwrap();
    *restored_env.test_http.lock().unwrap() = Some(Arc::new(|_, _| {
        Ok(br#"{"result":{"success":true}}"#.to_vec())
    }));
    let mut restored = CoreSession::restore_account(restored_env.clone(), "fixture-user")
        .await
        .unwrap();
    assert!(restored.snapshot().authenticated);
    assert_eq!(
        restored.login("other@example.test", "password").await,
        Err("account_login_unavailable")
    );
    restored.logout().await.unwrap();
    for _ in 0..200 {
        if !restored.snapshot().authenticated && restored.snapshot().pending_effects == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!restored.snapshot().authenticated);
    assert_eq!(restored.account_identity(), None);
    assert_eq!(
        restored.login("other@example.test", "password").await,
        Err("account_login_unavailable")
    );
    let profile = restored_env
        .scope(async { NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY).await })
        .await
        .unwrap()
        .unwrap();
    assert!(profile.auth.is_none());
    restored_env
        .scope(async {
            for name in [
                "library",
                "library_recent",
                "streams",
                "streaming_server_urls",
                "notifications",
                "search_history",
                "dismissed_events",
            ] {
                let value = NativeEnv::get_storage::<serde_json::Value>(name)
                    .await
                    .unwrap();
                if let Some(value) = value {
                    assert!(value["uid"].is_null(), "retained identity in {name}");
                }
            }
        })
        .await;
    restored.stop().await;
    drop(restored);
    drop(restored_env);
    assert!(
        CoreSession::restore_account(
            Environment::persistent(&directory, &key).unwrap(),
            "fixture-user"
        )
        .await
        .is_err()
    );
    assert!(
        CoreSession::new_account(Environment::persistent(&directory, &key).unwrap())
            .await
            .is_err()
    );
    let mut guest = CoreSession::guest(Environment::persistent(&directory, &key).unwrap())
        .await
        .unwrap();
    assert!(!guest.snapshot().authenticated);
    guest.stop().await;
    drop(guest);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn guest_cannot_admit_account_credentials() {
    let env = Environment::new().unwrap();
    *env.test_http.lock().unwrap() =
        Some(Arc::new(|_, _| panic!("Guest must not send login traffic")));
    let mut guest = CoreSession::guest(env).await.unwrap();
    assert_eq!(
        guest
            .login("fixture@example.test", "synthetic-password")
            .await,
        Err("account_login_unavailable")
    );
    assert!(!guest.snapshot().authenticated);
    guest.stop().await;
}
