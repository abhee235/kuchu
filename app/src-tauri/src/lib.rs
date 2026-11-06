//! Kuchu's core: the pet's window (placed in the screen's work area, never taking the focus, letting clicks through
//! everywhere but the pet and its cards), the pointer read for the pet's eyes, the sessions list and Settings windows,
//! the tray and the shortcut. The hub (`hub.rs`) takes the relay's events in; the pages in `app/src` draw it all.

mod alive;
mod chat;
mod hooks;
mod hub;
mod install;
mod locks;
mod prefs;
mod sessions;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{Builder as ShortcutBuilder, ShortcutState};

use chat::{Chat, Chats};
use locks::Hold;
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
/// A stroll's pace (logical px a second, as the page expects), each step's time, and the pause before coming back.
const STROLL_SPEED: f64 = 70.0;
const FRAME: Duration = Duration::from_millis(16);
const STROLL_PAUSE: Duration = Duration::from_millis(900);
/// A busy session silent this long is checked with Claude Code, at most this often.
const SILENT_MS: u64 = 15_000;
const CHECK_EVERY: Duration = Duration::from_secs(15);
/// How long after losing the focus the sessions list checks it really has, before hiding.
const BLUR_SETTLE: Duration = Duration::from_millis(150);
/// The shortcut that opens the sessions list from anywhere.
const SHORTCUT: &str = "ctrl+alt+k";
const SESSIONS_SIZE: (f64, f64) = (400.0, 560.0);

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

/// What the page needs from the pointer: where it is from the pet's centre, and whether it is on the pet or a card.
#[derive(Clone, Serialize, PartialEq)]
struct Pointer {
    dx: f64,
    dy: f64,
    over: bool,
    card: bool,
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
    /// The pointer is on the pet or one of its cards (the window takes the mouse).
    taking: AtomicBool,
    /// Which way the pet is walking (carried or strolling): -1 left, 1 right, 0 standing.
    walking: AtomicI32,
    /// Strolls started; one stops when this moves on (a newer one, or the pet taken in hand).
    strolls: AtomicU64,
}

struct Kuchu {
    prefs: Mutex<Prefs>,
    prefs_path: PathBuf,
    layout: Layout,
    hub: Arc<Hub>,
    /// The relay where the hooks point (copied there at start).
    relay: Mutex<Result<PathBuf, String>>,
    /// Conversations started from Kuchu.
    chats: Arc<Chats>,
    /// A folder is being picked: the sessions list stays while the dialog has the focus.
    picking: AtomicBool,
    /// The conversation the sessions list should open next (a click on its card).
    open: Mutex<Option<String>>,
}

impl Kuchu {
    fn width(&self) -> u32 {
        self.prefs.hold().width()
    }

    /// The window around the pet: its own box, or with room for a card beside and above it.
    fn window_rect(&self, scale: f64) -> (i32, i32, f64, f64) {
        let (bw, bh) = box_of(self.width());
        let (px, py) = *self.layout.pet.hold();
        if !self.layout.wide.load(Ordering::Relaxed) {
            return (px, py, bw, bh);
        }
        let more_x = (CARD_W * scale).round() as i32;
        let more_y = (CARD_H * scale).round() as i32;
        let x = if self.layout.right.load(Ordering::Relaxed) { px - more_x } else { px };
        (x, py - more_y, bw + CARD_W, bh + CARD_H)
    }

    /// The pet and the room its cards take beside and above it (physical px), whether a card shows or not: the
    /// sessions list keeps out of it.
    fn zone(&self, scale: f64) -> (f64, f64, f64, f64) {
        let (bw, bh) = box_of(self.width());
        let (px, py) = *self.layout.pet.hold();
        let x = if self.layout.right.load(Ordering::Relaxed) { px as f64 - CARD_W * scale } else { px as f64 };
        (x, py as f64 - CARD_H * scale, (bw + CARD_W) * scale, (bh + CARD_H) * scale)
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

    /// Moves the window to where the pet is, its size as it is.
    fn move_window(&self, window: &WebviewWindow) {
        let scale = window.scale_factor().unwrap_or(1.0);
        let (x, y, _, _) = self.window_rect(scale);
        let _ = window.set_position(PhysicalPosition::new(x, y));
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
        let (px, py) = *self.layout.pet.hold();
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
        // The pet moved with the sessions list open: the list moves out of its way.
        let app = window.app_handle();
        if let Some(list) = app.get_webview_window("sessions").filter(|l| l.is_visible().unwrap_or(false)) {
            place_sessions(app, &list);
        }
        let mut prefs = self.prefs.hold();
        if prefs.at != Some((px, py)) {
            prefs.at = Some((px, py));
            prefs.save(&self.prefs_path);
        }
    }

    fn set_prefs(&self, app: &AppHandle, next: Prefs) -> Prefs {
        let before = self.prefs.hold().clone();
        *self.prefs.hold() = next.clone();
        next.save(&self.prefs_path);
        self.hub.approvals_on.store(next.approvals, Ordering::Relaxed);
        if before.width() != next.width() {
            if let Some(window) = app.get_webview_window("pet") {
                // The pet's bottom corner nearest the screen's edge stays where it is.
                let scale = window.scale_factor().unwrap_or(1.0);
                let ((w0, h0), (w1, h1)) = (box_of(before.width()), box_of(next.width()));
                let mut pet = self.layout.pet.hold();
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

/// Kuchu was started again while it runs, and the second copy handed its arguments over and went: `--quit` ends this
/// one (`kuchu quit`, `kuchu uninstall`), `--settings` shows Settings (`kuchu setup`), anything else (a second
/// double-click) shows the pet and the sessions.
fn again(app: &AppHandle, args: &[String]) {
    match handed(args) {
        Handed::Quit => app.exit(0),
        Handed::Settings => show_settings(app),
        Handed::Show => {
            if let Some(pet) = app.get_webview_window("pet") {
                let _ = pet.show();
            }
            if !app.get_webview_window("sessions").is_some_and(|w| w.is_visible().unwrap_or(false)) {
                toggle_sessions(app);
            }
        }
    }
}

/// What a second Kuchu's arguments ask of the running one. Never more than these: the hand-off is a window message
/// any program in the session can send, so nothing it can say decides, writes, or starts Claude Code.
#[derive(Debug, PartialEq)]
enum Handed {
    Quit,
    Settings,
    Show,
}

fn handed(args: &[String]) -> Handed {
    if args.iter().any(|a| a == "--quit") {
        Handed::Quit
    } else if args.iter().any(|a| a == "--settings") {
        Handed::Settings
    } else {
        Handed::Show
    }
}

pub fn run() {
    // `kuchu setup`, `status`, `quit`, `uninstall`: done without starting Kuchu.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = install::command(&args) {
        std::process::exit(code);
    }
    let builder = tauri::Builder::default();
    // First of all, so that a second Kuchu hands its arguments over before it does anything else.
    #[cfg(windows)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| again(app, &args)));
    builder
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
        .plugin(tauri_plugin_dialog::init())
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
            stroll,
            stop_stroll,
            follow,
            hooks_status,
            hooks_plan,
            hooks_apply,
            start_at_sign_in,
            set_start_at_sign_in,
            open_settings,
            open_sessions,
            take_open,
            pick_folder,
            chats,
            chat_start,
            chat_send,
            chat_stop,
            chat_terminal,
            report_answer,
        ])
        .setup(move |app| {
            // Asked to quit with no other Kuchu to hand it to: nothing to do.
            if args.iter().any(|a| a == "--quit") {
                std::process::exit(0);
            }
            let config = app.path().app_config_dir()?;
            let prefs_path = prefs::path_in(config);
            // Kuchu's very first start (installed, or unzipped and opened): its settings are written once, and
            // Settings opens, where it is connected to Claude Code.
            let first = !prefs_path.exists();
            let prefs = Prefs::load(&prefs_path);
            if first {
                prefs.save(&prefs_path);
            }
            let hub = Hub::new(app.handle().clone(), prefs.approvals);
            let window = app.get_webview_window("pet").expect("the pet's window is in tauri.conf.json");
            let at = prefs.at.filter(|at| still_on_screen(&window, *at)).unwrap_or_else(|| first_place(&window, prefs.width()));
            let relay = install::start_relay();
            install::restart_after_update();
            let handle = app.handle().clone();
            let chats = Chats::new(app.handle().clone(), hub.clone(), Box::new(move || own_settings(&handle)));
            let kuchu = Arc::new(Kuchu {
                prefs: Mutex::new(prefs),
                prefs_path,
                layout: Layout { pet: Mutex::new(at), wide: false.into(), right: true.into(), rects: Mutex::new(Vec::new()), moved: Mutex::new(None), taking: false.into(), walking: 0.into(), strolls: 0.into() },
                hub: hub.clone(),
                relay: Mutex::new(relay),
                chats,
                picking: false.into(),
                open: Mutex::new(None),
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
                let seen = Mutex::new(*kuchu.layout.pet.hold());
                window.on_window_event(move |event| {
                    if let WindowEvent::Moved(at) = event {
                        let scale = w.scale_factor().unwrap_or(1.0);
                        let now = kuchu.pet_from_window(*at, scale);
                        *kuchu.layout.pet.hold() = now;
                        *kuchu.layout.moved.hold() = Some(Instant::now());
                        // The pet itself moved (dragged, or strolling), not only its window around it: it walks that
                        // way until it settles.
                        let before = std::mem::replace(&mut *seen.hold(), now);
                        let (dx, dy) = (now.0 - before.0, now.1 - before.1);
                        if dx != 0 || dy.abs() >= 2 {
                            let was = kuchu.layout.walking.load(Ordering::Relaxed);
                            let dir = if dx != 0 { dx.signum() } else if was != 0 { was } else { 1 };
                            if kuchu.layout.walking.swap(dir, Ordering::Relaxed) != dir {
                                let _ = w.emit("kuchu://walk", dir);
                            }
                        }
                    }
                });
            }
            tray(app.handle())?;
            follow_pointer(app.handle().clone(), window, kuchu.clone());

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(why) = hub::serve(hub).await {
                    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
                    // Said in a box: a windowed app's stderr goes nowhere.
                    app_handle.dialog().message(why).title("Kuchu").kind(MessageDialogKind::Error).blocking_show();
                    app_handle.exit(0);
                }
            });
            // Sessions busy but silent a while: does Claude Code still run them? (A closed terminal, or Esc, says
            // nothing.) Asked only when there is such a session.
            let hub = kuchu.hub.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(CHECK_EVERY).await;
                    let asked = hub.silent(SILENT_MS);
                    if asked.is_empty() {
                        continue;
                    }
                    if let Some(running) = alive::running().await {
                        hub.check_running(&asked, &running, SILENT_MS);
                    }
                }
            });
            let (hub, chats) = (kuchu.hub.clone(), kuchu.chats.clone());
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    hub.expire();
                    chats.expire();
                }
            });
            // Started by `kuchu setup`, or for the first time: Settings open, where Kuchu is connected to Claude Code.
            if first || args.iter().any(|a| a == "--settings") {
                show_settings(app.handle());
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Kuchu could not start")
        .run(|_app, _event| {
            if let tauri::RunEvent::Exit = _event {
                install::tidy_relay();
            }
            // Opened again while it runs (the system keeps an app to one copy on macOS).
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                again(_app, &[]);
            }
        });
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
            let settled = kuchu.layout.moved.hold().is_some_and(|at| at.elapsed() >= SETTLE);
            if settled {
                *kuchu.layout.moved.hold() = None;
                kuchu.settle(&window);
                if kuchu.layout.walking.swap(0, Ordering::Relaxed) != 0 {
                    let _ = window.emit("kuchu://walk", 0);
                }
            }
            if !window.is_visible().unwrap_or(false) {
                continue;
            }
            let (Ok(cursor), Ok(at), Ok(scale)) = (app.cursor_position(), window.outer_position(), window.scale_factor()) else {
                continue;
            };
            let x = (cursor.x - at.x as f64) / scale;
            let y = (cursor.y - at.y as f64) / scale;
            let rects = kuchu.layout.rects.hold().clone();
            let pet = rects.first().copied().unwrap_or_else(|| kuchu.pet_in_window());
            let over = pet.holds(x, y);
            let card = rects.iter().skip(1).any(|r| r.holds(x, y));
            let take = over || card;
            let now = Pointer { dx: (x - pet.x - pet.w / 2.0).round(), dy: (y - pet.y - pet.h / 2.0).round(), over, card };
            if take != taking {
                taking = take;
                kuchu.layout.taking.store(take, Ordering::Relaxed);
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

/// The sessions list, near the pet but never over it or its cards. It hides when it loses the focus.
fn toggle_sessions(app: &AppHandle) {
    if let Some(list) = app.get_webview_window("sessions") {
        if list.is_visible().unwrap_or(false) {
            let _ = list.hide();
            return;
        }
        place_sessions(app, &list);
        let _ = list.show();
        let _ = list.set_focus();
        raise_pet(app);
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
        // Hidden when it loses the focus, but not to the folder dialog it opened, nor to a click on the pet's cards
        // (allowing a request mid-conversation). A window being made, its page still loading, reports losing the focus
        // when it has not: it hides only if, a moment later, it is still without it.
        let hide = list.clone();
        let handle = app.clone();
        list.on_window_event(move |event| {
            if let WindowEvent::Focused(false) = event {
                let (hide, handle) = (hide.clone(), handle.clone());
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(BLUR_SETTLE).await;
                    let k = kuchu(&handle);
                    let ours = k.picking.load(Ordering::Relaxed) || k.layout.taking.load(Ordering::Relaxed);
                    if !ours && !hide.is_focused().unwrap_or(true) {
                        let _ = hide.hide();
                    }
                });
            }
        });
        place_sessions(app, &list);
        let _ = list.show();
        let _ = list.set_focus();
        raise_pet(app);
    }
}

/// Where the sessions list goes: out of the pet's zone (the pet and the room its cards take), inside the work area.
/// Above the zone if it fits there, else below it, else beside it (away from the screen's edge first).
fn place_sessions(app: &AppHandle, list: &WebviewWindow) {
    let Some(pet) = app.get_webview_window("pet") else { return };
    let k = kuchu(app);
    let scale = pet.scale_factor().unwrap_or(1.0);
    let (zx, zy, zw, zh) = k.zone(scale);
    let Ok(Some(monitor)) = pet.monitor_from_point(zx + zw / 2.0, zy + zh) else { return };
    let area = monitor.work_area();
    let area = (area.position.x as f64, area.position.y as f64, area.size.width as f64, area.size.height as f64);
    let (px, _) = *k.layout.pet.hold();
    let centre = px as f64 + box_of(k.width()).0 * scale / 2.0;
    let size = (SESSIONS_SIZE.0 * scale, SESSIONS_SIZE.1 * scale);
    let (x, y) = out_of_the_way((zx, zy, zw, zh), centre, size, area, k.layout.right.load(Ordering::Relaxed), 8.0 * scale);
    let _ = list.set_position(PhysicalPosition::new(x.round() as i32, y.round() as i32));
}

/// A place for a window of `size` in `area` that does not cover `zone`: above it, else below it (centred on `centre`),
/// else on its inner side, else its outer side (bottoms lined up); if none fits, the inner side, kept on the screen.
fn out_of_the_way(zone: (f64, f64, f64, f64), centre: f64, size: (f64, f64), area: (f64, f64, f64, f64), right: bool, gap: f64) -> (f64, f64) {
    let (zx, zy, zw, zh) = zone;
    let (w, h) = size;
    let (ax, ay, aw, ah) = area;
    let clamp_x = |x: f64| x.clamp(ax + gap, (ax + aw - w - gap).max(ax + gap));
    let clamp_y = |y: f64| y.clamp(ay + gap, (ay + ah - h - gap).max(ay + gap));
    let fits = |(x, y): (f64, f64)| x >= ax + gap - 0.5 && y >= ay + gap - 0.5 && x + w <= ax + aw - gap + 0.5 && y + h <= ay + ah - gap + 0.5;
    let x = clamp_x(centre - w / 2.0);
    let level = clamp_y(zy + zh - h);
    let (inner, outer) = if right { (zx - gap - w, zx + zw + gap) } else { (zx + zw + gap, zx - gap - w) };
    [(x, zy - gap - h), (x, zy + zh + gap), (inner, level), (outer, level)].into_iter().find(|&p| fits(p)).unwrap_or((clamp_x(inner), level))
}

/// Puts the pet's window back on top of the other windows that stay on top (the sessions list too), without taking
/// the focus: its cards, a permission request above all, are never covered.
fn raise_pet(app: &AppHandle) {
    if let Some(pet) = app.get_webview_window("pet") {
        // Set again only when it changes, so it is taken off and put back.
        let _ = pet.set_always_on_top(false);
        let _ = pet.set_always_on_top(true);
    }
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
        .inner_size(880.0, 680.0)
        .min_inner_size(700.0, 480.0)
        .center()
        .build();
}

#[tauri::command]
fn view(app: AppHandle) -> View {
    kuchu(&app).hub.view()
}

#[tauri::command]
fn get_prefs(app: AppHandle) -> Prefs {
    kuchu(&app).prefs.hold().clone()
}

#[tauri::command]
fn set_prefs(app: AppHandle, patch: serde_json::Value) -> Prefs {
    let k = kuchu(&app);
    let next = k.prefs.hold().merged(&patch);
    k.set_prefs(&app, next)
}

/// A click on the pet: what was done is seen, and the list or Settings opens, as Settings says. (Commands that make
/// windows are async: one made from a plain command can lock up on Windows.)
#[tauri::command]
async fn pet_clicked(app: AppHandle) {
    let k = kuchu(&app);
    k.hub.seen();
    let click = k.prefs.hold().click.clone();
    if click == "settings" {
        show_settings(&app);
    } else {
        toggle_sessions(&app);
    }
}

/// The card switched to `session` (it stays there while that session works), or back to the latest.
#[tauri::command]
fn follow(app: AppHandle, session: Option<String>) {
    kuchu(&app).hub.follow(session);
}

#[tauri::command]
fn forget(app: AppHandle, session: String) {
    let k = kuchu(&app);
    k.hub.forget(&session);
    k.chats.forget(&session);
}

/// A click on a permission card. The sessions list, if open, has the focus back (the click took it), so it still hides
/// when you go elsewhere.
#[tauri::command]
fn decide(app: AppHandle, id: u64, choice: Choice) {
    kuchu(&app).hub.decide(id, choice);
    if let Some(list) = app.get_webview_window("sessions").filter(|l| l.is_visible().unwrap_or(false)) {
        let _ = list.set_focus();
    }
}

#[tauri::command]
fn approval_shown(app: AppHandle, id: u64) {
    kuchu(&app).hub.shown(id);
    raise_pet(&app);
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
    if wide {
        raise_pet(&app);
    }
}

/// What takes the mouse in the pet's window, as the page laid it out: the pet first, then its cards.
#[tauri::command]
fn set_rects(app: AppHandle, rects: Vec<Rect>) {
    *kuchu(&app).layout.rects.hold() = rects;
}

/// A short stroll and back, when Kuchu has nothing to do: it walks `dx` logical px along its screen (the page sends
/// it toward the middle), stops a moment, and comes home. Returns how far it goes; 0 when it stays (a card shows, the
/// pointer is on it, it is being moved, or there is no room).
#[tauri::command]
fn stroll(app: AppHandle, dx: f64) -> f64 {
    let k = kuchu(&app);
    let Some(window) = app.get_webview_window("pet") else { return 0.0 };
    let busy = k.layout.wide.load(Ordering::Relaxed) || k.layout.taking.load(Ordering::Relaxed) || k.layout.moved.hold().is_some();
    if busy {
        return 0.0;
    }
    let scale = window.scale_factor().unwrap_or(1.0);
    let home = *k.layout.pet.hold();
    let (bw, _) = box_of(k.width());
    let Ok(Some(monitor)) = window.monitor_from_point(home.0 as f64 + bw * scale / 2.0, home.1 as f64) else { return 0.0 };
    let area = monitor.work_area();
    let (lo, hi) = (area.position.x as f64, area.position.x as f64 + area.size.width as f64 - bw * scale);
    let to = (home.0 as f64 + dx * scale).clamp(lo, hi.max(lo)).round() as i32;
    let far = to - home.0;
    if (far.abs() as f64) < 4.0 * scale {
        return 0.0;
    }
    let nth = k.layout.strolls.fetch_add(1, Ordering::Relaxed) + 1;
    let step = (STROLL_SPEED * scale * FRAME.as_secs_f64()).max(1.0);
    std::thread::spawn(move || {
        let walk = |from: i32, to: i32| {
            let n = ((to - from).abs() as f64 / step).ceil().max(1.0) as i32;
            for i in 1..=n {
                std::thread::sleep(FRAME);
                if k.layout.strolls.load(Ordering::Relaxed) != nth {
                    return false;
                }
                k.layout.pet.hold().0 = from + ((to - from) as f64 * i as f64 / n as f64).round() as i32;
                k.move_window(&window);
            }
            true
        };
        if walk(home.0, to) {
            std::thread::sleep(STROLL_PAUSE);
            if k.layout.strolls.load(Ordering::Relaxed) == nth {
                walk(to, home.0);
            }
        }
    });
    far as f64 / scale
}

/// The pet taken in hand: a stroll under way stops where it is.
#[tauri::command]
fn stop_stroll(app: AppHandle) {
    kuchu(&app).layout.strolls.fetch_add(1, Ordering::Relaxed);
}

#[tauri::command]
fn is_right(app: AppHandle) -> bool {
    kuchu(&app).layout.right.load(Ordering::Relaxed)
}

fn relay_path(app: &AppHandle) -> Result<PathBuf, String> {
    kuchu(app).relay.hold().clone()
}

/// Whether Kuchu starts when the person signs in.
#[tauri::command]
fn start_at_sign_in() -> install::Start {
    install::start_at_sign_in()
}

#[tauri::command]
fn set_start_at_sign_in(on: bool) -> Result<install::Start, String> {
    install::set_start_at_sign_in(on)
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

/// Connecting puts the relay in place first (from the Store it is there only while connected); disconnecting takes it
/// away again, unless one of Kuchu's own conversations may still run it (then on quitting).
#[tauri::command]
fn hooks_apply(app: AppHandle, install: bool, fingerprint: String) -> Result<Applied, String> {
    let relay = relay_path(&app)?;
    if install {
        install::place_relay()?;
    }
    let path = hooks::settings_path();
    let backup = hooks::apply(&path, &relay, install, &fingerprint)?;
    if !install && !kuchu(&app).chats.running() {
        install::tidy_relay();
    }
    let report = hooks::status(&path, &relay);
    let _ = app.emit("kuchu://hooks", report.clone());
    Ok(Applied { backup, report })
}

#[tauri::command]
async fn open_settings(app: AppHandle) {
    show_settings(&app);
}

/// Where an answer from Claude in one of Kuchu's conversations is reported: the Store asks apps that show text an AI
/// wrote for a way to report it from the app.
const REPORT_URL: &str = "https://github.com/abhee235/kuchu/issues/new?title=About%20an%20answer%20from%20Claude";

/// Opens the page for reporting an answer in the person's browser. Always that one page: the page names nothing.
#[tauri::command]
fn report_answer() -> Result<(), String> {
    #[cfg(windows)]
    let opened = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", REPORT_URL]).spawn();
    #[cfg(target_os = "macos")]
    let opened = std::process::Command::new("open").arg(REPORT_URL).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let opened = std::process::Command::new("xdg-open").arg(REPORT_URL).spawn();
    let mut child = opened.map_err(|e| format!("Kuchu could not open your browser: {e}"))?;
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// The sessions list; given a conversation started from Kuchu, the list opens on it (a click on its card).
#[tauri::command]
async fn open_sessions(app: AppHandle, session: Option<String>) {
    let k = kuchu(&app);
    match session.filter(|id| k.chats.has(id)) {
        Some(id) => {
            *k.open.hold() = Some(id);
            let shown = app.get_webview_window("sessions").is_some_and(|w| w.is_visible().unwrap_or(false));
            if !shown {
                toggle_sessions(&app);
            }
            let _ = app.emit("kuchu://open", ());
        }
        None => toggle_sessions(&app),
    }
}

/// The conversation the sessions list should open, once.
#[tauri::command]
fn take_open(app: AppHandle) -> Option<String> {
    kuchu(&app).open.hold().take()
}

/// A folder for a new conversation, from the system's folder dialog.
#[tauri::command]
async fn pick_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let k = kuchu(&app);
    k.picking.store(true, Ordering::Relaxed);
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut dialog = app.dialog().file().set_title("Where should Claude work?");
    let list = app.get_webview_window("sessions");
    if let Some(list) = &list {
        dialog = dialog.set_parent(list);
    }
    dialog.pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let folder = rx.await.ok().flatten();
    k.picking.store(false, Ordering::Relaxed);
    if let Some(list) = &list {
        let _ = list.set_focus();
    }
    folder.and_then(|f| f.into_path().ok()).map(|p| p.display().to_string())
}

#[tauri::command]
fn chats(app: AppHandle) -> Vec<Chat> {
    kuchu(&app).chats.list()
}

/// A new conversation in `cwd`; the folder goes first among the recent ones. Returns its session id.
#[tauri::command]
async fn chat_start(app: AppHandle, cwd: String, text: String) -> Result<String, String> {
    let k = kuchu(&app);
    let id = k.chats.start(&cwd, &text)?;
    let mut next = k.prefs.hold().clone();
    next.used_folder(&cwd);
    k.set_prefs(&app, next);
    Ok(id)
}

#[tauri::command]
async fn chat_send(app: AppHandle, session: String, text: String) -> Result<(), String> {
    kuchu(&app).chats.send(&session, &text)
}

#[tauri::command]
async fn chat_stop(app: AppHandle, session: String) {
    kuchu(&app).chats.stop(&session);
}

#[tauri::command]
async fn chat_terminal(app: AppHandle, session: String) -> Result<(), String> {
    kuchu(&app).chats.hand_off(&session)
}

/// Settings for a session Kuchu starts itself: nothing extra while Claude Code's settings have Kuchu's hooks; until
/// they do, a file with only those hooks, so the session still shows beside the pet and asks for permission there (and
/// the relay put in place for them, since from the Store it is there only while connected).
fn own_settings(app: &AppHandle) -> Option<PathBuf> {
    let relay = relay_path(app).ok()?;
    if hooks::status(&hooks::settings_path(), &relay).state != "missing" {
        return None;
    }
    install::place_relay().ok()?;
    let path = app.path().app_local_data_dir().ok()?.join("hooks.json");
    std::fs::write(&path, serde_json::to_string_pretty(&hooks::own(&relay)).ok()?).ok()?;
    Some(path)
}

/// The tray: always a way back to the pet, the sessions, Settings, sounds, quitting.
fn tray(app: &AppHandle) -> tauri::Result<()> {
    let shown = CheckMenuItem::with_id(app, "shown", "Show Kuchu", true, true, None::<&str>)?;
    let sessions = MenuItem::with_id(app, "sessions", "Sessions", true, Some("Ctrl+Alt+K"))?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let sounds = CheckMenuItem::with_id(app, "sounds", "Sounds", true, kuchu(app).prefs.hold().sounds, None::<&str>)?;
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
                    let mut next = k.prefs.hold().clone();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_kuchu_can_only_ask_to_quit_show_settings_or_show() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(handed(&args(&["--quit"])), Handed::Quit);
        assert_eq!(handed(&args(&["--settings"])), Handed::Settings);
        assert_eq!(handed(&args(&[])), Handed::Show);
        assert_eq!(handed(&args(&["--resume", "x", "--decide", "allow"])), Handed::Show);
        assert_eq!(handed(&args(&["--quit", "--settings"])), Handed::Quit);
    }

    fn apart(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
        a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1
    }

    #[test]
    fn the_sessions_list_keeps_out_of_the_pets_way() {
        let size = (400.0, 560.0);
        let screen = (0.0, 0.0, 2560.0, 1400.0);
        // The pet low on the right (where it starts): the list goes above it and its cards.
        let low = (2000.0, 900.0, 480.0, 410.0);
        let at = out_of_the_way(low, 2400.0, size, screen, true, 8.0);
        assert_eq!(at, (2152.0, 332.0));
        // Near the top: below.
        let high = (2000.0, 20.0, 480.0, 410.0);
        let at = out_of_the_way(high, 2400.0, size, screen, true, 8.0);
        assert_eq!(at, (2152.0, 438.0));
        assert!(apart((at.0, at.1, size.0, size.1), high));
        // A short screen, room neither above nor below: beside it, away from the edge.
        let short = (0.0, 0.0, 2560.0, 800.0);
        let middle = (2000.0, 200.0, 480.0, 410.0);
        let at = out_of_the_way(middle, 2400.0, size, short, true, 8.0);
        assert_eq!(at, (1592.0, 50.0));
        assert!(apart((at.0, at.1, size.0, size.1), middle));
        // On the left half, beside means to its right.
        let left = (40.0, 200.0, 480.0, 410.0);
        let at = out_of_the_way(left, 120.0, size, short, false, 8.0);
        assert_eq!(at, (528.0, 50.0));
    }
}
