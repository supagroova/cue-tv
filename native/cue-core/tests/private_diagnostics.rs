use cue_core::ffi::cue_core_configure_private_diagnostics;

#[test]
fn private_panic_policy_child() {
    if std::env::var_os("CUE_PRIVATE_PANIC_CHILD").is_none() {
        return;
    }
    assert_eq!(cue_core_configure_private_diagnostics(), 0);
    assert_eq!(cue_core_configure_private_diagnostics(), 0);
    assert!(
        std::panic::catch_unwind(|| panic!(
            "synthetic-account-token https://fixture.invalid/private-title"
        ))
        .is_err()
    );
    assert!(
        std::thread::spawn(|| panic!("synthetic-tracker-passkey private-video-id"))
            .join()
            .is_err()
    );
}

#[test]
fn opted_in_panic_output_contains_only_a_fixed_category() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "private_panic_policy_child", "--nocapture"])
        .env("CUE_PRIVATE_PANIC_CHILD", "1")
        .env("RUST_BACKTRACE", "full")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Cue TV: native panic.\nCue TV: native panic.\n"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    for private in [
        "synthetic-account-token",
        "https://",
        "private-title",
        "synthetic-tracker-passkey",
        "private-video-id",
    ] {
        assert!(!stdout.contains(private));
    }
}
