//! C ABI: input strings borrowed for each call; snapshot strings must be freed.
//! Handles are monotonic IDs, not pointers. Stop is idempotent and nonblocking.
use super::{PlaybackSession, TorrentSource};
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

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("native runtime")
});
static NEXT: AtomicU64 = AtomicU64::new(1);
static SESSIONS: LazyLock<Mutex<HashMap<u64, Arc<Entry>>>> = LazyLock::new(Default::default);
struct Entry {
    status: Mutex<String>,
    cancel: CancellationToken,
}
#[derive(Deserialize)]
struct Config {
    torrent_path: Option<PathBuf>,
    magnet: Option<String>,
    #[serde(default)]
    discovery: bool,
    output: PathBuf,
    file_index: usize,
    initial_peers: Vec<SocketAddr>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_torrent_start(config: *const c_char) -> u64 {
    std::panic::catch_unwind(|| {
        if config.is_null() { return 0; }
        let Ok(config) = serde_json::from_slice::<Config>(unsafe { CStr::from_ptr(config) }.to_bytes()) else { return 0; };
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let entry = Arc::new(Entry { status: Mutex::new(r#"{"state":"starting"}"#.into()), cancel: CancellationToken::new() });
        SESSIONS.lock().unwrap().insert(id, entry.clone());
        RUNTIME.spawn(async move {
            let setup = async {
                let source = match (config.torrent_path, config.magnet) {
                    (Some(path), None) => TorrentSource::Bytes(std::fs::read(path)?),
                    (None, Some(magnet)) => TorrentSource::Magnet(magnet),
                    _ => anyhow::bail!("exactly one torrent source required"),
                };
                PlaybackSession::start_source(source, config.file_index, config.output, config.initial_peers, config.discovery).await
            };
            let session = tokio::select! {
                _ = entry.cancel.cancelled() => return,
                result = setup => match result {
                    Ok(s) => s,
                    Err(_) => { *entry.status.lock().unwrap() = r#"{"state":"failed","error":"Torrent session could not start"}"#.into(); return; }
                }
            };
            *entry.status.lock().unwrap() = serde_json::json!({"state":"ready", "url":session.url}).to_string();
            entry.cancel.cancelled().await;
            session.stop().await;
        });
        id
    }).unwrap_or(0)
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_torrent_snapshot(id: u64) -> *mut c_char {
    std::panic::catch_unwind(|| {
        let entry = SESSIONS.lock().unwrap().get(&id).cloned();
        let state = entry
            .map(|e| e.status.lock().unwrap().clone())
            .unwrap_or_else(|| r#"{"state":"stopped"}"#.into());
        CString::new(state).unwrap().into_raw()
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
    fn invalid_input_and_repeated_stop_are_safe() {
        assert_eq!(unsafe { cue_torrent_start(std::ptr::null()) }, 0);
        let invalid = CString::new("{}").unwrap();
        assert_eq!(unsafe { cue_torrent_start(invalid.as_ptr()) }, 0);
        cue_torrent_stop(u64::MAX);
        cue_torrent_stop(u64::MAX);
        let status = cue_torrent_snapshot(u64::MAX);
        assert!(!status.is_null());
        assert_eq!(
            unsafe { CStr::from_ptr(status) }.to_str().unwrap(),
            r#"{"state":"stopped"}"#
        );
        unsafe {
            cue_torrent_string_free(status);
        }
    }
    #[test]
    fn stopping_pending_start_invalidates_handle() {
        let config = CString::new(r#"{"torrent_path":"/nonexistent/cue-test.torrent","output":"/nonexistent/cue-test-output","file_index":0,"initial_peers":[]}"#).unwrap();
        let id = unsafe { cue_torrent_start(config.as_ptr()) };
        assert_ne!(id, 0);
        cue_torrent_stop(id);
        cue_torrent_stop(id);
        let status = cue_torrent_snapshot(id);
        assert_eq!(
            unsafe { CStr::from_ptr(status) }.to_str().unwrap(),
            r#"{"state":"stopped"}"#
        );
        unsafe {
            cue_torrent_string_free(status);
        }
    }
}
