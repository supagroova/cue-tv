//! One bounded upload. Scheduling, retry and acknowledgement persistence are separate.
use crate::sync_journal::Upload;
use stremio_core::types::profile::Auth;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Identity,
    Transport,
    Rejected,
}

pub(crate) async fn send(upload: &Upload, auth: &Auth) -> Result<(), Error> {
    use stremio_core::types::api::{
        APIResult, DatastoreCommand, DatastoreRequest, SuccessResponse, fetch_api,
    };
    if !upload.belongs_to(&auth.user.id.0) || auth.key.0.is_empty() || auth.key.0.len() > 4096 {
        return Err(Error::Identity);
    }
    let request = DatastoreRequest {
        auth_key: auth.key.clone(),
        collection: stremio_core::constants::LIBRARY_COLLECTION_NAME.into(),
        command: DatastoreCommand::Put {
            changes: vec![upload.item().clone()],
        },
    };
    match fetch_api::<crate::environment::NativeEnv, _, _, SuccessResponse>(&request).await {
        Ok(APIResult::Ok(_)) => Ok(()),
        Ok(APIResult::Err(_)) => Err(Error::Rejected),
        Err(stremio_core::runtime::EnvError::Fetch(message))
            if message == crate::environment::AUTH_REJECTED =>
        {
            Err(Error::Rejected)
        }
        Err(_) => Err(Error::Transport),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{environment::Environment, sync_journal::Journal};
    use std::sync::{Arc, Mutex};
    use stremio_core::types::library::LibraryBucket;

    #[tokio::test]
    async fn sends_exact_snapshot_and_rejects_foreign_credentials_before_http() {
        let env = Environment::new().unwrap();
        let observed = Arc::new(Mutex::new(Vec::new()));
        let requests = observed.clone();
        *env.test_http.lock().unwrap() = Some(Arc::new(move |parts, bytes| {
            assert_eq!(parts.method, http::Method::POST);
            assert_eq!(parts.uri.scheme_str(), Some("https"));
            assert!(parts.uri.path().ends_with("/datastorePut"));
            requests
                .lock()
                .unwrap()
                .push(serde_json::from_slice::<serde_json::Value>(bytes).unwrap());
            Ok(br#"{"result":{"success":true}}"#.to_vec())
        }));
        let mut auth: Auth = serde_json::from_value(serde_json::json!({
            "key":"synthetic-token", "user":{"_id":"owner-a", "email":"fixture@example.test",
            "lastModified":"2026-01-01T00:00:00Z", "dateRegistered":"2026-01-01T00:00:00Z",
            "premium_expire":null,"gdpr_consent":{"tos":true,"privacy":true,"marketing":false,"from":null}}
        })).unwrap();
        let item = serde_json::from_value(
            serde_json::json!({"_id":"title","type":"movie","name":"Fixture",
            "removed":false,"temp":false,"_mtime":"2026-01-01T00:00:00Z","state":{
            "timeWatched":0,"timeOffset":100,"overallTimeWatched":0,"timesWatched":0,
            "flaggedWatched":0,"duration":1000}}),
        )
        .unwrap();
        let mut library = LibraryBucket::new(Some("owner-a".parse().unwrap()), vec![item]);
        let journal = Journal::new("owner-a")
            .unwrap()
            .with_intent("owner-a", "movie", "title")
            .unwrap();
        let upload = journal.next_upload(&library).unwrap().unwrap();
        library.items.get_mut("title").unwrap().state.time_offset = 200;
        env.scope(async {
            assert_eq!(send(&upload, &auth).await, Ok(()));
            auth.user.id = "owner-b".parse().unwrap();
            assert_eq!(send(&upload, &auth).await, Err(Error::Identity));
        })
        .await;
        let requests = observed.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["authKey"], "synthetic-token");
        assert_eq!(requests[0]["collection"], "libraryItem");
        assert_eq!(requests[0]["changes"].as_array().unwrap().len(), 1);
        assert_eq!(requests[0]["changes"][0]["state"]["timeOffset"], 100);
        assert_eq!(
            journal.revision("title"),
            Some(1),
            "Transport success is not a durable acknowledgement"
        );
        drop(requests);
        auth.user.id = "owner-a".parse().unwrap();
        for (body, expected) in [
            (
                br#"{"error":{"code":1,"message":"synthetic-token private-title"}}"#.to_vec(),
                Error::Rejected,
            ),
            (
                br#"{"result":{"success":false}}"#.to_vec(),
                Error::Transport,
            ),
            (b"not-json".to_vec(), Error::Transport),
        ] {
            *env.test_http.lock().unwrap() = Some(Arc::new(move |_, _| Ok(body.clone())));
            let result = env.scope(send(&upload, &auth)).await;
            assert_eq!(result, Err(expected));
            assert!(!format!("{result:?}").contains("synthetic-token"));
            assert_eq!(journal.revision("title"), Some(1));
        }
        *env.test_http.lock().unwrap() = Some(Arc::new(|_, _| {
            Err(stremio_core::runtime::EnvError::Fetch(
                "HTTP authentication rejected".into(),
            ))
        }));
        assert_eq!(env.scope(send(&upload, &auth)).await, Err(Error::Rejected));
        assert_eq!(journal.revision("title"), Some(1));
        env.shutdown().await;
    }
}
