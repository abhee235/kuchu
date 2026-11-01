//! Kuchu's core: the pet's window (placed in the screen's work area, never taking the focus, letting clicks through
//! everywhere but the pet and its cards), the pointer read for the pet's eyes, the sessions list and Settings windows,
//! the tray and the shortcut. The hub (`hub.rs`) takes the relay's events in; the pages in `app/src` draw it all.

mod hooks;
mod hub;
mod prefs;
mod sessions;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{Builder as ShortcutBuilder, ShortcutState};

use hub::{Choice, Hub, View};
use prefs::Prefs;

/// Where the pet sits in its own box: 16 px from the side, 14 px from the bottom, room above for its antenna.
const SIDE: f64 = 16.0;
const BOTTOM: f64 = 14.0;
const ABOVE: f64 = 22.0;
/// The room the window gains while a card shows: beside the pet (away from the screen's edge) and above it.
const CARD_W: f64 = 320.0;
const CARD_H: f64 = 210.0;
/// How often the pointer is read: often enough for the eyes, and for the pet to take a click at once.
const POLL: Duration = Duration::from_millis(40);
/// The pointer counts as on the pet or a card this close to its edge, so a click on an outline is not lost.
const NEAR: f64 = 4.0;
/// A drag is over once the window has not moved for this long.
const SETTLE: Duration = Duration::from_millis(300);
/// The shortcut that opens the sessions list from anywhere.
const SHORTCUT: &str = "ctrl+alt+k";
const SESSIONS_SIZE: (f64, f64) = (360.0, 460.0);

fn pet_height(width: u32) -> f64 {
    (width as f64 * 154.0 / 120.0).round()
}

/// The pet's own box (logical px).
fn box_of(width: u32) -> (f64, f64) {
    (width as f64 + 2.0 * SIDE, pet_height(width) + BOTTOM + ABOVE)
}

/// A part of the pet's window that takes the mouse, in the window's logical px. The first is the pet itself.
#[derive(Clone, Copy, Deserialize, Serialize, Debug, Default)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn holds(&self, x: f64, y: f64) -> bool {
        x >= self.x - NEAR && x <= self.x + self.w + NEAR && y >= self.y - NEAR && y <= self.y + self.h + NEAR
    }
}

/// What the page needs from the pointer: where it is from the pet's centre, and whether it is on the pet.
#[derive(Clone, Serialize, PartialEq)]
struct Pointer {
    dx: f64,
    dy: f64,
    over: bool,
}

/// Where the pet is and what its window holds.
struct Layout {
    /// The pet's box, top-left, physical px.
    pet: Mutex<(i32, i32)>,
    /// A card shows: the window is bigger, the pet staying where it is.
    wide: AtomicBool,
    /// The pet is on the screen's right half: its cards go to its left.
    right: AtomicBool,
    rects: Mutex<Vec<Rect>>,
    moved: Mutex<Option<Instant>>,
}

struct Kuchu {
    prefs: Mutex<Prefs>,
    prefs_path: PathBuf,
    layout: Layout,
    hub: Arc<Hub>,
    /// The relay where the hooks point (copied there at start).
    relay: Mutex<Result<PathBuf, String>>,
}

impl Kuchu {
    fn width(&self) -> u32 {
        self.prefs.lock().unwrap().width()
    }

    /// The window around the pet: its own box, or with room for a card beside and above it.
    fn window_rect(&self, scale: f64) -> (i32, i32, f64, f64) {
        let (bw, bh) = box_of(self.width());
        let (px, py) = *self.layout.pet.lock().unwrap();
        if !self.layout.wide.load(Ordering::Relaxed) {
            return (px, py, bw, bh);
        }
        let more_x = (CARD_W * scale).round() as i32;
        let more_y = (CARD_H * scale).round() as i32;
        let x = if self.layout.right.load(Ordering::Relaxed) { px - more_x } else { px };
        (x, py - more_y, bw + CARD_W, bh + CARD_H)
    }

    /// Where the pet is, from where its window is.
    fn pet_from_window(&self, at: PhysicalPosition<i32>, scale: f64) -> (i32, i32) {
        if !self.layout.wide.load(Ordering::Relaxed) {
            return (at.x, at.y);
        }
        let more_x = (CARD_W * scale).round() as i32;
        let more_y = (CARD_H * scale).round() as i32;
        let x = if self.layout.right.load(Ordering::Relaxed) { at.x + more_x } else { at.x };
        (x, at.y + more_y)
    }

    /// The pet's box in the window, in the window's logical px.
    fn pet_in_window(&self) -> Rect {
        let width = self.width() as f64;
        let height = pet_height(self.width());
        let (bw, _) = box_of(self.width());
        let (w, h) = if self.layout.wide.load(Ordering::Relaxed) { (bw + CARD_W, box_of(self.width()).1 + CARD_H) } else { box_of(self.width()) };
        let x = if self.layout.wide.load(Ordering::Relaxed) && self.layout.right.load(Ordering::Relaxed) { w - SIDE - width } else { SIDE };
        Rect { x, y: h - BOTTOM - height, w: width, h: height }
    }

    fn place(&self, window: &WebviewWindow) {
        let scale = window.scale_factor().unwrap_or(1.0);
        let (x, y, w, h) = self.window_rect(scale);
        let _ = window.set_size(LogicalSize::new(w, h));
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }

    /// Which half of its screen the pet is on; told to the page when it changes.
    fn settle(&self, window: &WebviewWindow) {
        let scale = window.scale_factor().unwrap_or(1.0);
        let (px, py) = *self.layout.pet.lock().unwrap();
        let (bw, bh) = box_of(self.width());
        let centre = (px as f64 + bw * scale / 2.0, py as f64 + bh * scale / 2.0);
        if let Ok(Some(monitor)) = window.monitor_from_point(centre.0, centre.1) {
            let area = monitor.work_area();
            let right = centre.0 > area.position.x as f64 + area.size.width as f64 / 2.0;
            if self.layout.right.swap(right, Ordering::Relaxed) != right {
                let _ = window.emit("kuchu://side", right);
                if self.layout.wide.load(Ordering::Relaxed) {
                    self.place(window);
                }
            }
        }
        let mut prefs = self.prefs.lock().unwrap();
        if prefs.at != Some((px, py)) {
            prefs.at = Some((px, py));
            prefs.save(&self.prefs_path);
        }
    }

    fn set_prefs(&self, app: &AppHandle, next: Prefs) -> Prefs {
        let before = self.prefs.lock().unwrap().clone();
        *self.prefs.lock().unwrap() = next.clone();
        next.save(&self.prefs_path);
        self.hub.approvals_on.store(next.approvals, Ordering::Relaxed);
        if before.width() != next.width() {
            if let Some(window) = app.get_webview_window("pet") {
                // The pet's bottom corner nearest the screen's edge stays where it is.
                let scale = window.scale_factor().unwrap_or(1.0);
                let ((w0, h0), (w1, h1)) = (box_of(before.width()), box_of(next.width()));
                let mut pet = self.layout.pet.lock().unwrap();
                pet.1 += ((h0 - h1) * scale).round() as i32;
                if self.layout.right.load(Ordering::Relaxed) {
                    pet.0 += ((w0 - w1) * scale).round() as i32;
                }
                drop(pet);
                self.place(&window);
            }
        }
        let _ = app.emit("kuchu://prefs", next.clone());
        next
    }
}

/// The pet's first place: the bottom-right of the work area (above the taskbar or dock), 24 px in and 80 px up.
fn first_place(window: &WebviewWindow, width: u32) -> (i32, i32) {
    let (w, h) = box_of(width);
    let Ok(Some(monitor)) = window.primary_monitor() else { return (100, 100) };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let x = area.position.x as f64 + area.size.width as f64 - (w + 24.0) * scale;
    let y = area.position.y as f64 + area.size.height as f64 - (h + 80.0) * scale;
    (x.round() as i32, y.round() as i32)
}

/// A place saved earlier, if it is still on a screen.
fn still_on_screen(window: &WebviewWindow, at: (i32, i32)) -> bool {
    window.monitor_from_point(at.0 as f64 + 40.0, at.1 as f64 + 40.0).ok().flatten().is_some()
}

/// The relay shipped beside Kuchu (or, while developing, the one cargo built).
fn bundled_relay() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    [dir.join(hooks::RELAY_FILE), dir.join("..").join("release").join(hooks::RELAY_FILE), dir.join("..").join("debug").join(hooks::RELAY_FILE)]
        .into_iter()
        .find(|p| p.is_file())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            ShortcutBuilder::new()
                .with_shortcuts([SHORTCUT])
                .expect("a valid shortcut")
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(pet) = app.get_webview_window("pet") {
                            let _ = pet.show();
                        }
                        toggle_sessions(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            view,
            get_prefs,
            set_prefs,
            pet_clicked,
            forget,
            decide,
            approval_shown,
            set_wide,
            set_rects,
            is_right,
            hooks_status,
            hooks_plan,
            hooks_apply,
            open_settings,
            open_sessions,
        ])
        .setup(|app| {
            let config = app.path().app_config_dir()?;
            let prefs_path = prefs::path_in(config);
            let prefs = Prefs::load(&prefs_path);
            let hub = Hub::new(app.handle().clone(), prefs.approvals);
            let window = app.get_webview_window("pet").expect("the pet's window is in tauri.conf.json");
            let at = prefs.at.filter(|at| still_on_screen(&window, *at)).unwrap_or_else(|| first_place(&window, prefs.width()));
            let relay = bundled_relay()
                .ok_or_else(|| "The relay is missing beside Kuchu. Reinstall Kuchu.".to_string())
                .and_then(|from| {
                    let dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("bin");
                    hooks::place_relay(&from, &dir).map_err(|e| format!("Kuchu could not put its relay in place: {e}"))
                });
            let kuchu = Arc::new(Kuchu {
                prefs: Mutex::new(prefs),
                prefs_path,
                layout: Layout { pet: Mutex::new(at), wide: false.into(), right: true.into(), rects: Mutex::new(Vec::new()), moved: Mutex::new(None) },
                hub: hub.clone(),
                relay: Mutex::new(relay),
            });
            app.manage(kuchu.clone());

            // Clicks go through until the pointer is on the pet or a card (see `follow_pointer`).
            let _ = window.set_ignore_cursor_events(true);
            kuchu.place(&window);
            kuchu.settle(&window);
            let _ = window.show();
            {
                let kuchu = kuchu.clone();
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Moved(at) = event {
                        let scale = w.scale_factor().unwrap_or(1.0);
                        *kuchu.layout.pet.lock().unwrap() = kuchu.pet_from_window(*at, scale);
                        *kuchu.layout.moved.lock().unwrap() = Some(Instant::now());
                    }
                });
            }
            tray(app.handle())?;
            follow_pointer(app.handle().clone(), window, kuchu.clone());

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(why) = hub::serve(hub).await {
                    eprintln!("{why}");
                    app_handle.exit(0);
                }
            });
            let hub = kuchu.hub.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    hub.expire();
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Kuchu could not start");
}

/// Reads the pointer a few times a second while the pet shows: tells the page where it is (for the eyes and for
/// see-through), lets the window take the mouse only over the pet and its cards, and settles a drag once it stops.
/// Polled rather than taken from the window's own mouse events, which stop as soon as clicks go through, and which a
/// window left by a fast pointer never hears about.
fn follow_pointer(app: AppHandle, window: WebviewWindow, kuchu: Arc<Kuchu>) {
    std::thread::spawn(move || {
        let mut last: Option<Pointer> = None;
        let mut taking = false;
        loop {
            std::thread::sleep(POLL);
            let settled = kuchu.layout.moved.lock().unwrap().is_some_and(|at| at.elapsed() >= SETTLE);
            if settled {
                *kuchu.layout.moved.lock().unwrap() = None;
                kuchu.settle(&window);
            }
            if !window.is_visible().unwrap_or(false) {
                continue;
            }
            let (Ok(cursor), Ok(at), Ok(scale)) = (app.cursor_position(), window.outer_position(), window.scale_factor()) else {
                continue;
            };
            let x = (cursor.x - at.x as f64) / scale;
            let y = (cursor.y - at.y as f64) / scale;
            let rects = kuchu.layout.rects.lock().unwrap().clone();
            let pet = rects.first().copied().unwrap_or_else(|| kuchu.pet_in_window());
            let over = pet.holds(x, y);
            let take = over || rects.iter().skip(1).any(|r| r.holds(x, y));
            let now = Pointer { dx: (x - pet.x - pet.w / 2.0).round(), dy: (y - pet.y - pet.h / 2.0).round(), over };
            if take != taking {
                taking = take;
                let _ = window.set_ignore_cursor_events(!take);
            }
            if last.as_ref() != Some(&now) {
                let _ = window.emit("kuchu://pointer", now.clone());
                last = Some(now);
            }
        }
    });
}

fn kuchu(app: &AppHandle) -> Arc<Kuchu> {
    app.state::<Arc<Kuchu>>().inner().clone()
}

/// The sessions list: beside the pet, above it if there is room. It hides when it loses the focus.
fn toggle_sessions(app: &AppHandle) {
    if let Some(list) = app.get_webview_window("sessions") {
        if list.is_visible().unwrap_or(false) {
            let _ = list.hide();
            return;
        }
        place_sessions(app, &list);
        let _ = list.show();
        let _ = list.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, "sessions", WebviewUrl::App("index.html".into()))
        .title("Kuchu")
        .inner_size(SESSIONS_SIZE.0, SESSIONS_SIZE.1)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(true)
        .visible(false)
        .build();
    if let Ok(list) = built {
        // Hidden when it loses the focus, once it has had it: a window being made reports "not focused" first.
        let hide = list.clone();
        let had_focus = Arc::new(AtomicBool::new(false));
        list.on_window_event(move |event| match event {
            WindowEvent::Focused(true) => had_focus.store(true, Ordering::Relaxed),
            WindowEvent::Focused(false) if had_focus.swap(false, Ordering::Relaxed) => {
                let _ = hide.hide();
            }
            _ => {}
        });
        place_sessions(app, &list);
        let _ = list.show();
        let _ = list.set_focus();
    }
}

fn place_sessions(app: &AppHandle, list: &WebviewWindow) {
    let Some(pet) = app.get_webview_window("pet") else { return };
    let k = kuchu(app);
    let scale = pet.scale_factor().unwrap_or(1.0);
    let (px, py) = *k.layout.pet.lock().unwrap();
    let (bw, _) = box_of(k.width());
    let (w, h) = (SESSIONS_SIZE.0 * scale, SESSIONS_SIZE.1 * scale);
    let Ok(Some(monitor)) = pet.monitor_from_point(px as f64, py as f64) else { return };
    let area = monitor.work_area();
    let (ax, ay) = (area.position.x as f64, area.position.y as f64);
    let (aw, ah) = (area.size.width as f64, area.size.height as f64);
    let centre = px as f64 + bw * scale / 2.0;
    let x = (centre - w / 2.0).clamp(ax + 8.0, ax + aw - w - 8.0);
    let y = (py as f64 - h + 8.0 * scale).clamp(ay + 8.0, ay + ah - h - 8.0);
    let _ = list.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
}

fn show_settings(app: &AppHandle) {
    if let Some(settings) = app.get_webview_window("settings") {
        let _ = settings.show();
        let _ = settings.unminimize();
        let _ = settings.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
        .title("Kuchu Settings")
        .inner_size(680.0, 760.0)
        .min_inner_size(520.0, 480.0)
        .center()
        .build();
}

#[tauri::command]
fn view(app: AppHandle) -> View {
    kuchu(&app).hub.view()
}

#[tauri::command]
fn get_prefs(app: AppHandle) -> Prefs {
    kuchu(&app).prefs.lock().unwrap().clone()
}

#[tauri::command]
fn set_prefs(app: AppHandle, patch: serde_json::Value) -> Prefs {
    let k = kuchu(&app);
    let next = k.prefs.lock().unwrap().merged(&patch);
    k.set_prefs(&app, next)
}

/// A click on the pet: what was done is seen, and the list or Settings opens, as Settings says. (Commands that make
/// windows are async: one made from a plain command can lock up on Windows.)
#[tauri::command]
async fn pet_clicked(app: AppHandle) {
    let k = kuchu(&app);
    k.hub.seen();
    let click = k.prefs.lock().unwrap().click.clone();
    if click == "settings" {
        show_settings(&app);
    } else {
        toggle_sessions(&app);
    }
}

#[tauri::command]
fn forget(app: AppHandle, session: String) {
    kuchu(&app).hub.forget(&session);
}

#[tauri::command]
fn decide(app: AppHandle, id: u64, choice: Choice) {
    kuchu(&app).hub.decide(id, choice);
}

#[tauri::command]
fn approval_shown(app: AppHandle, id: u64) {
    kuchu(&app).hub.shown(id);
}

/// The page shows a card (or stops): the window grows beside and above the pet, which stays where it is.
#[tauri::command]
fn set_wide(app: AppHandle, wide: bool) {
    let k = kuchu(&app);
    if k.layout.wide.swap(wide, Ordering::Relaxed) == wide {
        return;
    }
    if let Some(window) = app.get_webview_window("pet") {
        k.place(&window);
    }
}

/// What takes the mouse in the pet's window, as the page laid it out: the pet first, then its cards.
#[tauri::command]
fn set_rects(app: AppHandle, rects: Vec<Rect>) {
    *kuchu(&app).layout.rects.lock().unwrap() = rects;
}

#[tauri::command]
fn is_right(app: AppHandle) -> bool {
    kuchu(&app).layout.right.load(Ordering::Relaxed)
}

fn relay_path(app: &AppHandle) -> Result<PathBuf, String> {
    kuchu(app).relay.lock().unwrap().clone()
}

#[tauri::command]
fn hooks_status(app: AppHandle) -> hooks::Report {
    match relay_path(&app) {
        Ok(relay) => hooks::status(&hooks::settings_path(), &relay),
        Err(why) => hooks::Report { state: "unreadable".into(), why: Some(why), path: hooks::settings_path().display().to_string(), disabled: false },
    }
}

#[tauri::command]
fn hooks_plan(app: AppHandle, install: bool) -> Result<hooks::Plan, String> {
    hooks::plan(&hooks::settings_path(), &relay_path(&app)?, install)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Applied {
    backup: Option<String>,
    report: hooks::Report,
}

#[tauri::command]
fn hooks_apply(app: AppHandle, install: bool, fingerprint: String) -> Result<Applied, String> {
    let relay = relay_path(&app)?;
    let path = hooks::settings_path();
    let backup = hooks::apply(&path, &relay, install, &fingerprint)?;
    let report = hooks::status(&path, &relay);
    let _ = app.emit("kuchu://hooks", report.clone());
    Ok(Applied { backup, report })
}

#[tauri::command]
async fn open_settings(app: AppHandle) {
    show_settings(&app);
}

#[tauri::command]
async fn open_sessions(app: AppHandle) {
    toggle_sessions(&app);
}

/// The tray: always a way back to the pet, the sessions, Settings, sounds, quitting.
fn tray(app: &AppHandle) -> tauri::Result<()> {
    let shown = CheckMenuItem::with_id(app, "shown", "Show Kuchu", true, true, None::<&str>)?;
    let sessions = MenuItem::with_id(app, "sessions", "Sessions", true, Some("Ctrl+Alt+K"))?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let sounds = CheckMenuItem::with_id(app, "sounds", "Sounds", true, kuchu(app).prefs.lock().unwrap().sounds, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Kuchu", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&shown, &sessions, &settings, &separator, &sounds, &separator2, &quit])?;

    let mut builder = TrayIconBuilder::with_id("kuchu").tooltip("Kuchu").menu(&menu).show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let shown_item = shown.clone();
    let sounds_item = sounds.clone();
    builder
        .on_menu_event(move |app, event| {
            let Some(window) = app.get_webview_window("pet") else { return };
            match event.id().as_ref() {
                "shown" => {
                    let show = !window.is_visible().unwrap_or(true);
                    let _ = if show { window.show() } else { window.hide() };
                    let _ = shown_item.set_checked(show);
                }
                "sessions" => toggle_sessions(app),
                "settings" => show_settings(app),
                "sounds" => {
                    let k = kuchu(app);
                    let mut next = k.prefs.lock().unwrap().clone();
                    next.sounds = !next.sounds;
                    let _ = sounds_item.set_checked(next.sounds);
                    k.set_prefs(app, next);
                }
                "quit" => app.exit(0),
                _ => {}
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
