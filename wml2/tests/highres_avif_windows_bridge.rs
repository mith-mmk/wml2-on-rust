//! Launch the public AVIF bridge as a normal executable on Windows.

#[cfg(windows)]
#[test]
fn public_native_bridge_runs_through_a_real_main() {
    let executable = env!("CARGO_BIN_EXE_highres_avif_windows_main");
    let status = std::process::Command::new(executable)
        .status()
        .expect("Windows bridge executable should start");
    assert!(
        status.success(),
        "Windows bridge executable exited with {status}"
    );
}

#[cfg(not(windows))]
#[test]
fn public_native_bridge_is_windows_only() {}
