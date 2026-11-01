// No console window with the app on Windows (release builds).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    kuchu_lib::run()
}
