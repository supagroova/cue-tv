#![cfg(feature = "account-fixture")]

use cue_core::{environment::Environment, session::CoreSession};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn account_fixture_uses_real_http_and_redacts_rejected_login() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut chunk = [0; 1024];
            let size = socket.read(&mut chunk).await.unwrap();
            assert!(size > 0);
            bytes.extend_from_slice(&chunk[..size]);
            assert!(bytes.len() < 16384);
            if let Some(offset) = bytes.windows(4).position(|value| value == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&bytes[..offset]).unwrap().to_lowercase();
                let length: usize = headers.lines().find_map(|line| line.strip_prefix("content-length: ")).unwrap().parse().unwrap();
                if bytes.len() >= offset + 4 + length { break; }
            }
        }
        let body = br#"{"error":{"code":1,"message":"fixture-password fixture@example.test"}}"#;
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
        socket.write_all(body).await.unwrap();
        String::from_utf8(bytes).unwrap()
    });
    let env = Environment::account_fixture(port).unwrap();
    let mut session = CoreSession::new_account(env).await.unwrap();
    session.login("fixture@example.test", "fixture-password").await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while session.snapshot().error.is_none() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    let request = server.await.unwrap();
    assert!(request.starts_with("POST /api/login HTTP/1.1\r\n"));
    assert!(request.contains("fixture-password"));
    let snapshot = serde_json::to_string(&session.snapshot()).unwrap();
    assert!(!session.snapshot().authenticated);
    assert!(!snapshot.contains("fixture-password"));
    assert!(!snapshot.contains("fixture@example.test"));
    session.stop().await;
}
