//! C ABI: input strings borrowed for each call; snapshot strings must be freed.
//! Handles are monotonic IDs, not pointers. Stop is idempotent and nonblocking.
use super::failure::FailureCode;
use super::{Discovery, PlaybackSession, TorrentSource};
use serde::Deserialize;
use std::{
    collections::HashMap,
    ffi::{CStr, CString, c_char},
    net::SocketAddr,
    path::PathBuf,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio_util::sync::CancellationToken;

pub(crate) static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("native runtime")
});
static NEXT: AtomicU64 = AtomicU64::new(1);
static SESSIONS: LazyLock<Mutex<HashMap<u64, Arc<Entry>>>> = LazyLock::new(Default::default);
static STOPS: LazyLock<Mutex<HashMap<u64, CancellationToken>>> = LazyLock::new(Default::default);
struct Entry {
    done: CancellationToken,
    status: Mutex<String>,
    cancel: CancellationToken,
}
#[derive(Deserialize)]
struct Config {
    api_version: u32,
    torrent_path: Option<PathBuf>,
    magnet: Option<String>,
    #[serde(default)]
    discovery: Discovery,
    output: PathBuf,
    cache_root: Option<PathBuf>,
    file_index: usize,
    initial_peers: Vec<SocketAddr>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_torrent_start(config: *const c_char) -> u64 {
    std::panic::catch_unwind(|| {
        if config.is_null() {
            return 0;
        }
        let Ok(config) =
            serde_json::from_slice::<Config>(unsafe { CStr::from_ptr(config) }.to_bytes())
        else {
            return 0;
        };
        if config.api_version != 1 {
            return 0;
        }
        let setup = async move {
            let source = match (config.torrent_path, config.magnet) {
                (Some(path), None) => TorrentSource::Bytes(std::fs::read(path)?),
                (None, Some(magnet)) => TorrentSource::Magnet(magnet),
                _ => anyhow::bail!(FailureCode::InvalidMetadata),
            };
            PlaybackSession::start_budgeted_in_root(
                source,
                config.file_index,
                config.output,
                config.initial_peers,
                config.discovery,
                super::storage_budget::StorageBudget::default(),
                None,
                config.cache_root,
            )
            .await
        };
        launch(setup)
    })
    .unwrap_or(0)
}

pub(crate) fn launch(
    setup: impl std::future::Future<Output = anyhow::Result<PlaybackSession>> + Send + 'static,
) -> u64 {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let entry = Arc::new(Entry {
        done: CancellationToken::new(),
        status: Mutex::new(r#"{"state":"starting"}"#.into()),
        cancel: CancellationToken::new(),
    });
    SESSIONS.lock().unwrap().insert(id, entry.clone());
    RUNTIME.spawn(async move {
        let _done = entry.done.clone().drop_guard();
        let setup = setup;
            let session = tokio::select! {
                _ = entry.cancel.cancelled() => return,
                result = setup => match result {
                    Ok(s) => s,
                    Err(error) => { *entry.status.lock().unwrap() = FailureCode::from_error(&error).snapshot().to_string(); return; }
                }
            };
            loop {
                let snapshot = session.snapshot();
                let failed = snapshot["state"] == "failed";
                *entry.status.lock().unwrap() = snapshot.to_string();
                if failed { break; }
                tokio::select! {
                    _ = entry.cancel.cancelled() => break,
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
                }
            }
            session.stop().await;
        });
    id
}

#[unsafe(no_mangle)]
pub extern "C" fn cue_torrent_snapshot(id: u64) -> *mut c_char {
    std::panic::catch_unwind(|| {
        let entry = SESSIONS.lock().unwrap().get(&id).cloned();
        let state = entry
            .map(|e| e.status.lock().unwrap().clone())
            .unwrap_or_else(|| r#"{"state":"stopped"}"#.into());
        let mut payload: serde_json::Value = serde_json::from_str(&state).unwrap();
        payload["api_version"] = 1.into();
        payload["session_id"] = id.into();
        CString::new(payload.to_string()).unwrap().into_raw()
    })
    .unwrap_or(std::ptr::null_mut())
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_torrent_stop(id: u64) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(entry) = SESSIONS.lock().unwrap().remove(&id) {
            entry.cancel.cancel();
        }
    });
}
/// Caller must release the ticket even if it abandons waiting.
#[unsafe(no_mangle)]
pub extern "C" fn cue_torrent_stop_ticket(id: u64) -> u64 {
    std::panic::catch_unwind(|| {
        let Some(entry) = SESSIONS.lock().unwrap().remove(&id) else {
            return 0;
        };
        STOPS.lock().unwrap().insert(id, entry.done.clone());
        entry.cancel.cancel();
        id
    })
    .unwrap_or(0)
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_torrent_stop_status(ticket: u64) -> i32 {
    std::panic::catch_unwind(|| {
        STOPS
            .lock()
            .unwrap()
            .get(&ticket)
            .map(|done| if done.is_cancelled() { 1 } else { 0 })
            .unwrap_or(-1)
    })
    .unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_torrent_stop_release(ticket: u64) {
    let _ = std::panic::catch_unwind(|| {
        STOPS.lock().unwrap().remove(&ticket);
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_torrent_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shutdown_ticket_waits_for_ready_session_and_can_be_released() {
        let output = tempfile::tempdir().unwrap();
        let torrent = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/torrents/single.torrent");
        let config = CString::new(
            serde_json::json!({"api_version":1,"torrent_path":torrent,
            "output":output.path(),"file_index":0,"initial_peers":[]})
            .to_string(),
        )
        .unwrap();
        let id = unsafe { cue_torrent_start(config.as_ptr()) };
        assert_ne!(id, 0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let ptr = cue_torrent_snapshot(id);
            let value: serde_json::Value =
                serde_json::from_slice(unsafe { CStr::from_ptr(ptr) }.to_bytes()).unwrap();
            unsafe {
                cue_torrent_string_free(ptr);
            }
            if value["state"] == "ready" {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "session did not become ready: {value}"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let ticket = cue_torrent_stop_ticket(id);
        assert_ne!(ticket, 0);
        assert_eq!(cue_torrent_stop_ticket(id), 0);
        while cue_torrent_stop_status(ticket) == 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "shutdown acknowledgement timed out"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(cue_torrent_stop_status(ticket), 1);
        std::fs::remove_dir_all(output.path()).unwrap();
        cue_torrent_stop_release(ticket);
        cue_torrent_stop_release(ticket);
        assert_eq!(cue_torrent_stop_status(ticket), -1);
    }

    #[test]
    fn shutdown_ticket_handles_cancelled_setup() {
        let config = CString::new(r#"{"api_version":1,"torrent_path":"/nonexistent/cue-test.torrent","output":"/nonexistent/cue-test-output","file_index":0,"initial_peers":[]}"#).unwrap();
        let id = unsafe { cue_torrent_start(config.as_ptr()) };
        let ticket = cue_torrent_stop_ticket(id);
        assert_ne!(ticket, 0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while cue_torrent_stop_status(ticket) == 0 {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(cue_torrent_stop_status(ticket), 1);
        cue_torrent_stop_release(ticket);
    }

    #[test]
    fn invalid_input_and_repeated_stop_are_safe() {
        assert_eq!(unsafe { cue_torrent_start(std::ptr::null()) }, 0);
        let invalid = CString::new("{}").unwrap();
        assert_eq!(unsafe { cue_torrent_start(invalid.as_ptr()) }, 0);
        let future = CString::new(r#"{"api_version":2,"torrent_path":"/nonexistent","output":"/nonexistent","file_index":0,"initial_peers":[]}"#).unwrap();
        assert_eq!(unsafe { cue_torrent_start(future.as_ptr()) }, 0);
        cue_torrent_stop(u64::MAX);
        cue_torrent_stop(u64::MAX);
        let status = cue_torrent_snapshot(u64::MAX);
        assert!(!status.is_null());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                unsafe { CStr::from_ptr(status) }.to_bytes()
            )
            .unwrap()["state"],
            "stopped"
        );
        unsafe {
            cue_torrent_string_free(status);
        }
    }
    #[test]
    fn stopping_pending_start_invalidates_handle() {
        let config = CString::new(r#"{"api_version":1,"torrent_path":"/nonexistent/cue-test.torrent","output":"/nonexistent/cue-test-output","file_index":0,"initial_peers":[]}"#).unwrap();
        let id = unsafe { cue_torrent_start(config.as_ptr()) };
        assert_ne!(id, 0);
        cue_torrent_stop(id);
        cue_torrent_stop(id);
        let status = cue_torrent_snapshot(id);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                unsafe { CStr::from_ptr(status) }.to_bytes()
            )
            .unwrap()["state"],
            "stopped"
        );
        unsafe {
            cue_torrent_string_free(status);
        }
    }
    #[test]
    fn invalid_metadata_has_stable_redacted_failure_code() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("private-fixture-name.torrent");
        std::fs::write(&path, b"not a torrent: private-marker").unwrap();
        let config = CString::new(
            serde_json::json!({"api_version":1,"torrent_path":path,
            "output":directory.path(),"file_index":0,"initial_peers":[]})
            .to_string(),
        )
        .unwrap();
        let id = unsafe { cue_torrent_start(config.as_ptr()) };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let payload = loop {
            let ptr = cue_torrent_snapshot(id);
            let raw = unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned();
            unsafe {
                cue_torrent_string_free(ptr);
            }
            let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
            if value["state"] == "failed" {
                break (raw, value);
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        cue_torrent_stop(id);
        assert_eq!(payload.1["error_code"], "invalid_metadata");
        assert!(!payload.0.contains("private-marker"));
        assert!(!payload.0.contains("private-fixture-name"));
    }
}
