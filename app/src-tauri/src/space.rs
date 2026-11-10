//! On macOS, a Space of Kuchu's own for the pet. Going from one Space to the next, macOS takes every window that shows
//! on all of them off the screen for as long as the slide lasts, so the pet blinked out at each change. Kuchu's own
//! Space is shown above the person's Spaces and is not part of the slide: a window moved into it stays on the screen
//! throughout.
//!
//! macOS has no public way to do this, so it is done with the window server's own functions (the SkyLight framework).
//! Each is looked up by name before any is called. If one is missing, or the Space cannot be made, nothing is moved:
//! the pet stays on every Space as `join_every_space` (lib.rs) put it, and blinks at a change of Space, as before.
//!
//! Now and then macOS puts the window back on the person's Spaces by itself, and the pet blinks again. So while the pet
//! shows, Kuchu asks about once a second where its window is (`keep`), and moves it back if it has to.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2_foundation::{NSArray, NSNumber};

/// The window server's framework, at the same path on every Mac.
const SKYLIGHT: &CStr = c"/System/Library/PrivateFrameworks/SkyLight.framework/Versions/A/SkyLight";
/// Kuchu's Space shown at the level of the person's own: the pet above their windows (it keeps its window level within
/// the Space), and below what the system shows over everything, such as the lock screen.
const LEVEL: c_int = 0;
/// Which Spaces a window leaves as it joins Kuchu's: every other one.
const FROM_EVERY_OTHER: c_int = 7;
/// Which Spaces a window is asked about: the person's own, of every kind (Kuchu's is not among them).
const THE_PERSONS: c_int = 7;
const RTLD_LAZY: c_int = 1;

extern "C" {
    fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

type Connection = unsafe extern "C" fn() -> c_int;
type Create = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type SetLevel = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type Show = unsafe extern "C" fn(c_int, *const c_void) -> c_int;
type AddWindows = unsafe extern "C" fn(c_int, c_int, *const c_void, c_int) -> c_int;
type SpacesOf = unsafe extern "C" fn(c_int, c_int, *const c_void) -> *mut c_void;

struct Own {
    connection: c_int,
    space: c_int,
    add: AddWindows,
    /// To ask which of the person's Spaces a window is on. Without it, nothing is checked (`keep`).
    spaces_of: Option<SpacesOf>,
}

/// Moves the window numbered `window` (its `windowNumber`) into Kuchu's Space. Asked again for a window already there,
/// nothing changes. Does nothing where Kuchu's Space cannot be had.
pub fn take(window: isize) {
    let Some(own) = own() else { return };
    let Ok(window) = c_int::try_from(window) else { return };
    if window <= 0 {
        return;
    }
    let windows = one(window);
    // SAFETY: `add` was found under its own name in the system's framework and takes these arguments; the list lives
    // until the call returns.
    unsafe { (own.add)(own.connection, own.space, Retained::as_ptr(&windows).cast(), FROM_EVERY_OTHER) };
}

/// Moves the window numbered `window` back into Kuchu's Space if macOS has put it on the person's Spaces since. One
/// question to the window server, and nothing more while the window stays where it is.
pub fn keep(window: isize) {
    let Some(own) = own() else { return };
    let Some(spaces_of) = own.spaces_of else { return };
    let Ok(number) = c_int::try_from(window) else { return };
    if number <= 0 {
        return;
    }
    let windows = one(number);
    // SAFETY: `spaces_of` was found under its own name and takes these arguments; the list it returns is a copy of
    // Kuchu's own, released when `spaces` is dropped.
    let spaces = unsafe {
        let list = spaces_of(own.connection, THE_PERSONS, Retained::as_ptr(&windows).cast());
        Retained::from_raw(list.cast::<NSArray<NSNumber>>())
    };
    if spaces.is_some_and(|spaces| spaces.count() > 0) {
        take(window);
    }
}

/// Kuchu's Space, made the first time it is asked for. `None` when the window server lacks one of the functions or
/// will not make the Space, and then for as long as Kuchu runs.
fn own() -> Option<&'static Own> {
    static OWN: OnceLock<Option<Own>> = OnceLock::new();
    OWN.get_or_init(|| {
        // SAFETY: the framework is the system's own; a function is called only once found under its name, and with
        // the arguments it takes. (What these calls return is not relied on: only the Space's number is.)
        unsafe {
            let lib = dlopen(SKYLIGHT.as_ptr(), RTLD_LAZY);
            if lib.is_null() {
                return None;
            }
            let find = |name: &CStr| {
                let f = dlsym(lib, name.as_ptr());
                (!f.is_null()).then_some(f)
            };
            let connection: Connection = std::mem::transmute(find(c"SLSMainConnectionID")?);
            let create: Create = std::mem::transmute(find(c"SLSSpaceCreate")?);
            let set_level: SetLevel = std::mem::transmute(find(c"SLSSpaceSetAbsoluteLevel")?);
            let show: Show = std::mem::transmute(find(c"SLSShowSpaces")?);
            let add: AddWindows = std::mem::transmute(find(c"SLSSpaceAddWindowsAndRemoveFromSpaces")?);
            let spaces_of = find(c"SLSCopySpacesForWindows").map(|f| std::mem::transmute::<*mut c_void, SpacesOf>(f));
            let connection = connection();
            let space = create(connection, 1, 0);
            if space <= 0 {
                return None;
            }
            set_level(connection, space, LEVEL);
            let spaces = one(space);
            show(connection, Retained::as_ptr(&spaces).cast());
            Some(Own { connection, space, add, spaces_of })
        }
    })
    .as_ref()
}

/// A list of one number, as the window server takes Spaces and windows.
fn one(n: c_int) -> Retained<NSArray<NSNumber>> {
    NSArray::from_retained_slice(&[NSNumber::new_i32(n)])
}
