//! Metadata handles retain torrent bytes internally; caller buffers contain file windows only.
use crate::{
    Discovery, TorrentSource,
    failure::FailureCode,
    metadata::{self, Resolved},
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tokio_util::sync::CancellationToken;
static NEXT: AtomicU64 = AtomicU64::new(1);
static ENTRIES: LazyLock<Mutex<HashMap<u64, Arc<Entry>>>> = LazyLock::new(Default::default);
struct Entry {
    discovery: Discovery,
    cancel: CancellationToken,
    done: AtomicBool,
    result: Mutex<Option<Result<Resolved, FailureCode>>>,
}
struct Completion(Arc<Entry>);
impl Drop for Completion {
    fn drop(&mut self) {
        let mut result = self.0.result.lock().unwrap_or_else(|e| e.into_inner());
        if result.is_none() && !self.0.cancel.is_cancelled() {
            *result = Some(Err(FailureCode::Read));
        }
        self.0.done.store(true, Ordering::Release);
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    api_version: u32,
    magnet: String,
    output: PathBuf,
    #[serde(default)]
    discovery: Discovery,
}
fn start(source: TorrentSource, output: PathBuf, discovery: Discovery) -> u64 {
    let mut entries = ENTRIES.lock().unwrap();
    if entries.len() >= 8 {
        return 0;
    }
    let Ok(id) = NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
    else {
        return 0;
    };
    let entry = Arc::new(Entry {
        discovery: discovery.clone(),
        cancel: CancellationToken::new(),
        done: AtomicBool::new(false),
        result: Mutex::new(None),
    });
    entries.insert(id, entry.clone());
    crate::ffi::RUNTIME.spawn(async move {
        let _completion = Completion(entry.clone());
        let result =
            metadata::resolve(source, output, vec![], discovery, entry.cancel.clone()).await;
        {
            let mut state = entry.result.lock().unwrap();
            if !entry.cancel.is_cancelled() {
                *state = Some(result.map_err(|e| FailureCode::from_error(&e)));
            }
        }
    });
    id
}
/// Input borrowed only for this call, at most 24 KiB. Returns zero on rejection.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_metadata_start(input: *const u8, len: usize) -> u64 {
    std::panic::catch_unwind(|| {
        if input.is_null() || len == 0 || len > 24576 {
            return 0;
        }
        let Ok(config) =
            serde_json::from_slice::<Config>(unsafe { std::slice::from_raw_parts(input, len) })
        else {
            return 0;
        };
        if config.api_version != 1
            || config.magnet.len() > 16384
            || config.output.as_os_str().len() > 4096
        {
            return 0;
        }
        start(
            TorrentSource::Magnet(config.magnet),
            config.output,
            config.discovery,
        )
    })
    .unwrap_or(0)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaybackConfig {
    api_version: u32,
    output: PathBuf,
    cache_root: Option<PathBuf>,
}

/// On success consumes the metadata handle and returns a normal playback handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_metadata_play(
    id: u64,
    index: usize,
    input: *const u8,
    len: usize,
) -> u64 {
    std::panic::catch_unwind(|| {
        if input.is_null() || len == 0 || len > 12288 {
            return 0;
        }
        let Ok(config) = serde_json::from_slice::<PlaybackConfig>(unsafe {
            std::slice::from_raw_parts(input, len)
        }) else {
            return 0;
        };
        if config.api_version != 1
            || config.output.as_os_str().len() > 4096
            || config
                .cache_root
                .as_ref()
                .is_some_and(|p| p.as_os_str().len() > 4096)
        {
            return 0;
        }
        let mut entries = ENTRIES.lock().unwrap();
        let Some(entry) = entries.get(&id).cloned() else {
            return 0;
        };
        let mut state = entry.result.lock().unwrap();
        if entry.cancel.is_cancelled() || !entry.done.load(Ordering::Acquire) {
            return 0;
        }
        let Some(Ok(resolved)) = state.as_ref() else {
            return 0;
        };
        if resolved.selected_file(index).is_err() {
            return 0;
        }
        let Some(Ok(resolved)) = state.take() else {
            return 0;
        };
        let discovery = entry.discovery.clone();
        let playback = crate::ffi::launch(async move {
            crate::PlaybackSession::start_resolved(
                resolved,
                index,
                config.output,
                discovery,
                config.cache_root,
            )
            .await
        });
        entries.remove(&id);
        playback
    })
    .unwrap_or(0)
}

/// At most 24 original file entries. Returns required bytes; short buffers remain untouched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cue_metadata_snapshot(
    id: u64,
    offset: usize,
    output: *mut u8,
    capacity: usize,
) -> isize {
    std::panic::catch_unwind(|| {
        let Some(entry) = ENTRIES.lock().unwrap().get(&id).cloned() else { return -1; };
        let result = entry.result.lock().unwrap();
        let mut value = if entry.cancel.is_cancelled() {
            serde_json::json!({"state":"stopping"})
        } else { match result.as_ref() {
            None => serde_json::json!({"state":"loading"}),
            Some(Err(error)) => error.snapshot(),
            Some(Ok(resolved)) => {
                if offset > resolved.files.len() { return -1; }
                serde_json::json!({"state":"ready", "offset":offset, "total":resolved.files.len(), "files":resolved.files.iter().skip(offset).take(24).collect::<Vec<_>>()})
            }
        }};
        value["api_version"] = 1.into(); value["session_id"] = id.into();
        let bytes = serde_json::to_vec(&value).unwrap();
        if !output.is_null() && capacity >= bytes.len() { unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len()) }; }
        bytes.len() as isize
    }).unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub extern "C" fn cue_metadata_stop(id: u64) -> i32 {
    std::panic::catch_unwind(|| {
        let Some(entry) = ENTRIES.lock().unwrap().get(&id).cloned() else {
            return -1;
        };
        entry.cancel.cancel();
        *entry.result.lock().unwrap() = None;
        if entry.done.load(Ordering::Acquire) {
            1
        } else {
            0
        }
    })
    .unwrap_or(-1)
}
/// Release is rejected until discovery has joined and stop has been requested.
#[unsafe(no_mangle)]
pub extern "C" fn cue_metadata_release(id: u64) -> i32 {
    std::panic::catch_unwind(|| {
        let mut entries = ENTRIES.lock().unwrap();
        let Some(entry) = entries.get(&id) else {
            return -1;
        };
        if !entry.cancel.is_cancelled() || !entry.done.load(Ordering::Acquire) {
            return 0;
        }
        entries.remove(&id);
        1
    })
    .unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_transfers_metadata_once_and_preserves_invalid_choice_for_retry() {
        let output = tempfile::tempdir().unwrap();
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/torrents/multi-episode.torrent"),
        )
        .unwrap();
        let id = start(
            TorrentSource::Bytes(bytes),
            output.path().to_owned(),
            Discovery::default(),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !ENTRIES
            .lock()
            .unwrap()
            .get(&id)
            .unwrap()
            .done
            .load(Ordering::Acquire)
        {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let config =
            serde_json::to_vec(&serde_json::json!({"api_version":1,"output":output.path()}))
                .unwrap();
        assert_eq!(
            unsafe { cue_metadata_play(id, 99, config.as_ptr(), config.len()) },
            0
        );
        let playback = unsafe { cue_metadata_play(id, 1, config.as_ptr(), config.len()) };
        assert_ne!(playback, 0);
        assert_eq!(
            unsafe { cue_metadata_play(id, 1, config.as_ptr(), config.len()) },
            0
        );
        assert_eq!(cue_metadata_stop(id), -1);
        loop {
            let raw = crate::ffi::cue_torrent_snapshot(playback);
            let value: serde_json::Value =
                serde_json::from_slice(unsafe { std::ffi::CStr::from_ptr(raw) }.to_bytes())
                    .unwrap();
            unsafe {
                crate::ffi::cue_torrent_string_free(raw);
            }
            if value["state"] == "ready" {
                break;
            }
            assert!(std::time::Instant::now() < deadline, "{value}");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let ticket = crate::ffi::cue_torrent_stop_ticket(playback);
        while crate::ffi::cue_torrent_stop_status(ticket) == 0 {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(crate::ffi::cue_torrent_stop_status(ticket), 1);
        crate::ffi::cue_torrent_stop_release(ticket);
    }

    #[test]
    fn retained_metadata_windows_and_shutdown_obey_buffer_contract() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let output = tempfile::tempdir().unwrap();
        let id = start(
            TorrentSource::Bytes(
                std::fs::read(root.join("fixtures/torrents/multi-episode.torrent")).unwrap(),
            ),
            output.path().to_owned(),
            Discovery::default(),
        );
        assert_ne!(id, 0);
        assert_eq!(cue_metadata_release(id), 0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let value = loop {
            let mut buffer = vec![0; 256 * 1024];
            let count = unsafe { cue_metadata_snapshot(id, 1, buffer.as_mut_ptr(), buffer.len()) };
            assert!(count > 0);
            let value: serde_json::Value =
                serde_json::from_slice(&buffer[..count as usize]).unwrap();
            if value["state"] == "ready" {
                break value;
            }
            assert!(std::time::Instant::now() < deadline, "{value}");
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(value["session_id"], id);
        assert_eq!(value["total"], 3);
        assert_eq!(value["files"][0]["index"], 1);
        assert_eq!(value["files"].as_array().unwrap().len(), 2);
        let mut sentinel = [42; 8];
        let needed = unsafe { cue_metadata_snapshot(id, 0, sentinel.as_mut_ptr(), sentinel.len()) };
        assert!(needed > 8);
        assert_eq!(sentinel, [42; 8]);
        assert_eq!(
            unsafe { cue_metadata_snapshot(id, 4, std::ptr::null_mut(), 0) },
            -1
        );
        while cue_metadata_stop(id) == 0 {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(cue_metadata_release(id), 1);
        assert_eq!(cue_metadata_release(id), -1);
        assert_eq!(
            unsafe { cue_metadata_snapshot(id, 0, std::ptr::null_mut(), 0) },
            -1
        );
        assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 0);
    }
}
