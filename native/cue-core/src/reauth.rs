//! Obtain replacement credentials without applying Core's new-account state reset.
use crate::environment::NativeEnv;
use stremio_core::types::{
    api::{APIRequest, APIResult, AuthRequest, AuthResponse, fetch_api},
    profile::Auth,
};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    InvalidRequest,
    Identity,
    Rejected,
    Transport,
}

pub(crate) async fn authenticate(
    expected_uid: &str,
    email: &str,
    password: &str,
) -> Result<Auth, Error> {
    if crate::sync_journal::Journal::new(expected_uid).is_err()
        || email.is_empty()
        || email.len() > 512
        || email.chars().any(char::is_control)
        || password.is_empty()
        || password.len() > 4096
        || password.chars().any(char::is_control)
    {
        return Err(Error::InvalidRequest);
    }
    let request = APIRequest::Auth(AuthRequest::Login {
        email: email.to_owned(),
        password: password.to_owned(),
        facebook: false,
    });
    // Do not log request/response values: both carry private account information.
    let response = match fetch_api::<NativeEnv, _, _, AuthResponse>(&request).await {
        Ok(APIResult::Ok(response)) => response,
        Ok(APIResult::Err(_)) => return Err(Error::Rejected),
        Err(stremio_core::runtime::EnvError::Fetch(message))
            if message == crate::environment::AUTH_REJECTED =>
        {
            return Err(Error::Rejected);
        }
        Err(_) => return Err(Error::Transport),
    };
    if response.user.id.0 != expected_uid
        || response.key.0.is_empty()
        || response.key.0.len() > 4096
    {
        return Err(Error::Identity);
    }
    Ok(Auth {
        key: response.key,
        user: response.user,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Environment;
    use std::sync::Arc;

    fn response(uid: &str) -> Vec<u8> {
        serde_json::to_vec(
            &serde_json::json!({"result":{"authKey":"replacement-token","user":{
                "_id":uid,"email":"fixture@example.test","lastModified":"2026-01-01T00:00:00Z",
                "dateRegistered":"2026-01-01T00:00:00Z","premium_expire":null,
                "gdpr_consent":{"tos":true,"privacy":true,"marketing":false,"from":null}
            }}}),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn refresh_accepts_only_same_uid_and_sanitizes_rejection() {
        let env = Environment::new().unwrap();
        *env.test_http.lock().unwrap() = Some(Arc::new(|parts, bytes| {
            assert_eq!(parts.method, http::Method::POST);
            assert_eq!(parts.uri.scheme_str(), Some("https"));
            assert!(parts.uri.path().ends_with("/login"));
            let request: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            assert_eq!(request["email"], "fixture@example.test");
            assert_eq!(request["password"], "fixture-password");
            Ok(response("owner-a"))
        }));
        env.scope(async {
            let auth = authenticate("owner-a", "fixture@example.test", "fixture-password")
                .await
                .unwrap();
            assert_eq!(auth.user.id.0, "owner-a");
            assert_eq!(auth.key.0, "replacement-token");
            assert!(matches!(
                authenticate("owner-b", "fixture@example.test", "fixture-password").await,
                Err(Error::Identity)
            ));
        })
        .await;
        *env.test_http.lock().unwrap() = Some(Arc::new(|_, _| {
            Ok(
                br#"{"error":{"code":1,"message":"fixture-password replacement-token rejected"}}"#
                    .to_vec(),
            )
        }));
        env.scope(async {
            assert!(matches!(
                authenticate("owner-a", "fixture@example.test", "fixture-password").await,
                Err(Error::Rejected)
            ));
            assert!(matches!(
                authenticate("owner-a", "", "fixture-password").await,
                Err(Error::InvalidRequest)
            ));
        })
        .await;
        env.shutdown().await;
    }
    #[tokio::test]
    async fn credentials_and_pending_library_commit_atomically_and_reopen() {
        use crate::sync_journal::Journal;
        use stremio_core::{
            constants::*,
            runtime::Env,
            types::{library::LibraryBucket, profile::Profile},
        };
        let directory = std::env::temp_dir().join(format!(
            "cue-reauth-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir(&directory).unwrap();
        let unavailable = directory.with_extension("unavailable");
        let env = crate::sync_session_tests::seed_pending(
            Environment::persistent(&directory, &[37; 32]).unwrap(),
        )
        .await;
        env.scope(async {
            let profile = NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            let mut library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            let mut auth = profile.auth.clone().unwrap();
            auth.key.0 = "replacement-token".into();
            library.items.get_mut("title").unwrap().state.time_offset = 200;
            let mut foreign = auth.clone();
            foreign.user.id = "owner-b".parse().unwrap();
            assert!(
                crate::checkpoint::refresh_account(&profile, &foreign, &library, Some(&journal))
                    .is_err()
            );
            let (_, write) =
                crate::checkpoint::refresh_account(&profile, &auth, &library, Some(&journal))
                    .unwrap();
            std::fs::rename(&directory, &unavailable).unwrap();
            assert!(write.await.is_err());
            assert_eq!(
                NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY)
                    .await
                    .unwrap()
                    .unwrap()
                    .auth
                    .unwrap()
                    .key
                    .0,
                "synthetic-token"
            );
            assert_eq!(
                NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                    .await
                    .unwrap()
                    .unwrap()
                    .items["title"]
                    .state
                    .time_offset,
                100
            );
            std::fs::rename(&unavailable, &directory).unwrap();
            let (next, write) =
                crate::checkpoint::refresh_account(&profile, &auth, &library, Some(&journal))
                    .unwrap();
            assert_eq!(next.auth.as_ref().unwrap().key.0, "replacement-token");
            assert_eq!(profile.auth.as_ref().unwrap().key.0, "synthetic-token");
            write.await.unwrap();
        })
        .await;
        env.shutdown().await;
        drop(env);
        let env = Environment::persistent(&directory, &[37; 32]).unwrap();
        env.scope(async {
            let profile = NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(profile.auth.unwrap().key.0, "replacement-token");
            let library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(library.items["title"].state.time_offset, 200);
            let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(journal.revision("title"), Some(1));
        })
        .await;
        let ciphertext = std::fs::read(directory.join("state.bin")).unwrap();
        assert!(!String::from_utf8_lossy(&ciphertext).contains("replacement-token"));
        env.shutdown().await;
        drop(env);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
