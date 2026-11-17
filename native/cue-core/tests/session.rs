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

#[tokio::test]
async fn addon_action_fetches_manifest_through_core_and_redacts_transport() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let responses = [
            (
                "GET /private-config/manifest.json",
                r#"{"id":"org.cue.fixture","version":"1.0.0","name":"Fixture Addon","description":"Owned test fixture","resources":["catalog"],"types":["movie"],"catalogs":[{"id":"fixture","type":"movie","name":"My catalogue"}]}"#,
            ),
            (
                "GET /private-config/catalog/movie/fixture.json",
                r#"{"metas":[{"id":"tt123","type":"movie","name":"Fixture film"}]}"#,
            ),
        ];
        for (expected, body) in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let count = socket.read(&mut request).await.unwrap();
            assert!(
                std::str::from_utf8(&request[..count])
                    .unwrap()
                    .starts_with(expected)
            );
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
        }
    });
    let directory = std::env::temp_dir().join(format!(
        "cue-core-install-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let mut session = CoreSession::guest(Environment::persistent(&directory, &[5; 32]).unwrap())
        .await
        .unwrap();
    assert!(
        session
            .load_addon("file:///private/manifest.json")
            .await
            .is_err()
    );
    session
        .load_addon(&format!("http://{address}/private-config/manifest.json"))
        .await
        .unwrap();
    let state = tokio::time::timeout(Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    let addon = state.addon.as_ref().unwrap();
    assert_eq!(addon.status, "ready");
    assert_eq!(addon.name.as_deref(), Some("Fixture Addon"));
    assert_eq!(addon.id.as_deref(), Some("org.cue.fixture"));
    assert!(
        !serde_json::to_string(&state)
            .unwrap()
            .contains("private-config")
    );
    assert!(session.install_addon(0).await.is_err());
    session.install_addon(state.revision).await.unwrap();
    let installed = session.settled().await.unwrap();
    assert_eq!(installed.addon_count, state.addon_count + 1);
    assert!(installed.addon.unwrap().installed);
    assert!(installed.error.is_none());
    let choice = installed
        .catalogs
        .iter()
        .find(|c| c.name == "My catalogue")
        .unwrap();
    assert!(session.load_catalog(choice.index, 0).await.is_err());
    session
        .load_catalog(choice.index, installed.revision)
        .await
        .unwrap();
    let loaded = tokio::time::timeout(Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    let catalog = loaded.catalog.unwrap();
    assert_eq!(catalog.status, "ready");
    assert_eq!(catalog.items[0].id, "tt123");
    assert_eq!(catalog.items[0].name, "Fixture film");
    server.await.unwrap();
    session.stop().await;
    drop(session);
    let encrypted = std::fs::read(directory.join("state.bin")).unwrap();
    assert!(!String::from_utf8_lossy(&encrypted).contains("private-config"));
    let mut restored = CoreSession::guest(Environment::persistent(&directory, &[5; 32]).unwrap())
        .await
        .unwrap();
    assert_eq!(restored.snapshot().addon_count, state.addon_count + 1);
    restored.stop().await;
    drop(restored);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn stale_manifest_failure_cannot_replace_newer_selection() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let old = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let old_url = format!(
        "http://{}/old-secret/manifest.json",
        old.local_addr().unwrap()
    );
    let new = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let new_url = format!("http://{}/manifest.json", new.local_addr().unwrap());
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let first = tokio::spawn(async move {
        let (mut socket, _) = old.accept().await.unwrap();
        let mut request = [0; 4096];
        socket.read(&mut request).await.unwrap();
        started.send(()).unwrap();
        released.await.unwrap();
        socket.write_all(b"HTTP/1.1 500 Error\r\nContent-Length: 14\r\nConnection: close\r\n\r\nprivate-secret").await.unwrap();
    });
    let second = tokio::spawn(async move {
        let (mut socket, _) = new.accept().await.unwrap();
        let mut request = [0; 4096];
        socket.read(&mut request).await.unwrap();
        let body = r#"{"id":"new","version":"1.0.0","name":"New selection","description":"Fixture","resources":[],"types":[],"catalogs":[]}"#;
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .as_bytes(),
            )
            .await
            .unwrap();
    });
    let mut session = CoreSession::guest(Environment::new().unwrap())
        .await
        .unwrap();
    session.load_addon(&old_url).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), ready)
        .await
        .unwrap()
        .unwrap();
    session.load_addon(&new_url).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if session
                .snapshot()
                .addon
                .as_ref()
                .and_then(|a| a.id.as_deref())
                == Some("new")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    release.send(()).unwrap();
    let state = tokio::time::timeout(Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.addon.as_ref().unwrap().id.as_deref(), Some("new"));
    assert!(!serde_json::to_string(&state).unwrap().contains("secret"));
    first.await.unwrap();
    second.await.unwrap();
    session.stop().await;
}

#[tokio::test]
async fn failed_addon_can_be_retried_at_the_same_url() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/manifest.json", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for success in [false, true] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            socket.read(&mut request).await.unwrap();
            let body = if success {
                r#"{"id":"retry","name":"Recovered","version":"1.0.0","description":"Fixture","resources":[],"types":[],"catalogs":[]}"#
            } else {
                "invalid JSON"
            };
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
    });
    let mut session = CoreSession::guest(Environment::new().unwrap())
        .await
        .unwrap();
    session.load_addon(&url).await.unwrap();
    let state = tokio::time::timeout(Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.addon.unwrap().status, "failed");
    session.load_addon(&url).await.unwrap();
    let state = tokio::time::timeout(Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.addon.unwrap().name.as_deref(), Some("Recovered"));
    server.await.unwrap();
    session.stop().await;
}

#[tokio::test]
async fn returning_to_a_url_does_not_admit_its_previous_request() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (ready, mut started) = tokio::sync::mpsc::channel(3);
    let mut release = Vec::new();
    let mut gates = Vec::new();
    for _ in 0..3 {
        let (send, receive) = tokio::sync::oneshot::channel();
        release.push(send);
        gates.push(receive);
    }
    let server = tokio::spawn(async move {
        let mut workers = Vec::new();
        for (index, gate) in gates.into_iter().enumerate() {
            let (mut socket, _) = listener.accept().await.unwrap();
            let ready = ready.clone();
            workers.push(tokio::spawn(async move {
                let mut request = [0;4096];
                socket.read(&mut request).await.unwrap();
                ready.send(index).await.unwrap();
                gate.await.unwrap();
                let body = if index == 2 { r#"{"id":"new-a","name":"Current A","version":"1.0.0","description":"Fixture","resources":[],"types":[],"catalogs":[]}"# } else { "old invalid response" };
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
            }));
        }
        for worker in workers {
            worker.await.unwrap();
        }
    });
    let mut session = CoreSession::guest(Environment::new().unwrap())
        .await
        .unwrap();
    for (index, path) in ["a", "b", "a"].into_iter().enumerate() {
        session
            .load_addon(&format!("http://{address}/{path}/manifest.json"))
            .await
            .unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), started.recv())
                .await
                .unwrap(),
            Some(index)
        );
    }
    release.remove(0).send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while session.snapshot().pending_effects != 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        session.snapshot().addon.unwrap().status,
        "loading",
        "old A must not fail current A"
    );
    release.pop().unwrap().send(()).unwrap();
    release.pop().unwrap().send(()).unwrap();
    let state = tokio::time::timeout(Duration::from_secs(2), session.settled())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.addon.unwrap().id.as_deref(), Some("new-a"));
    server.await.unwrap();
    session.stop().await;
}
