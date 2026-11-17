use cue_core::ffi::*;
use std::time::{Duration, Instant};
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
    assert_eq!(cue_core_abi_version(), 1);
    let handle = cue_core_create();
    assert_ne!(handle, 0);
    assert_eq!(cue_core_release(handle), 0);
    until(|| state(handle)["status"] == "ready");
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
    let command = br#"{"version":1,"action":"set_language","language":"fra"}"#;
    assert_eq!(
        unsafe { cue_core_dispatch(handle, command.as_ptr(), command.len()) },
        0
    );
    until(|| state(handle)["model"]["interface_language"] == "fra");
    let mut tiny = [91u8; 2];
    assert!(unsafe { cue_core_snapshot(handle, tiny.as_mut_ptr(), tiny.len()) } > 2);
    assert_eq!(tiny, [91, 91]);
    assert_eq!(cue_core_stop(handle), 0);
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
}
