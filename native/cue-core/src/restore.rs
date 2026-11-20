//! Rebuild Core state from an identity-scoped store; reject mixed-identity buckets.
use crate::environment::NativeEnv;
use stremio_core::{
    constants::*,
    models::ctx::Ctx,
    runtime::{Env, EnvError},
    types::{
        events::DismissedEventsBucket, library::LibraryBucket, notifications::NotificationsBucket,
        profile::Profile, search_history::SearchHistoryBucket, server_urls::ServerUrlsBucket,
        streams::StreamsBucket,
    },
};

pub(crate) async fn context(profile: Profile) -> Result<Ctx, EnvError> {
    let uid = profile.uid();
    let mut library = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_STORAGE_KEY)
        .await?
        .unwrap_or_else(|| LibraryBucket::new(uid.clone(), vec![]));
    let recent = NativeEnv::get_storage::<LibraryBucket>(LIBRARY_RECENT_STORAGE_KEY)
        .await?
        .unwrap_or_else(|| LibraryBucket::new(uid.clone(), vec![]));
    let streams = NativeEnv::get_storage::<StreamsBucket>(STREAMS_STORAGE_KEY)
        .await?
        .unwrap_or_else(|| StreamsBucket::new(uid.clone()));
    let servers = NativeEnv::get_storage::<ServerUrlsBucket>(STREAMING_SERVER_URLS_STORAGE_KEY)
        .await?
        .unwrap_or_else(|| ServerUrlsBucket::new::<NativeEnv>(uid.clone()));
    let notifications = NativeEnv::get_storage::<NotificationsBucket>(NOTIFICATIONS_STORAGE_KEY)
        .await?
        .unwrap_or_else(|| NotificationsBucket::new::<NativeEnv>(uid.clone(), vec![]));
    let searches = NativeEnv::get_storage::<SearchHistoryBucket>(SEARCH_HISTORY_STORAGE_KEY)
        .await?
        .unwrap_or_else(|| SearchHistoryBucket::new(uid.clone()));
    let dismissed = NativeEnv::get_storage::<DismissedEventsBucket>(DISMISSED_EVENTS_STORAGE_KEY)
        .await?
        .unwrap_or_else(|| DismissedEventsBucket::new(uid.clone()));
    if [
        &library.uid,
        &recent.uid,
        &streams.uid,
        &servers.uid,
        &notifications.uid,
        &searches.uid,
        &dismissed.uid,
    ]
    .iter()
    .any(|stored| **stored != uid)
    {
        return Err(EnvError::StorageUnavailable);
    }
    library.merge_bucket(recent);
    // A pending revision is meaningful only alongside the same owner's full library.
    // Never discard a malformed or foreign journal and continue as if sync succeeded.
    if let Some(journal) =
        NativeEnv::get_storage::<crate::sync_journal::Journal>(crate::sync_journal::STORAGE_KEY)
            .await?
    {
        journal
            .validate_library(&library)
            .map_err(|_| EnvError::StorageUnavailable)?;
    }
    Ok(Ctx::new(
        profile,
        library,
        streams,
        servers,
        notifications,
        searches,
        dismissed,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Environment;
    use stremio_core::types::{library::LibraryItem, profile::UserId};
    fn item(year: i32, position: u64) -> LibraryItem {
        let state = stremio_core::types::library::LibraryItemState {
            time_offset: position,
            ..Default::default()
        };
        serde_json::from_value(serde_json::json!({"_id":"fixture", "name":"Owned fixture", "type":"movie", "removed":false,"temp":false,
            "_mtime":format!("{year}-01-01T00:00:00Z"), "state":state})).unwrap()
    }
    #[tokio::test]
    async fn guest_restore_rejects_account_journal_without_erasing_it() {
        let env = Environment::new().unwrap();
        env.scope(async {
            let journal = crate::sync_journal::Journal::new("other-account").unwrap();
            NativeEnv::set_storage(crate::sync_journal::STORAGE_KEY, Some(&journal))
                .await
                .unwrap();
            assert!(
                context(Profile::default()).await.is_err(),
                "An account journal must not be silently ignored by guest restoration"
            );
            let retained = NativeEnv::get_storage::<crate::sync_journal::Journal>(
                crate::sync_journal::STORAGE_KEY,
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(retained, journal);
        })
        .await;
        env.shutdown().await;
    }

    #[tokio::test]
    async fn merges_recent_progress_and_restores_history_without_changing_storage() {
        let env = Environment::new().unwrap();
        env.scope(async {
            let library = LibraryBucket::new(None, vec![item(2020, 10)]);
            let recent = LibraryBucket::new(None, vec![item(2021, 42)]);
            let mut searches = SearchHistoryBucket::new(None);
            searches
                .items
                .insert("fixture query".into(), chrono::Utc::now());
            NativeEnv::set_storage(LIBRARY_STORAGE_KEY, Some(&library))
                .await
                .unwrap();
            NativeEnv::set_storage(LIBRARY_RECENT_STORAGE_KEY, Some(&recent))
                .await
                .unwrap();
            NativeEnv::set_storage(SEARCH_HISTORY_STORAGE_KEY, Some(&searches))
                .await
                .unwrap();
            let ctx = context(Profile::default()).await.unwrap();
            assert_eq!(ctx.library.items["fixture"].state.time_offset, 42);
            assert!(ctx.search_history.items.contains_key("fixture query"));
            assert_eq!(
                NativeEnv::get_storage::<LibraryBucket>(LIBRARY_STORAGE_KEY)
                    .await
                    .unwrap()
                    .unwrap()
                    .items["fixture"]
                    .state
                    .time_offset,
                10
            );
        })
        .await;
        env.shutdown().await;
    }
    #[tokio::test]
    async fn every_identity_bucket_rejects_a_foreign_owner() {
        for key in [
            LIBRARY_STORAGE_KEY,
            LIBRARY_RECENT_STORAGE_KEY,
            STREAMS_STORAGE_KEY,
            STREAMING_SERVER_URLS_STORAGE_KEY,
            NOTIFICATIONS_STORAGE_KEY,
            SEARCH_HISTORY_STORAGE_KEY,
            DISMISSED_EVENTS_STORAGE_KEY,
        ] {
            let env = Environment::new().unwrap();
            env.scope(async {
                let mut value =
                    serde_json::json!({"uid":null, "items":{}, "created":"2026-01-01T00:00:00Z"});
                if key == STREAMS_STORAGE_KEY {
                    value["items"] = serde_json::json!([]);
                }
                NativeEnv::set_storage(key, Some(&value)).await.unwrap();
                assert!(
                    context(Profile::default()).await.is_ok(),
                    "valid bucket rejected: {key}"
                );
                value["uid"] = serde_json::json!("foreign-owner");
                NativeEnv::set_storage(key, Some(&value)).await.unwrap();
                assert!(
                    context(Profile::default()).await.is_err(),
                    "foreign bucket accepted: {key}"
                );
            })
            .await;
            env.shutdown().await;
        }
    }
    #[tokio::test]
    async fn rejects_mixed_identity_instead_of_silently_merging() {
        let env = Environment::new().unwrap();
        env.scope(async {
            NativeEnv::set_storage(
                STREAMS_STORAGE_KEY,
                Some(&StreamsBucket::new(Some(UserId("other-account".into())))),
            )
            .await
            .unwrap();
            assert!(context(Profile::default()).await.is_err());
        })
        .await;
        env.shutdown().await;
    }
}
