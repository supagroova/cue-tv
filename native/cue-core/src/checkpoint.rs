//! One local checkpoint replaces both Core library buckets atomically. No network effect.
use crate::environment::NativeEnv;
use futures::FutureExt;
use std::collections::BTreeMap;
use stremio_core::{
    constants::{LIBRARY_RECENT_STORAGE_KEY, LIBRARY_STORAGE_KEY},
    runtime::{Env, EnvError, TryEnvFuture},
    types::library::{LibraryBucket, LibraryBucketRef},
};
/// Execute on Core's sequential storage queue: read the last committed revision
/// there, rather than capturing a stale journal when the checkpoint is prepared.
pub(crate) fn write(library: &LibraryBucket, title_id: &str) -> Result<TryEnvFuture<()>, EnvError> {
    let Some(uid) = library.uid.as_ref() else {
        return Ok(NativeEnv::set_storage_batch(batch(library)?));
    };
    let item = library
        .items
        .get(title_id)
        .ok_or(EnvError::StorageUnavailable)?;
    let uid = uid.0.clone();
    let title_id = title_id.to_owned();
    let kind = item.r#type.clone();
    let library = library.clone();
    // Capture the environment while constructing the operation, as other Env calls do.
    let stored =
        NativeEnv::get_storage::<crate::sync_journal::Journal>(crate::sync_journal::STORAGE_KEY);
    Ok(async move {
        let journal = match stored.await? {
            Some(journal) => journal,
            None => {
                crate::sync_journal::Journal::new(&uid).map_err(|_| EnvError::StorageUnavailable)?
            }
        };
        journal
            .validate_library(&library)
            .map_err(|_| EnvError::StorageUnavailable)?;
        let next = journal
            .with_intent(&uid, &kind, &title_id)
            .map_err(|_| EnvError::StorageUnavailable)?;
        write_with_journal(&library, &next)?.await
    }
    .boxed())
}

pub(crate) fn write_with_journal(
    library: &LibraryBucket,
    journal: &crate::sync_journal::Journal,
) -> Result<TryEnvFuture<()>, EnvError> {
    journal
        .validate_library(library)
        .map_err(|_| EnvError::Serde("Invalid journal identity".into()))?;
    let mut values = batch(library)?;
    let bytes = journal
        .encode()
        .map_err(|_| EnvError::Serde("Invalid sync journal".into()))?;
    values.insert(crate::sync_journal::STORAGE_KEY.into(), Some(bytes));
    Ok(NativeEnv::set_storage_batch(values))
}

/// Run on the sequential storage queue, after preceding writes. Never build an
/// outbound request from session state whose checkpoint may still fail.
pub(crate) async fn committed_upload(
    expected_uid: &str,
) -> Result<Option<crate::sync_journal::Upload>, EnvError> {
    let Some(journal) =
        NativeEnv::get_storage::<crate::sync_journal::Journal>(crate::sync_journal::STORAGE_KEY)
            .await?
    else {
        return Ok(None);
    };
    let uid = Some(stremio_core::types::profile::UserId(expected_uid.into()));
    let mut library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_STORAGE_KEY)
        .await?
        .unwrap_or_else(|| LibraryBucket::new(uid.clone(), vec![]));
    let recent = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
        .await?
        .unwrap_or_else(|| LibraryBucket::new(uid.clone(), vec![]));
    if library.uid != uid || recent.uid != uid {
        return Err(EnvError::StorageUnavailable);
    }
    library.merge_bucket(recent);
    journal
        .next_upload(&library)
        .map_err(|_| EnvError::StorageUnavailable)
}

fn batch(library: &LibraryBucket) -> Result<crate::environment::StorageBatch, EnvError> {
    let (recent, older) = library.split_items_by_recent();
    let encode = |items: &[_]| {
        serde_json::to_vec(&LibraryBucketRef::new(&library.uid, items))
            .map_err(|_| EnvError::Serde("Invalid library checkpoint".into()))
    };
    let recent = encode(&recent)?;
    let older = if older.is_empty() {
        None
    } else {
        Some(encode(&older)?)
    };
    Ok(BTreeMap::from([
        (LIBRARY_RECENT_STORAGE_KEY.to_owned(), Some(recent)),
        (LIBRARY_STORAGE_KEY.to_owned(), older),
    ]))
}

#[cfg(test)]
mod journal_tests {
    use super::*;
    use crate::{environment::Environment, sync_journal::Journal};
    use stremio_core::runtime::Env;

    fn library(position: u64) -> LibraryBucket {
        let item = serde_json::from_value(serde_json::json!({
            "_id":"title", "name":"Owned fixture", "type":"movie", "removed":false, "temp":false,
            "_mtime":"2026-01-01T00:00:00Z", "state":{
                "timeWatched":0, "timeOffset":position, "overallTimeWatched":0,
                "timesWatched":0, "flaggedWatched":0, "duration":1000
            }
        }))
        .unwrap();
        LibraryBucket::new(Some("owner-a".parse().unwrap()), vec![item])
    }

    #[tokio::test]
    async fn upload_reads_committed_state_and_rejects_another_account() {
        let env = Environment::new().unwrap();
        env.scope(async {
            assert!(committed_upload("owner-a").await.unwrap().is_none());
            write(&library(100), "title").unwrap().await.unwrap();
            let not_saved = write(&library(200), "title").unwrap();
            let upload = committed_upload("owner-a").await.unwrap().unwrap();
            assert_eq!(upload.item().state.time_offset, 100);
            assert!(committed_upload("owner-b").await.is_err());
            not_saved.await.unwrap();
            assert_eq!(
                committed_upload("owner-a")
                    .await
                    .unwrap()
                    .unwrap()
                    .item()
                    .state
                    .time_offset,
                200
            );
            assert_eq!(upload.item().state.time_offset, 100);
        })
        .await;
        env.shutdown().await;
    }

    #[tokio::test]
    async fn account_checkpoint_records_intent_and_coalesces_repeated_progress() {
        let env = Environment::new().unwrap();
        env.scope(async {
            let first = write(&library(100), "title").unwrap();
            let second = write(&library(200), "title").unwrap();
            first.await.unwrap();
            second.await.unwrap();
            let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                .await
                .unwrap()
                .expect("Account progress requires durable pending intent");
            assert_eq!(journal.revision("title"), Some(2));
            let stored = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(stored.items["title"].state.time_offset, 200);
            journal.validate_library(&stored).unwrap();
        })
        .await;
        env.shutdown().await;
    }

    #[tokio::test]
    async fn checkpoint_rejects_foreign_journal_without_changing_progress() {
        let env = Environment::new().unwrap();
        env.scope(async {
            let foreign = Journal::new("owner-b").unwrap();
            NativeEnv::set_storage(LIBRARY_RECENT_STORAGE_KEY, Some(&library(100)))
                .await
                .unwrap();
            NativeEnv::set_storage(crate::sync_journal::STORAGE_KEY, Some(&foreign))
                .await
                .unwrap();
            assert!(write(&library(200), "title").unwrap().await.is_err());
            let stored = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(stored.items["title"].state.time_offset, 100);
            assert_eq!(
                NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                    .await
                    .unwrap(),
                Some(foreign)
            );
        })
        .await;
        env.shutdown().await;
    }

    #[tokio::test]
    async fn guest_checkpoint_does_not_create_account_intent() {
        let env = Environment::new().unwrap();
        env.scope(async {
            let mut guest = library(123);
            guest.uid = None;
            write(&guest, "title").unwrap().await.unwrap();
            assert!(
                NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                    .await
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                    .await
                    .unwrap()
                    .unwrap()
                    .items["title"]
                    .state
                    .time_offset,
                123
            );
        })
        .await;
        env.shutdown().await;
    }

    #[tokio::test]
    async fn journal_and_library_commit_or_fail_together_across_encrypted_reopen() {
        let directory = std::env::temp_dir().join(format!(
            "cue-journal-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        std::fs::create_dir(&directory).unwrap();
        let unavailable = directory.with_extension("unavailable");
        let first = Journal::new("owner-a")
            .unwrap()
            .with_intent("owner-a", "movie", "title")
            .unwrap();
        let second = first.with_intent("owner-a", "movie", "title").unwrap();
        let env = Environment::persistent(&directory, &[9; 32]).unwrap();
        env.scope(async {
            write_with_journal(&library(100), &first)
                .unwrap()
                .await
                .unwrap();
            std::fs::rename(&directory, &unavailable).unwrap();
            assert!(
                write_with_journal(&library(200), &second)
                    .unwrap()
                    .await
                    .is_err()
            );
            let stored = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(stored.items["title"].state.time_offset, 100);
            assert_eq!(journal.revision("title"), Some(1));
            std::fs::rename(&unavailable, &directory).unwrap();
        })
        .await;
        env.shutdown().await;
        drop(env);
        for (expected_position, expected_revision) in [(100, 1), (200, 2)] {
            let env = Environment::persistent(&directory, &[9; 32]).unwrap();
            env.scope(async {
                let stored = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                    .await
                    .unwrap()
                    .unwrap();
                let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(stored.items["title"].state.time_offset, expected_position);
                assert_eq!(journal.revision("title"), Some(expected_revision));
                write_with_journal(&library(200), &second)
                    .unwrap()
                    .await
                    .unwrap();
            })
            .await;
            env.shutdown().await;
            drop(env);
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn quota_failure_cannot_commit_library_without_its_pending_revision() {
        let first = Journal::new("owner-a")
            .unwrap()
            .with_intent("owner-a", "movie", "title")
            .unwrap();
        let next = first.with_intent("owner-a", "movie", "second").unwrap();
        let mut candidate = library(200);
        let mut second = candidate.items["title"].clone();
        second.id = "second".into();
        candidate.items.insert(second.id.clone(), second);
        let candidate_size: usize = batch(&candidate)
            .unwrap()
            .iter()
            .filter_map(|(key, value)| value.as_ref().map(|value| key.len() + value.len()))
            .sum();
        // New library alone fits with the old journal; the complete new pair does not.
        let remaining = 16 * 1024 * 1024
            - candidate_size
            - crate::sync_journal::STORAGE_KEY.len()
            - first.encode().unwrap().len()
            - "filler".len()
            - 1;
        let env = Environment::new().unwrap();
        env.scope(async {
            write_with_journal(&library(100), &first)
                .unwrap()
                .await
                .unwrap();
            NativeEnv::set_storage_batch(BTreeMap::from([(
                "filler".into(),
                Some(serde_json::to_vec(&"x".repeat(remaining - 2)).unwrap()),
            )]))
            .await
            .unwrap();
            assert!(
                write_with_journal(&candidate, &next)
                    .unwrap()
                    .await
                    .is_err()
            );
            let stored = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(stored.items["title"].state.time_offset, 100);
            assert!(!stored.items.contains_key("second"));
            assert_eq!(journal.revision("title"), Some(1));
            assert_eq!(journal.revision("second"), None);
        })
        .await;
        env.shutdown().await;
    }

    #[test]
    fn journal_cannot_be_paired_with_guest_other_owner_or_missing_title() {
        let journal = Journal::new("owner-a")
            .unwrap()
            .with_intent("owner-a", "movie", "title")
            .unwrap();
        let mut wrong = library(100);
        wrong.uid = None;
        assert!(write_with_journal(&wrong, &journal).is_err());
        wrong.uid = Some("owner-b".parse().unwrap());
        assert!(write_with_journal(&wrong, &journal).is_err());
        wrong.uid = Some("owner-a".parse().unwrap());
        wrong.items.get_mut("title").unwrap().r#type = "series".into();
        assert!(write_with_journal(&wrong, &journal).is_err());
        wrong.items.clear();
        assert!(write_with_journal(&wrong, &journal).is_err());
    }
}
