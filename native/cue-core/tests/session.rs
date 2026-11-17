use cue_core::{
    environment::{Environment, NativeEnv},
    session::CoreSession,
};
use std::time::Duration;
use stremio_core::{constants::PROFILE_STORAGE_KEY, runtime::Env, types::profile::Profile};

#[tokio::test]
async fn real_core_action_updates_state_and_executes_storage_effect() {
    let env = Environment::new().unwrap();
    let mut session = CoreSession::guest(env.clone()).await.unwrap();
    assert_eq!(session.snapshot().interface_language, "eng");
    assert!(session.snapshot().addon_count > 0);
    session.set_interface_language("fra").await.unwrap();
    let state = tokio::time::timeout(Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.interface_language, "fra");
    assert!(state.revision > 0);
    env.scope(async {
        let profile = NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(profile.settings.interface_language, "fra");
        assert!(profile.auth.is_none());
    })
    .await;
    session.stop().await;
    assert!(session.snapshot().stopped);
    assert!(session.set_interface_language("eng").await.is_err());
    session.stop().await;
}

#[tokio::test]
async fn sessions_do_not_share_model_actions_or_storage() {
    let mut first = CoreSession::guest(Environment::new().unwrap())
        .await
        .unwrap();
    let mut second = CoreSession::guest(Environment::new().unwrap())
        .await
        .unwrap();
    first.set_interface_language("fra").await.unwrap();
    first.settled().await.unwrap();
    assert_eq!(second.snapshot().interface_language, "eng");
    assert!(second.set_interface_language("../private").await.is_err());
    first.stop().await;
    second.set_interface_language("deu").await.unwrap();
    assert_eq!(second.settled().await.unwrap().interface_language, "deu");
    second.stop().await;
}

#[tokio::test]
async fn a_new_action_can_recover_after_an_effect_failure() {
    let env = Environment::new().unwrap();
    let mut session = CoreSession::guest(env.clone()).await.unwrap();
    env.scope(async {
        NativeEnv::set_storage("fill", Some(&"x".repeat(16 * 1024 * 1024 - 256)))
            .await
            .unwrap();
    })
    .await;
    session.set_interface_language("fra").await.unwrap();
    assert_eq!(
        session.settled().await.unwrap().error,
        Some("core_effect_failed")
    );
    env.scope(async {
        NativeEnv::set_storage::<()>("fill", None).await.unwrap();
    })
    .await;
    session.set_interface_language("deu").await.unwrap();
    let state = session.settled().await.unwrap();
    assert_eq!(state.error, None);
    env.scope(async {
        assert_eq!(
            NativeEnv::get_storage::<Profile>(PROFILE_STORAGE_KEY)
                .await
                .unwrap()
                .unwrap()
                .settings
                .interface_language,
            "deu"
        );
    })
    .await;
    session.stop().await;
}

#[tokio::test]
async fn failed_model_start_closes_its_environment() {
    let env = Environment::new().unwrap();
    env.scope(async {
        NativeEnv::set_storage(
            stremio_core::constants::SCHEMA_VERSION_STORAGE_KEY,
            Some(&stremio_core::constants::SCHEMA_VERSION),
        )
        .await
        .unwrap();
        NativeEnv::set_storage(PROFILE_STORAGE_KEY, Some(&42))
            .await
            .unwrap();
    })
    .await;
    assert!(CoreSession::guest(env.clone()).await.is_err());
    env.scope(async {
        assert!(NativeEnv::set_storage("late", Some(&1)).await.is_err());
    })
    .await;
    tokio::time::timeout(Duration::from_secs(1), env.shutdown())
        .await
        .unwrap();
}

#[tokio::test]
async fn failed_effect_stops_its_session_without_stranding_waiters() {
    for sequential in [false, true] {
        let env = Environment::new().unwrap();
        let mut session = CoreSession::guest(env.clone()).await.unwrap();
        env.scope(async {
            let failure = async { panic!("controlled effect failure") };
            if sequential {
                NativeEnv::exec_sequential(failure);
            } else {
                NativeEnv::exec_concurrent(failure);
            }
        })
        .await;
        tokio::time::timeout(Duration::from_secs(1), async {
            while !session.snapshot().stopped {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("failed effect must stop its model owner");
        assert_eq!(session.snapshot().error, Some("core_task_failed"));
        assert!(session.set_interface_language("fra").await.is_err());
        tokio::time::timeout(Duration::from_secs(1), session.stop())
            .await
            .unwrap();
    }
}
