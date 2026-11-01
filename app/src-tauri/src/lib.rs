//! Kuchu's core: the pet's window (placed in the screen's work area, never taking the focus, letting clicks through
//! everywhere but the pet), the pointer read for the pet's eyes, and the tray. The pet's page (`app/src`) draws it.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow, Wry};

/// The pet's widths, as the tray offers them (px; its height is 154/120 of that).
const SIZES: [(&str, &str, u32); 3] = [("size-small", "Small", 64), ("size-base", "Base", 96), ("size-big", "Big", 128)];
const BASE: u32 = 96;
/// Where the pet sits in its window: 16 px from the side, 14 px from the bottom, so its shadow has room.
const SIDE: f64 = 16.0;
const BOTTOM: f64 = 14.0;
/// How often the pointer is read: often enough for the eyes and for the pet to take a click at once.
const POLL: Duration = Duration::from_millis(40);
/// The pointer counts as on the pet this close to its edge, so a click on its outline is not lost.
const NEAR: f64 = 4.0;

fn pet_height(width: u32) -> f64 {
    (width as f64 * 154.0 / 120.0).round()
}

/// The window around a pet `width` wide (logical px).
fn box_of(width: u32) -> (f64, f64) {
    (width as f64 + 2.0 * SIDE, pet_height(width) + BOTTOM + 22.0)
}

/// What the page needs from the pointer: where it is from the pet's centre, and whether it is on the pet.
#[derive(Clone, Serialize, PartialEq)]
struct Pointer {
    dx: f64,
    dy: f64,
    over: bool,
}

struct Pet {
    width: AtomicU32,
    sounds: AtomicBool,
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let pet = Arc::new(Pet { width: AtomicU32::new(BASE), sounds: AtomicBool::new(true) });
            app.manage(pet.clone());
            let window = app.get_webview_window("pet").expect("the pet's window is in tauri.conf.json");
            // Clicks go through until the pointer is on the pet (see `follow_pointer`).
            let _ = window.set_ignore_cursor_events(true);
            place(&window, BASE);
            let _ = window.show();
            tray(app.handle())?;
            follow_pointer(app.handle().clone(), window, pet);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Kuchu could not start");
}

/// The pet's first place: the bottom-right of the work area of the screen it is on (above the taskbar or dock), 24 px
/// from the side and 80 px up, clear of other apps' bars.
fn place(window: &WebviewWindow, width: u32) {
    let (w, h) = box_of(width);
    let _ = window.set_size(LogicalSize::new(w, h));
    let Ok(Some(monitor)) = window.current_monitor().or_else(|_| window.primary_monitor()) else { return };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let x = area.position.x as f64 + area.size.width as f64 - (w + 24.0) * scale;
    let y = area.position.y as f64 + area.size.height as f64 - (h + 80.0) * scale;
    let _ = window.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
}

/// A new size, the pet's bottom-left corner where it was (the window grows up and to the right).
fn resize(window: &WebviewWindow, from: u32, to: u32) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let (_, h0) = box_of(from);
    let (w1, h1) = box_of(to);
    if let Ok(at) = window.outer_position() {
        let y = at.y as f64 + (h0 - h1) * scale;
        let _ = window.set_size(LogicalSize::new(w1, h1));
        let _ = window.set_position(PhysicalPosition::new(at.x, y.round() as i32));
    }
}

/// Reads the pointer a few times a second while the pet shows: tells the page where it is (for the eyes, and for
/// see-through), and lets the window take the mouse only while the pointer is on the pet. Polled rather than taken
/// from the window's own mouse events, which stop as soon as clicks go through, and which a window left by a fast
/// pointer never hears about.
fn follow_pointer(app: AppHandle, window: WebviewWindow, pet: Arc<Pet>) {
    std::thread::spawn(move || {
        let mut last: Option<Pointer> = None;
        let mut taking = false;
        loop {
            std::thread::sleep(POLL);
            if !window.is_visible().unwrap_or(false) {
                continue;
            }
            let (Ok(cursor), Ok(at), Ok(scale)) = (app.cursor_position(), window.outer_position(), window.scale_factor()) else {
                continue;
            };
            let width = pet.width.load(Ordering::Relaxed) as f64;
            let height = pet_height(width as u32);
            let (_, box_h) = box_of(width as u32);
            // The pointer in the window's own logical px, then from the pet's centre.
            let x = (cursor.x - at.x as f64) / scale;
            let y = (cursor.y - at.y as f64) / scale;
            let (left, top) = (SIDE, box_h - BOTTOM - height);
            let over = x >= left - NEAR && x <= left + width + NEAR && y >= top - NEAR && y <= top + height + NEAR;
            let now = Pointer { dx: (x - left - width / 2.0).round(), dy: (y - top - height / 2.0).round(), over };
            if over != taking {
                taking = over;
                let _ = window.set_ignore_cursor_events(!over);
            }
            if last.as_ref() != Some(&now) {
                let _ = window.emit("kuchu://pointer", now.clone());
                last = Some(now);
            }
        }
    });
}

/// The tray: always a way back to the pet, its size, its sounds, and (until sessions arrive) a tour of its moods.
fn tray(app: &AppHandle) -> tauri::Result<()> {
    let try_moods = MenuItem::with_id(app, "try-moods", "Try the moods", true, None::<&str>)?;
    let sizes: Vec<CheckMenuItem<Wry>> = SIZES
        .iter()
        .map(|(id, label, px)| CheckMenuItem::with_id(app, *id, *label, true, *px == BASE, None::<&str>))
        .collect::<tauri::Result<_>>()?;
    let size_refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = sizes.iter().map(|s| s as &dyn tauri::menu::IsMenuItem<Wry>).collect();
    let size = Submenu::with_items(app, "Size", true, &size_refs)?;
    let sounds = CheckMenuItem::with_id(app, "sounds", "Sounds", true, true, None::<&str>)?;
    let shown = CheckMenuItem::with_id(app, "shown", "Show Kuchu", true, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Kuchu", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&shown, &try_moods, &PredefinedMenuItem::separator(app)?, &size, &sounds, &PredefinedMenuItem::separator(app)?, &quit],
    )?;

    let mut builder = TrayIconBuilder::with_id("kuchu").tooltip("Kuchu").menu(&menu).show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let shown_item = shown.clone();
    builder
        .on_menu_event(move |app, event| {
            let Some(window) = app.get_webview_window("pet") else { return };
            let pet = app.state::<Arc<Pet>>();
            match event.id().as_ref() {
                "try-moods" => {
                    let _ = window.show();
                    let _ = shown_item.set_checked(true);
                    let _ = window.emit("kuchu://try-moods", ());
                }
                "sounds" => {
                    let on = !pet.sounds.load(Ordering::Relaxed);
                    pet.sounds.store(on, Ordering::Relaxed);
                    let _ = window.emit("kuchu://sounds", on);
                }
                "shown" => {
                    let show = !window.is_visible().unwrap_or(true);
                    let _ = if show { window.show() } else { window.hide() };
                    let _ = shown_item.set_checked(show);
                }
                "quit" => app.exit(0),
                id => {
                    if let Some((_, _, px)) = SIZES.iter().find(|(sid, _, _)| *sid == id) {
                        let from = pet.width.swap(*px, Ordering::Relaxed);
                        resize(&window, from, *px);
                        let _ = window.emit("kuchu://size", *px);
                        for (item, (_, _, p)) in sizes.iter().zip(SIZES.iter()) {
                            let _ = item.set_checked(p == px);
                        }
                    }
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            // A click on the tray icon always brings the pet back.
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                if let Some(window) = tray.app_handle().get_webview_window("pet") {
                    let _ = window.show();
                }
            }
        })
        .build(app)?;
    Ok(())
}
