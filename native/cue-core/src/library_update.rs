//! Keep Core's library mutation rules while replacing account write effects.
use crate::environment::NativeEnv;
use stremio_core::{
    models::ctx::Ctx,
    runtime::{Effects, Update, msg::Msg},
};

pub(crate) fn update(ctx: &mut Ctx, message: &Msg, retry_title: Option<&str>) -> Effects {
    use futures::FutureExt;
    use stremio_core::{
        models::ctx::CtxError,
        runtime::{
            EffectFuture, EnvError,
            msg::{Event, Internal},
        },
    };
    let title = if ctx.profile.auth.is_some() {
        match message {
            Msg::Internal(Internal::UpdateLibraryItem(item)) => Some(item.id.clone()),
            Msg::Internal(Internal::LibraryChanged(false)) => retry_title.map(str::to_owned),
            _ => None,
        }
    } else {
        None
    };
    // Core computes membership, watched flags and timestamps. Its lazy futures have
    // not executed here; replace the unjournaled local/API writes for this mutation.
    let core = <Ctx as Update<NativeEnv>>::update(ctx, message);
    let Some(title) = title else {
        return core;
    };
    let changed = core.has_changed;
    drop(core);
    let write = crate::checkpoint::write(&ctx.library, &title);
    let future = async move {
        let result: Result<(), EnvError> = match write {
            Ok(write) => write.await,
            Err(error) => Err(error),
        };
        let event = Event::LibraryItemsPushedToStorage { ids: vec![title] };
        match result {
            Ok(()) => Msg::Event(event),
            Err(error) => Msg::Event(Event::Error {
                error: CtxError::from(error),
                source: Box::new(event),
            }),
        }
    }
    .boxed();
    let mut effects = Effects::future(EffectFuture::Sequential(future))
        .join(Effects::msg(Msg::Internal(Internal::LibraryChanged(true))));
    effects.has_changed = changed;
    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{environment::Environment, sync_journal::Journal};
    use stremio_core::{
        runtime::{Effect, EffectFuture, Env, msg::Internal},
        types::profile::Profile,
    };

    #[tokio::test]
    async fn account_edit_has_one_atomic_local_write_and_no_untracked_upload() {
        let env = Environment::new().unwrap();
        env.scope(async {
            let mut profile = Profile::default();
            profile.auth = Some(serde_json::from_value(serde_json::json!({
                "key":"synthetic-token", "user":{"_id":"fixture-user", "email":"fixture@example.test",
                "lastModified":"2026-01-01T00:00:00Z", "dateRegistered":"2026-01-01T00:00:00Z",
                "premium_expire":null, "gdpr_consent":{"tos":true,"privacy":true,"marketing":false,"from":null}}
            })).unwrap());
            let mut ctx = crate::restore::context(profile).await.unwrap();
            let item = serde_json::from_value(serde_json::json!({"_id":"fixture", "type":"movie",
                "name":"Fixture", "removed":false, "temp":false, "_mtime":"2026-01-01T00:00:00Z",
                "state":{"timeWatched":0,"timeOffset":0,"overallTimeWatched":0,"timesWatched":0,"flaggedWatched":1,"duration":0}
            })).unwrap();
            let effects = update(&mut ctx, &Msg::Internal(Internal::UpdateLibraryItem(item)), None);
            let mut writes = 0;
            for effect in effects {
                match effect {
                    Effect::Future(EffectFuture::Concurrent(_)) => panic!("Untracked account upload escaped"),
                    Effect::Future(EffectFuture::Sequential(write)) => {
                        writes += 1;
                        assert!(!matches!(write.await, Msg::Event(stremio_core::runtime::msg::Event::Error { .. })));
                    }
                    _ => {}
                }
            }
            assert_eq!(writes, 1);
            let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY).await.unwrap().unwrap();
            assert_eq!(journal.revision("fixture"), Some(1));
            assert_eq!(ctx.library.items["fixture"].state.flagged_watched, 1);
            journal.validate_library(&ctx.library).unwrap();
            // Retrying unchanged intent after a storage error must still write, without
            // applying watched semantics a second time or reviving an immediate upload.
            for effect in update(&mut ctx, &Msg::Internal(Internal::LibraryChanged(false)), Some("fixture")) {
                match effect {
                    Effect::Future(EffectFuture::Concurrent(_)) => panic!("Retry uploaded outside journal"),
                    Effect::Future(EffectFuture::Sequential(write)) => {
                        assert!(!matches!(write.await, Msg::Event(stremio_core::runtime::msg::Event::Error { .. })));
                    }
                    _ => {}
                }
            }
            let journal = NativeEnv::get_storage::<Journal>(crate::sync_journal::STORAGE_KEY).await.unwrap().unwrap();
            assert_eq!(journal.revision("fixture"), Some(2));
            let stored = NativeEnv::get_storage::<stremio_core::types::library::LibraryBucket>(stremio_core::constants::LIBRARY_RECENT_STORAGE_KEY).await.unwrap().unwrap();
            assert_eq!(stored.items["fixture"].state.flagged_watched, 1);
            journal.validate_library(&stored).unwrap();
        }).await;
        env.shutdown().await;
    }
}
