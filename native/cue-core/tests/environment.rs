use cue_core::environment::{Environment, NativeEnv};
use serde_json::Value;
use std::time::Duration;
use stremio_core::runtime::Env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn core_schema_migration_uses_isolated_environment_storage() {
    let first = Environment::new().unwrap();
    let second = Environment::new().unwrap();
    first
        .scope(async {
            NativeEnv::migrate_storage_schema().await.unwrap();
            let version: Option<u32> =
                NativeEnv::get_storage(stremio_core::constants::SCHEMA_VERSION_STORAGE_KEY)
                    .await
                    .unwrap();
            assert_eq!(version, Some(stremio_core::constants::SCHEMA_VERSION));
            NativeEnv::set_storage("private", Some(&"account-one"))
                .await
                .unwrap();
        })
        .await;
    second
        .scope(async {
            assert_eq!(
                NativeEnv::get_storage::<String>("private").await.unwrap(),
                None
            );
        })
        .await;
    first.cancel();
    first
        .scope(async {
            assert!(
                NativeEnv::set_storage("private", Some(&"late-write"))
                    .await
                    .is_err()
            );
            assert!(NativeEnv::get_storage::<String>("private").await.is_err());
        })
        .await;
    second
        .scope(async {
            NativeEnv::set_storage("private", Some(&"account-two"))
                .await
                .unwrap();
        })
        .await;
    second.cancel();
}

#[tokio::test]
async fn json_fetch_uses_real_http_and_rejects_oversized_response() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for body in ["{\"value\":42}", "oversized"] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            socket.read(&mut request).await.unwrap();
            let response = if body == "oversized" {
                "HTTP/1.1 200 OK\r\nContent-Length: 5000000\r\nConnection: close\r\n\r\n".to_owned()
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
            };
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let env = Environment::new().unwrap();
    env.scope(async {
        let request = || {
            http::Request::get(format!("http://{address}/fixture"))
                .body(())
                .unwrap()
        };
        let value: Value = NativeEnv::fetch(request()).await.unwrap();
        assert_eq!(value["value"], 42);
        let result = NativeEnv::fetch::<_, Value>(request()).await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Response too large")
        );
    })
    .await;
    env.cancel();
    server.await.unwrap();
}

#[tokio::test]
async fn cancellation_interrupts_a_pending_http_response() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (accepted, ready) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (_socket, _) = listener.accept().await.unwrap();
        accepted.send(()).unwrap();
        futures::future::pending::<()>().await;
    });
    let env = Environment::new().unwrap();
    let task_env = env.clone();
    let fetch = tokio::spawn(async move {
        task_env
            .scope(async {
                NativeEnv::fetch::<_, Value>(
                    http::Request::get(format!("http://{address}/private-token"))
                        .body(())
                        .unwrap(),
                )
                .await
            })
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), ready)
        .await
        .unwrap()
        .unwrap();
    env.cancel();
    let error = tokio::time::timeout(Duration::from_secs(1), fetch)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(!error.to_string().contains("private-token"));
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sequential_effects_keep_submission_order_and_context() {
    let env = Environment::new().unwrap();
    let (release, barrier) = tokio::sync::oneshot::channel();
    let (done, mut observed) = tokio::sync::mpsc::channel(16);
    env.scope(async {
        NativeEnv::exec_sequential(async move {
            barrier.await.unwrap();
        });
        for index in 0..10u32 {
            let done = done.clone();
            NativeEnv::exec_sequential(async move {
                NativeEnv::set_storage("sequence", Some(&index))
                    .await
                    .unwrap();
                done.send(index).await.unwrap();
            });
        }
    })
    .await;
    assert!(observed.try_recv().is_err());
    release.send(()).unwrap();
    for expected in 0..10 {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), observed.recv())
                .await
                .unwrap(),
            Some(expected)
        );
    }
    env.scope(async {
        assert_eq!(
            NativeEnv::get_storage::<u32>("sequence").await.unwrap(),
            Some(9)
        );
    })
    .await;
    env.cancel();
}

#[tokio::test]
async fn chunked_http_body_is_bounded_without_content_length() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        socket.read(&mut request).await.unwrap();
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        let block = vec![b' '; 16384];
        for _ in 0..257 {
            if socket.write_all(b"4000\r\n").await.is_err() {
                break;
            }
            if socket.write_all(&block).await.is_err() {
                break;
            }
            if socket.write_all(b"\r\n").await.is_err() {
                break;
            }
        }
        let _ = socket.write_all(b"0\r\n\r\n").await;
    });
    let env = Environment::new().unwrap();
    env.scope(async {
        let request = http::Request::get(format!("http://{address}/fixture"))
            .body(())
            .unwrap();
        let error = NativeEnv::fetch::<_, Value>(request).await.unwrap_err();
        assert!(error.to_string().contains("Response too large"));
    })
    .await;
    env.cancel();
    server.await.unwrap();
}

#[test]
fn environment_without_runtime_returns_error_instead_of_panicking() {
    assert!(Environment::new().is_err());
}

#[tokio::test]
async fn injected_clock_and_storage_budget_are_session_scoped() {
    let instant = chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
        .unwrap()
        .to_utc();
    let env = Environment::with_clock(std::sync::Arc::new(move || instant)).unwrap();
    env.scope(async {
        assert_eq!(NativeEnv::now(), instant);
        NativeEnv::set_storage("saved", Some(&42)).await.unwrap();
        let oversized = "x".repeat(16 * 1024 * 1024);
        assert!(
            NativeEnv::set_storage("saved", Some(&oversized))
                .await
                .is_err()
        );
        assert_eq!(
            NativeEnv::get_storage::<u32>("saved").await.unwrap(),
            Some(42)
        );
    })
    .await;
    env.cancel();
}

#[tokio::test]
async fn shutdown_joins_concurrent_and_queued_effects() {
    struct Dropped(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
    let dropped = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let executed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let env = Environment::new().unwrap();
    let (ready, mut started) = tokio::sync::mpsc::channel(2);
    env.scope(async {
        let marker = Dropped(dropped.clone());
        let concurrent_ready = ready.clone();
        NativeEnv::exec_concurrent(async move {
            let _marker = marker;
            concurrent_ready.send(()).await.unwrap();
            futures::future::pending::<()>().await;
        });
        let marker = Dropped(dropped.clone());
        NativeEnv::exec_sequential(async move {
            let _marker = marker;
            ready.send(()).await.unwrap();
            futures::future::pending::<()>().await;
        });
        let marker = Dropped(dropped.clone());
        let executed = executed.clone();
        NativeEnv::exec_sequential(async move {
            let _marker = marker;
            executed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
    })
    .await;
    for _ in 0..2 {
        tokio::time::timeout(Duration::from_secs(2), started.recv())
            .await
            .unwrap()
            .unwrap();
    }
    tokio::time::timeout(Duration::from_secs(1), env.shutdown())
        .await
        .unwrap();
    assert_eq!(dropped.load(std::sync::atomic::Ordering::SeqCst), 3);
    env.scope(async {
        let executed = executed.clone();
        NativeEnv::exec_concurrent(async move {
            executed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
    })
    .await;
    env.shutdown().await;
    assert_eq!(executed.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[tokio::test]
async fn api_post_declares_json_and_preserves_credentials_in_body_only() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let (headers, body) = loop {
            let mut chunk = [0; 1024];
            let count = socket.read(&mut chunk).await.unwrap();
            assert!(count > 0 && bytes.len() + count <= 8192);
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                let headers = String::from_utf8(bytes[..end].to_vec()).unwrap();
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                if bytes.len() >= end + 4 + length {
                    break (headers, bytes[end + 4..end + 4 + length].to_vec());
                }
            }
        };
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
            .await
            .unwrap();
        (headers, body)
    });
    let env = Environment::new().unwrap();
    env.scope(async {
        let request = http::Request::post(format!("http://{address}/api/login"))
            .body(serde_json::json!({"email":"fixture@example.test", "password":"synthetic-password"})).unwrap();
        NativeEnv::fetch::<_, Value>(request).await.unwrap();
    }).await;
    let (headers, body) = tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    assert!(
        headers
            .to_ascii_lowercase()
            .contains("\r\ncontent-type: application/json")
    );
    assert!(!headers.contains("synthetic-password"));
    assert!(!headers.contains("fixture@example.test"));
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap(),
        serde_json::json!({"email":"fixture@example.test", "password":"synthetic-password"})
    );
    env.shutdown().await;
}

#[tokio::test]
async fn addon_activity_reporting_never_reaches_http_but_content_requests_do() {
    use stremio_core::types::addon::ResourcePath;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = reqwest::Url::parse(&format!(
        "http://{}/manifest.json",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let paths = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let observed = paths.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = [0; 4096];
            let count = socket.read(&mut bytes).await.unwrap();
            let request = std::str::from_utf8(&bytes[..count]).unwrap();
            let path = request.split_whitespace().nth(1).unwrap().to_owned();
            observed.lock().unwrap().push(path.clone());
            let body = if path == "/manifest.json" {
                r#"{"id":"owned","name":"Owned","version":"1.0.0","description":"Owned","resources":["catalog"],"types":["movie"],"catalogs":[]}"#
            } else {
                r#"{"metas":[]}"#
            };
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    let env = Environment::new().unwrap();
    let outcome = tokio::time::timeout(
        Duration::from_secs(3),
        env.scope(async {
            for resource in ["player", "library"] {
                let result = NativeEnv::addon_transport(&url)
                    .resource(&ResourcePath::without_extra(
                        resource,
                        "movie",
                        "private-title",
                    ))
                    .await;
                assert!(
                    result.is_err(),
                    "Activity reporting must be disabled locally"
                );
            }
            assert_eq!(
                NativeEnv::addon_transport(&url)
                    .manifest()
                    .await
                    .unwrap()
                    .id,
                "owned"
            );
            NativeEnv::addon_transport(&url)
                .resource(&ResourcePath::without_extra("catalog", "movie", "owned"))
                .await
                .unwrap();
        }),
    )
    .await;
    server.abort();
    env.cancel();
    outcome.unwrap();
    assert_eq!(
        *paths.lock().unwrap(),
        ["/manifest.json", "/catalog/movie/owned.json"]
    );
}

#[tokio::test]
async fn http_auth_rejection_is_distinct_from_retryable_status_and_redacted() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for status in [401, 403, 429, 503] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            socket.read(&mut request).await.unwrap();
            let body = "synthetic-token private-title";
            let response = format!(
                "HTTP/1.1 {status} Rejected\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let env = Environment::new().unwrap();
    env.scope(async {
        for expected in ["HTTP authentication rejected", "HTTP authentication rejected", "HTTP status rejected", "HTTP status rejected"] {
            let request = http::Request::get(format!("http://{address}/fixture")).body(()).unwrap();
            let error = NativeEnv::fetch::<_, Value>(request).await.unwrap_err();
            assert!(matches!(error, stremio_core::runtime::EnvError::Fetch(ref text) if text == expected));
            assert!(!error.to_string().contains("synthetic-token"));
            assert!(!error.to_string().contains("private-title"));
        }
    }).await;
    env.shutdown().await;
    server.await.unwrap();
}
