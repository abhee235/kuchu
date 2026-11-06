//! The relay runs on PCs where no Visual C++ runtime was ever installed: Claude Code starts it from the person's own
//! folder, outside any package or installer, so it may need nothing Windows does not have. Its runtime is built in, as
//! Tauri does for Kuchu itself; Windows' own C runtime (the UCRT) stays shared.

fn main() {
    static_vcruntime::metabuild();
}
