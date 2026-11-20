use cue_core::ffi::*;
use std::time::{Duration, Instant};
// These tests exercise one process-global handle table, including its full capacity.
static BRIDGE: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn state(handle: u64) -> serde_json::Value {
    let mut bytes = vec![0u8; 4096];
    let len = unsafe { cue_core_snapshot(handle, bytes.as_mut_ptr(), bytes.len()) };
    assert!(len > 0 && len <= bytes.len() as isize);
    serde_json::from_slice(&bytes[..len as usize]).unwrap()
}
fn until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(Instant::now() < deadline, "native bridge did not settle");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn native_boundary_owns_commands_snapshots_and_shutdown() {
    let _exclusive = BRIDGE.lock().unwrap();
    assert_eq!(cue_core_abi_version(), 1);
    let handle = cue_core_create();
    assert_ne!(handle, 0);
    assert_eq!(cue_core_release(handle), 0);
    until(|| state(handle)["status"] == "ready");
    let revision = state(handle)["model"]["revision"].as_u64().unwrap();
    assert_eq!(cue_core_request_library(handle, 2, revision), 0);
    assert_eq!(cue_core_request_playback(handle, 0, 0, 0, 0), 0);
    assert_eq!(cue_core_request_playback(handle, 1, 8, 0, 0), 0);
    assert_eq!(cue_core_request_playback(handle, 1, 0, 1, 0), 0);
    assert_eq!(cue_core_request_playback(handle, 1, 1, u64::MAX, 1000), 0);
    assert_eq!(cue_core_request_playback(handle, 1, 2, 0, u64::MAX), 0);
    for action in 0..8 {
        let ticket = cue_core_request_playback(handle, u64::MAX, action, 0, 0);
        assert_ne!(ticket, 0);
        assert_eq!(cue_core_request_playback(handle, 1, 0, 0, 0), 0);
        until(|| unsafe { cue_core_take_torrent(handle, ticket, std::ptr::null_mut(), 0) } > 0);
        let mut bytes = vec![0; 4096];
        let len = unsafe { cue_core_take_torrent(handle, ticket, bytes.as_mut_ptr(), bytes.len()) };
        let receipt: serde_json::Value = serde_json::from_slice(&bytes[..len as usize]).unwrap();
        assert_eq!(receipt["session"], handle);
        assert_eq!(receipt["ticket"], ticket);
        assert_eq!(receipt["error"], "stale_playback");
        assert!(receipt["request"].is_null());
    }

    unsafe {
        assert_eq!(
            cue_core_request_watched(handle, std::ptr::null(), 1, 1, revision),
            0
        );
        assert_eq!(
            cue_core_request_watched(handle, std::ptr::null(), 0, 2, revision),
            0
        );
        assert_eq!(
            cue_core_request_watched(handle, [b'x'; 513].as_ptr(), 513, 1, revision),
            0
        );
        assert_eq!(
            cue_core_request_watched(handle, [255u8].as_ptr(), 1, 1, revision),
            0
        );
        let watched = cue_core_request_watched(handle, std::ptr::null(), 0, 1, revision);
        assert_ne!(watched, 0);
        assert_eq!(cue_core_request_library(handle, 1, revision), 0);
        until(|| cue_core_take_torrent(handle, watched, std::ptr::null_mut(), 0) > 0);
        let mut bytes = vec![0; 4096];
        let len = cue_core_take_torrent(handle, watched, bytes.as_mut_ptr(), bytes.len());
        let receipt: serde_json::Value = serde_json::from_slice(&bytes[..len as usize]).unwrap();
        assert_eq!(receipt["session"], handle);
        assert_eq!(receipt["ticket"], watched);
        assert_eq!(receipt["error"], "details_not_ready");
        assert!(receipt["request"].is_null());
    }
    let library = cue_core_request_library(handle, 1, revision);
    assert_ne!(library, 0);
    assert_eq!(cue_core_request_torrent(handle, 0, revision), 0);
    until(|| unsafe { cue_core_take_torrent(handle, library, std::ptr::null_mut(), 0) } > 0);
    let mut library_bytes = vec![0; 4096];
    let library_len = unsafe {
        cue_core_take_torrent(
            handle,
            library,
            library_bytes.as_mut_ptr(),
            library_bytes.len(),
        )
    };
    let library_reply: serde_json::Value =
        serde_json::from_slice(&library_bytes[..library_len as usize]).unwrap();
    assert_eq!(library_reply["ticket"], library);
    assert_eq!(library_reply["session"], handle);
    assert_eq!(library_reply["error"], "details_not_ready");
    assert!(library_reply["request"].is_null());
    let ticket = cue_core_request_torrent(handle, 0, revision);
    assert_ne!(ticket, 0);
    assert_eq!(cue_core_request_torrent(handle, 0, revision), 0);
    assert_eq!(cue_core_request_https(handle, 0, revision), 0);
    until(|| unsafe { cue_core_take_torrent(handle, ticket, std::ptr::null_mut(), 0) } > 0);
    let mut short = [77u8; 1];
    assert!(unsafe { cue_core_take_torrent(handle, ticket, short.as_mut_ptr(), 1) } > 1);
    assert_eq!(short, [77]);
    let mut result = vec![0; 4096];
    let length =
        unsafe { cue_core_take_torrent(handle, ticket, result.as_mut_ptr(), result.len()) };
    let result: serde_json::Value = serde_json::from_slice(&result[..length as usize]).unwrap();
    assert_eq!(result["ticket"], ticket);
    assert_eq!(result["error"], "invalid_source");
    assert_eq!(
        unsafe { cue_core_take_torrent(handle, ticket, std::ptr::null_mut(), 0) },
        -1
    );
    let cancelled = cue_core_request_torrent(handle, 0, revision);
    assert_ne!(cancelled, ticket);
    assert_eq!(cue_core_discard_torrent(handle, cancelled), 1);
    assert_eq!(
        unsafe { cue_core_take_torrent(handle, cancelled, std::ptr::null_mut(), 0) },
        -1
    );

    assert_eq!(
        unsafe { cue_core_dispatch(handle, std::ptr::null(), 2) },
        -2
    );
    for invalid in [
        r#"{"version":2,"action":"set_language","language":"fra"}"#,
        r#"{"version":1,"action":"set_language","language":"../"}"#,
        r#"{"version":1,"action":"set_language","language":"fra","token":"private"}"#,
    ] {
        assert_eq!(
            unsafe { cue_core_dispatch(handle, invalid.as_ptr(), invalid.len()) },
            -2
        );
    }
    let stale = br#"{"version":1,"action":"install_addon","revision":999}"#;
    assert_eq!(
        unsafe { cue_core_dispatch(handle, stale.as_ptr(), stale.len()) },
        0
    );
    until(|| state(handle)["error"] == "stale_preview");
    assert_eq!(state(handle)["status"], "ready");
    let command = br#"{"version":1,"action":"set_language","language":"fra"}"#;
    assert_eq!(
        unsafe { cue_core_dispatch(handle, command.as_ptr(), command.len()) },
        0
    );
    until(|| state(handle)["model"]["interface_language"] == "fra");
    let mut tiny = [91u8; 2];
    assert!(unsafe { cue_core_snapshot(handle, tiny.as_mut_ptr(), tiny.len()) } > 2);
    assert_eq!(tiny, [91, 91]);
    let retired_ticket = cue_core_request_torrent(
        handle,
        0,
        state(handle)["model"]["revision"].as_u64().unwrap(),
    );
    assert_ne!(retired_ticket, 0);
    assert_eq!(cue_core_stop(handle), 0);
    assert_eq!(
        unsafe { cue_core_take_torrent(handle, retired_ticket, std::ptr::null_mut(), 0) },
        -1
    );
    assert_eq!(cue_core_request_torrent(handle, 0, 0), 0);
    until(|| cue_core_stop_status(handle) == 1);
    assert_eq!(state(handle)["status"], "stopped");
    assert_eq!(cue_core_stop(handle), 1);
    assert_eq!(
        unsafe { cue_core_dispatch(handle, command.as_ptr(), command.len()) },
        -1
    );
    assert_eq!(cue_core_release(handle), 1);
    assert_eq!(cue_core_stop_status(handle), -1);
    let next = cue_core_create();
    assert!(next > handle);
    cue_core_stop(next);
    until(|| cue_core_stop_status(next) == 1);
    assert_eq!(cue_core_release(next), 1);
    let handles: Vec<_> = (0..8).map(|_| cue_core_create()).collect();
    assert!(handles.iter().all(|handle| *handle != 0));
    assert_eq!(
        cue_core_create(),
        0,
        "unreleased sessions must remain bounded"
    );
    for handle in handles {
        cue_core_stop(handle);
        until(|| cue_core_stop_status(handle) == 1);
        assert_eq!(cue_core_release(handle), 1);
    }
    let directory = std::env::temp_dir().join(format!(
        "cue-core-ffi-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.to_str().unwrap().as_bytes();
    let key = [19u8; 32];
    let create = || unsafe {
        cue_core_create_persistent(path.as_ptr(), path.len(), key.as_ptr(), key.len())
    };
    assert_eq!(
        unsafe { cue_core_create_persistent(std::ptr::null(), 0, key.as_ptr(), 32) },
        0
    );
    assert_eq!(
        unsafe { cue_core_create_persistent(path.as_ptr(), path.len(), key.as_ptr(), 31) },
        0
    );
    let persisted = create();
    assert_ne!(persisted, 0);
    until(|| state(persisted)["status"] == "ready");
    let duplicate = create();
    until(|| cue_core_stop_status(duplicate) == 1);
    assert_eq!(state(duplicate)["error"], "core_start_failed");
    assert_eq!(cue_core_release(duplicate), 1);
    assert_eq!(
        unsafe { cue_core_dispatch(persisted, command.as_ptr(), command.len()) },
        0
    );
    until(|| {
        state(persisted)["model"]["interface_language"] == "fra"
            && state(persisted)["model"]["pending_effects"] == 0
    });
    cue_core_stop(persisted);
    until(|| cue_core_stop_status(persisted) == 1);
    assert_eq!(cue_core_release(persisted), 1);
    let restored = create();
    until(|| state(restored)["status"] == "ready");
    assert_eq!(state(restored)["model"]["interface_language"], "fra");
    cue_core_stop(restored);
    until(|| cue_core_stop_status(restored) == 1);
    assert_eq!(cue_core_release(restored), 1);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn guest_login_is_rejected_without_publishing_credentials() {
    let _exclusive = BRIDGE.lock().unwrap();
    let handle = cue_core_create();
    assert_ne!(handle, 0);
    until(|| state(handle)["status"] == "ready");
    let email = b"fixture@example.test";
    let password = b"synthetic-password";
    assert_eq!(
        unsafe {
            cue_core_login(
                handle,
                email.as_ptr(),
                email.len(),
                password.as_ptr(),
                password.len(),
            )
        },
        0
    );
    until(|| state(handle)["error"] == "account_login_unavailable");
    let snapshot = state(handle).to_string();
    assert!(!snapshot.contains("synthetic-password"));
    assert!(!snapshot.contains("fixture@example.test"));
    cue_core_stop(handle);
    until(|| cue_core_stop_status(handle) == 1);
    assert_eq!(cue_core_release(handle), 1);
}

#[test]
fn new_account_requires_empty_storage_and_owns_shutdown() {
    let _exclusive = BRIDGE.lock().unwrap();
    let directory = std::env::temp_dir().join(format!(
        "cue-account-ffi-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.to_str().unwrap().as_bytes();
    let key = [57; 32];
    for expected in ["ready", "stopped"] {
        let handle =
            unsafe { cue_core_create_account(path.as_ptr(), path.len(), key.as_ptr(), key.len()) };
        assert_ne!(handle, 0);
        until(|| state(handle)["status"] != "starting");
        assert_eq!(state(handle)["status"], expected);
        if expected == "ready" {
            let revision = state(handle)["model"]["revision"].as_u64().unwrap();
            let command = br#"{"version":1,"action":"logout"}"#;
            assert_eq!(
                unsafe { cue_core_dispatch(handle, command.as_ptr(), command.len()) },
                0
            );
            until(|| state(handle)["model"]["revision"].as_u64().unwrap() > revision);
            assert_eq!(state(handle)["model"]["authenticated"], false);
        } else {
            assert_eq!(state(handle)["error"], "core_start_failed");
        }
        cue_core_stop(handle);
        until(|| cue_core_stop_status(handle) == 1);
        assert_eq!(cue_core_release(handle), 1);
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn https_handoff_uses_bounded_ticket_and_rejects_unavailable_sources() {
    let _exclusive = BRIDGE.lock().unwrap();
    let handle = cue_core_create();
    until(|| state(handle)["status"] == "ready");
    let revision = state(handle)["model"]["revision"].as_u64().unwrap();
    let ticket = cue_core_request_https(handle, 0, revision);
    assert_ne!(ticket, 0);
    assert_eq!(cue_core_request_torrent(handle, 0, revision), 0);
    until(|| unsafe { cue_core_take_torrent(handle, ticket, std::ptr::null_mut(), 0) } > 0);
    let mut bytes = vec![0; 32768];
    let count = unsafe { cue_core_take_torrent(handle, ticket, bytes.as_mut_ptr(), bytes.len()) };
    assert!(count > 0);
    let reply: serde_json::Value = serde_json::from_slice(&bytes[..count as usize]).unwrap();
    assert_eq!(reply["error"], "invalid_source");
    assert!(reply["request"].is_null());
    assert_eq!(
        unsafe { cue_core_take_torrent(handle, ticket, std::ptr::null_mut(), 0) },
        -1
    );
    cue_core_stop(handle);
    until(|| cue_core_stop_status(handle) == 1);
    assert_eq!(cue_core_request_https(handle, 0, revision), 0);
    assert_eq!(cue_core_release(handle), 1);
}

#[test]
fn subtitle_handoff_validates_input_and_shares_the_private_ticket_slot() {
    let _exclusive = BRIDGE.lock().unwrap();
    let handle = cue_core_create();
    until(|| state(handle)["status"] == "ready");
    for input in [
        r#"{"version":2,"meta_id":"tt1","video_id":"tt1:1:1","kind":"series"}"#,
        r#"{"version":1,"meta_id":"","video_id":"tt1:1:1","kind":"series"}"#,
        r#"{"version":1,"meta_id":"tt1","video_id":"tt1:1:1","kind":"series","token":"private"}"#,
    ] {
        assert_eq!(
            unsafe { cue_core_request_subtitles(handle, input.as_ptr(), input.len()) },
            0
        );
    }
    assert_eq!(
        unsafe { cue_core_request_subtitles(handle, std::ptr::null(), 1) },
        0
    );
    let input = br#"{"version":1,"meta_id":"tt1","video_id":"tt1:1:1","kind":"series"}"#;
    let ticket = unsafe { cue_core_request_subtitles(handle, input.as_ptr(), input.len()) };
    assert_ne!(ticket, 0);
    assert_eq!(cue_core_request_https(handle, 0, 0), 0);
    until(|| unsafe { cue_core_take_torrent(handle, ticket, std::ptr::null_mut(), 0) } > 0);
    let mut bytes = vec![0; 4096];
    let len = unsafe { cue_core_take_torrent(handle, ticket, bytes.as_mut_ptr(), bytes.len()) };
    let reply: serde_json::Value = serde_json::from_slice(&bytes[..len as usize]).unwrap();
    assert_eq!(reply["error"], "stale_subtitle_identity");
    assert_eq!(reply["ticket"], ticket);
    let cancelled = unsafe { cue_core_request_subtitles(handle, input.as_ptr(), input.len()) };
    assert_ne!(cancelled, 0);
    assert_eq!(cue_core_discard_torrent(handle, cancelled), 1);
    assert_eq!(
        unsafe { cue_core_take_torrent(handle, cancelled, std::ptr::null_mut(), 0) },
        -1
    );
    cue_core_stop(handle);
    until(|| cue_core_stop_status(handle) == 1);
    assert_eq!(cue_core_release(handle), 1);
}

#[test]
fn refresh_credentials_are_bounded_private_and_rejected_for_guests() {
    let _exclusive = BRIDGE.lock().unwrap();
    let handle = cue_core_create();
    until(|| state(handle)["status"] == "ready");
    let email = b"fixture@example.test";
    let password = b"synthetic-password";
    for invalid in [
        vec![],
        vec![0xff],
        b"bad\npassword".to_vec(),
        vec![b'x'; 4097],
    ] {
        assert_eq!(
            unsafe {
                cue_core_reauthenticate(
                    handle,
                    email.as_ptr(),
                    email.len(),
                    invalid.as_ptr(),
                    invalid.len(),
                )
            },
            -2
        );
    }
    assert_eq!(
        unsafe {
            cue_core_reauthenticate(
                handle,
                std::ptr::null(),
                1,
                password.as_ptr(),
                password.len(),
            )
        },
        -2
    );
    let json = br#"{"version":1,"action":"reauthenticate","email":"fixture@example.test","password":"synthetic-password"}"#;
    assert_eq!(
        unsafe { cue_core_dispatch(handle, json.as_ptr(), json.len()) },
        -2
    );
    assert_eq!(
        unsafe {
            cue_core_reauthenticate(
                handle,
                email.as_ptr(),
                email.len(),
                password.as_ptr(),
                password.len(),
            )
        },
        0
    );
    until(|| state(handle)["error"] == "account_sync_unavailable");
    let snapshot = state(handle).to_string();
    assert!(!snapshot.contains("synthetic-password"));
    assert!(!snapshot.contains("fixture@example.test"));
    assert_eq!(state(handle)["model"]["authenticated"], false);
    cue_core_stop(handle);
    until(|| cue_core_stop_status(handle) == 1);
    assert_eq!(cue_core_release(handle), 1);
    assert_eq!(
        unsafe {
            cue_core_reauthenticate(
                handle,
                email.as_ptr(),
                email.len(),
                password.as_ptr(),
                password.len(),
            )
        },
        -1
    );
}
