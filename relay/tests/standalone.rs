//! What the relay needs from the PC it runs on.

/// Claude Code runs the relay on PCs without Visual C++ installed (a fresh Windows has none): one that needed
/// `vcruntime140.dll` would not even start there, and every hook would fail.
#[test]
#[cfg(windows)]
fn the_relay_starts_on_a_pc_without_visual_cpp() {
    let exe = std::fs::read(env!("CARGO_BIN_EXE_kuchu-relay")).unwrap();
    let text = String::from_utf8_lossy(&exe).to_ascii_lowercase();
    assert!(!text.contains("vcruntime140"), "the relay links the Visual C++ runtime as a DLL");
}
