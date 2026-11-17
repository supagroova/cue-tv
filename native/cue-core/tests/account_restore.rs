use cue_core::{
    environment::{Environment, NativeEnv},
    session::CoreSession,
};
use stremio_core::{
    constants::{PROFILE_STORAGE_KEY, SCHEMA_VERSION, SCHEMA_VERSION_STORAGE_KEY},
    runtime::Env,
    types::profile::Profile,
};

async fn stored_account() -> std::sync::Arc<Environment> {
    let env = Environment::new().unwrap();
    let mut profile = Profile::default();
    profile.auth = Some(serde_json::from_value(serde_json::json!({
        "key":"synthetic-secret-token",
        "user":{"_id":"fixture-user", "email":"fixture@example.test",
            "lastModified":"2026-01-01T00:00:00Z", "dateRegistered":"2026-01-01T00:00:00Z",
            "premium_expire":null, "gdpr_consent":{"tos":true,"privacy":true,"marketing":false,"from":null}}
    })).unwrap());
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
            .await
            .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
    })
    .await;
    env
}

#[tokio::test]
async fn account_restore_requires_matching_identity_and_keeps_secrets_out_of_snapshot() {
    let mut session = CoreSession::restore_account(stored_account().await, "fixture-user")
        .await
        .unwrap();
    let snapshot = serde_json::to_string(&session.snapshot()).unwrap();
    assert!(session.snapshot().authenticated);
    for private in [
        "synthetic-secret-token",
        "fixture@example.test",
        "fixture-user",
    ] {
        assert!(!snapshot.contains(private));
    }
    session.stop().await;
    assert!(
        CoreSession::restore_account(stored_account().await, "different-user")
            .await
            .is_err()
    );
    assert!(CoreSession::guest(stored_account().await).await.is_err());
    assert!(
        CoreSession::restore_account(Environment::new().unwrap(), "fixture-user")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn ffi_account_restore_copies_identity_and_releases_encrypted_store() {
    use cue_core::ffi::*;
    let source = stored_account().await;
    let profile = source
        .scope(async { NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY).await })
        .await
        .unwrap()
        .unwrap();
    source.shutdown().await;
    let directory = std::env::temp_dir().join(format!(
        "cue-core-account-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let key = [73; 32];
    let env = Environment::persistent(directory.as_path(), &key).unwrap();
    env.scope(async {
        NativeEnv::set_storage(SCHEMA_VERSION_STORAGE_KEY, Some(&SCHEMA_VERSION))
            .await
            .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&profile))
            .await
            .unwrap();
    })
    .await;
    env.shutdown().await;
    drop(env);
    for (user, expected) in [("wrong-user", "stopped"), ("fixture-user", "ready")] {
        let path = directory.as_path().to_str().unwrap().as_bytes().to_vec();
        let mut identity = user.as_bytes().to_vec();
        let id = unsafe {
            cue_core_restore_account(
                path.as_ptr(),
                path.len(),
                key.as_ptr(),
                key.len(),
                identity.as_ptr(),
                identity.len(),
            )
        };
        assert_ne!(id, 0);
        identity.fill(b'x'); // Startup must own its copy, not borrow caller memory.
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let mut bytes = vec![0; 256 * 1024];
            let size = unsafe { cue_core_snapshot(id, bytes.as_mut_ptr(), bytes.len()) };
            assert!(size > 0 && size as usize <= bytes.len());
            let text = std::str::from_utf8(&bytes[..size as usize]).unwrap();
            assert!(!text.contains("synthetic-secret-token"));
            let value: serde_json::Value = serde_json::from_str(text).unwrap();
            if value["status"] != "starting" {
                assert_eq!(value["status"], expected);
                if expected == "ready" {
                    assert_eq!(value["model"]["authenticated"], true);
                }
                break;
            }
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        cue_core_stop(id);
        while cue_core_stop_status(id) == 0 {
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(cue_core_release(id), 1);
    }
    std::fs::remove_dir_all(directory).unwrap();
}
