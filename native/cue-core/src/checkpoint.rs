//! One local checkpoint replaces both Core library buckets atomically. No network effect.
use crate::environment::NativeEnv;
use std::collections::BTreeMap;
use stremio_core::{
    constants::{LIBRARY_RECENT_STORAGE_KEY, LIBRARY_STORAGE_KEY},
    runtime::{EnvError, TryEnvFuture},
    types::library::{LibraryBucket, LibraryBucketRef},
};
pub(crate) fn write(library: &LibraryBucket) -> Result<TryEnvFuture<()>, EnvError> {
    Ok(NativeEnv::set_storage_batch(batch(library)?))
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
