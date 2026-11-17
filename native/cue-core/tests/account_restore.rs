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
