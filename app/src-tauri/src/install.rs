//! Kuchu on this account: the folders it keeps things in, starting when the person signs in, and the commands Claude
//! Code runs to install it (the README's Install): `kuchu setup`, `kuchu status`, `kuchu quit` and `kuchu uninstall`.
//!
//! Kuchu is always the person's own app. It starts at sign-in from their account's own entry (the current user's Run
//! key on Windows, a LaunchAgent on macOS), never as a service or anything elevated. Claude Code's settings.json is
//! changed only from Kuchu's Settings, after the person has seen the change: `setup` opens that window, and
//! `uninstall` asks for Kuchu's hooks to be taken out there first.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::hooks;

/// Kuchu's identifier, as in tauri.conf.json: Tauri names Kuchu's folders after it, and so does this module.
pub const IDENTIFIER: &str = "dev.kuchu.app";

/// How long `quit` and `uninstall` wait for a running Kuchu to be gone.
const QUIT_WITHIN: Duration = Duration::from_secs(5);
/// How long `uninstall` keeps trying to remove a folder the web view may still hold for a moment after Kuchu quits.
const REMOVE_WITHIN: Duration = Duration::from_secs(5);

const USAGE: &str = "Kuchu, a desk pet for Claude Code.

  kuchu              start Kuchu
  kuchu setup        put Kuchu's relay in place, start Kuchu when you sign in, and start it now with its
                     Settings open, where you connect it to Claude Code after seeing the change
  kuchu status       how Kuchu stands: running, starting at sign-in, connected to Claude Code
  kuchu quit         quit a running Kuchu
  kuchu uninstall    take Kuchu off this account (disconnect it in Kuchu's Settings first)";

/// Where Kuchu keeps its settings: Tauri's app config folder.
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(IDENTIFIER))
}

/// Where Kuchu keeps its relay and its web view's data: Tauri's app local data folder.
pub fn data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join(IDENTIFIER))
}

/// Where the relay is put for the hooks to run: a path that stays the same across updates. From the Store, `~/.kuchu/bin`:
/// what such an app writes in AppData goes to a folder of its own, where Claude Code would not find it.
pub fn relay_dir() -> Option<PathBuf> {
    if packaged() {
        return dirs::home_dir().map(|home| home.join(".kuchu").join("bin"));
    }
    data_dir().map(|dir| dir.join("bin"))
}

/// Kuchu installed from the Microsoft Store: an MSIX package, whose manifest sits beside the program. Windows then
/// starts it at sign-in from the package's own startup task, and takes it off again when it is uninstalled.
pub fn packaged() -> bool {
    cfg!(windows)
        && std::env::current_exe().is_ok_and(|exe| exe.parent().is_some_and(|dir| dir.join("AppxManifest.xml").is_file()))
}

/// The relay shipped beside Kuchu (or, while developing, the one cargo built).
pub fn bundled_relay() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    [dir.join(hooks::RELAY_FILE), dir.join("..").join("release").join(hooks::RELAY_FILE), dir.join("..").join("debug").join(hooks::RELAY_FILE)]
        .into_iter()
        .find(|p| p.is_file())
}

/// Puts the bundled relay where the hooks point. Returns where it is.
pub fn place_relay() -> Result<PathBuf, String> {
    let from = bundled_relay().ok_or("The relay is missing beside Kuchu. Reinstall Kuchu.")?;
    let dir = relay_dir().ok_or("Kuchu could not find this account's app data folder.")?;
    hooks::place_relay(&from, &dir).map_err(|e| format!("Kuchu could not put its relay in place: {e}"))
}

/// Whether a Kuchu is listening for Claude Code on this account. Asked as a relay would: a connection opened and
/// closed at once, which Kuchu takes as an event that never came.
pub fn running() -> bool {
    matches!(kuchu_wire::open(Duration::ZERO), kuchu_wire::Pipe::Kuchu(_) | kuchu_wire::Pipe::Busy)
}

/// Whether something that is not this account's Kuchu holds Kuchu's address: no relay speaks to it, so nothing shows.
pub fn taken() -> bool {
    matches!(kuchu_wire::open(Duration::ZERO), kuchu_wire::Pipe::Someone)
}

// ── Starting at sign-in ──────────────────────────────────────────────────────────────────────────────────────────

/// Whether Kuchu starts when the person signs in.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Start {
    pub on: bool,
    /// The entry is there, but switched off in the system's own list of startup apps (Windows' Task Manager).
    pub paused: bool,
    /// The program the entry starts, when it is not this Kuchu (a copy somewhere else).
    pub elsewhere: Option<String>,
    /// Windows starts Kuchu itself (installed from the Store), and it is switched on and off in Windows' own Settings.
    pub by_windows: bool,
}

const OFF: Start = Start { on: false, paused: false, elsewhere: None, by_windows: false };

fn this_program() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Kuchu could not tell where it is: {e}"))?;
    Ok(std::fs::canonicalize(&exe).map(plain_path).unwrap_or(exe))
}

/// A path without Windows' `\\?\` prefix, which `canonicalize` adds and the Run key does not need.
fn plain_path(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|p| p.strip_prefix(r"\\?\")) {
        Some(p) if !p.starts_with("UNC\\") => PathBuf::from(p),
        _ => path,
    }
}

fn same_program(entry: &str, exe: &Path) -> bool {
    let entry = Path::new(entry);
    entry == exe || std::fs::canonicalize(entry).map(plain_path).is_ok_and(|e| e == exe)
}

#[cfg(windows)]
mod sign_in {
    use super::*;
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    /// Where Windows notes the startup apps switched off in Task Manager.
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    const ENTRY: &str = "Kuchu";

    pub fn get() -> Start {
        get_in(RUN, APPROVED)
    }

    pub fn set(on: bool) -> Result<(), String> {
        set_in(RUN, APPROVED, on)
    }

    pub(super) fn get_in(run: &str, approved: &str) -> Start {
        let user = RegKey::predef(HKEY_CURRENT_USER);
        let Ok(command) = user.open_subkey(run).and_then(|run| run.get_value::<String, _>(ENTRY)) else { return OFF };
        // The first byte's low bit set means switched off.
        let paused = user.open_subkey(approved).and_then(|k| k.get_raw_value(ENTRY)).is_ok_and(|v| v.bytes.first().is_some_and(|b| b & 1 == 1));
        let program = command.trim().trim_matches('"').to_string();
        let elsewhere = this_program().ok().filter(|exe| !same_program(&program, exe)).map(|_| program);
        Start { on: true, paused, elsewhere, by_windows: false }
    }

    pub(super) fn set_in(run: &str, approved: &str, on: bool) -> Result<(), String> {
        let user = RegKey::predef(HKEY_CURRENT_USER);
        // Switched on here after being switched off in Task Manager: on is what the person wants now.
        if let Ok(approved) = user.open_subkey_with_flags(approved, winreg::enums::KEY_SET_VALUE) {
            let _ = approved.delete_value(ENTRY);
        }
        if on {
            let exe = this_program()?;
            let (run, _) = user.create_subkey(run).map_err(|e| format!("Kuchu could not open its startup entry: {e}"))?;
            run.set_value(ENTRY, &format!("\"{}\"", exe.display())).map_err(|e| format!("Kuchu could not add its startup entry: {e}"))
        } else {
            match user.open_subkey_with_flags(run, winreg::enums::KEY_SET_VALUE).and_then(|run| run.delete_value(ENTRY)) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(format!("Kuchu could not remove its startup entry: {e}")),
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod sign_in {
    use super::*;

    fn agent() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join("Library").join("LaunchAgents").join(format!("{IDENTIFIER}.plist")))
    }

    fn escape(text: &str) -> String {
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    }

    pub fn get() -> Start {
        let Some(text) = agent().and_then(|path| std::fs::read_to_string(path).ok()) else { return OFF };
        let program = text
            .split_once("<key>ProgramArguments</key>")
            .and_then(|(_, rest)| rest.split_once("<string>"))
            .and_then(|(_, rest)| rest.split_once("</string>"))
            .map(|(program, _)| program.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&"))
            .unwrap_or_default();
        let elsewhere = this_program().ok().filter(|exe| !same_program(&program, exe)).map(|_| program);
        Start { on: true, paused: false, elsewhere, by_windows: false }
    }

    pub fn set(on: bool) -> Result<(), String> {
        let path = agent().ok_or("Kuchu could not find your home folder.")?;
        if !on {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("Kuchu could not remove {}: {e}", path.display())),
                _ => Ok(()),
            };
        }
        let exe = this_program()?;
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">
<plist version=\"1.0\">
<dict>
  <key>Label</key>
  <string>{IDENTIFIER}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
",
            escape(&exe.display().to_string())
        );
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("Kuchu could not make {}: {e}", dir.display()))?;
        }
        std::fs::write(&path, plist).map_err(|e| format!("Kuchu could not write {}: {e}", path.display()))
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod sign_in {
    use super::*;

    pub fn get() -> Start {
        OFF
    }

    pub fn set(_on: bool) -> Result<(), String> {
        Err("Starting at sign-in is not ready on this system yet.".into())
    }
}

/// Whether Kuchu starts when the person signs in.
pub fn start_at_sign_in() -> Start {
    if packaged() {
        return Start { on: true, paused: false, elsewhere: None, by_windows: true };
    }
    sign_in::get()
}

/// Starts this Kuchu when the person signs in, or no longer.
pub fn set_start_at_sign_in(on: bool) -> Result<Start, String> {
    if packaged() {
        return Err("Windows starts Kuchu when you sign in: switch it on or off in Windows' Settings, Apps, Startup.".into());
    }
    sign_in::set(on)?;
    Ok(sign_in::get())
}

// ── Commands ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// Runs `kuchu <command>`, if the first argument is one of Kuchu's commands, and returns its exit code; `None` starts
/// Kuchu itself (with no argument, or with flags: Kuchu's own `--settings` and `--quit`, or the system's).
pub fn command(args: &[String]) -> Option<i32> {
    let first = args.first()?;
    let code = match first.as_str() {
        "setup" => done(setup(!args.iter().any(|a| a == "--no-start"))),
        "status" => {
            status();
            0
        }
        "quit" => done(quit()),
        "uninstall" => done(uninstall()),
        "help" | "--help" | "-h" => {
            say(USAGE);
            0
        }
        "version" | "--version" | "-V" => {
            say(&format!("Kuchu {}", env!("CARGO_PKG_VERSION")));
            0
        }
        flag if flag.starts_with('-') => return None,
        other => {
            say(&format!("Kuchu has no command \"{other}\".\n\n{USAGE}"));
            2
        }
    };
    Some(code)
}

/// Writes to whoever ran the command. A Windows build has no console of its own, so this reaches a pipe (Claude
/// Code's shell, or `| Out-String` in PowerShell) and is lost otherwise; never an error.
fn say(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{text}");
    let _ = out.flush();
}

fn done(result: Result<(), String>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(why) => {
            say(&why);
            1
        }
    }
}

/// `now`: start Kuchu as well, with its Settings open. The Windows installer does not: its last page offers to start
/// Kuchu, and Kuchu's first start opens Settings anyway.
fn setup(now: bool) -> Result<(), String> {
    let relay = place_relay()?;
    let start = set_start_at_sign_in(true);
    let report = hooks::status(&hooks::settings_path(), &relay);
    if now {
        start_kuchu(&["--settings"])?;
    }
    let mut lines = vec![
        "Kuchu is set up.".to_string(),
        format!("  Kuchu:      {}", this_program()?.display()),
        format!("  Its relay:  {}", relay.display()),
        match &start {
            Ok(s) if s.on => "  It starts when you sign in.".to_string(),
            Ok(_) => "  It does not start when you sign in.".to_string(),
            Err(why) => format!("  It does not start when you sign in: {why}"),
        },
        String::new(),
    ];
    lines.push(match report.state.as_str() {
        _ if !now => "Start Kuchu; its Settings connect it to Claude Code, with the change shown first.".to_string(),
        "installed" => "Kuchu is running, and connected to Claude Code: every session shows beside it.".to_string(),
        "outdated" => format!(
            "Kuchu is running, with its Settings open. Kuchu's hooks in {} are from an older install: click Update \
             there to point them at this one (Kuchu shows the change first).",
            report.path
        ),
        "unreadable" => format!(
            "Kuchu is running, but it cannot connect to Claude Code: {}",
            report.why.unwrap_or_default()
        ),
        _ => format!(
            "Kuchu is running, with its Settings open. To show Claude Code's sessions beside it, click Connect there: \
             Kuchu first shows the change it would make to {}, and keeps a copy of the file. Nothing is changed until \
             that click.",
            report.path
        ),
    });
    say(&lines.join("\n"));
    Ok(())
}

fn status() {
    let exe = this_program().map(|p| p.display().to_string()).unwrap_or_else(|why| why);
    let relay = relay_dir().map(|dir| dir.join(hooks::RELAY_FILE));
    let relay_line = match (&relay, bundled_relay()) {
        (Some(placed), _) if !placed.is_file() => format!("not in place yet ({}): run kuchu setup", placed.display()),
        (Some(placed), Some(bundled)) if std::fs::read(placed).ok() != std::fs::read(&bundled).ok() => {
            format!("{} (from another version of Kuchu: it is replaced when this one starts)", placed.display())
        }
        (Some(placed), _) => placed.display().to_string(),
        (None, _) => "Kuchu could not find this account's app data folder".to_string(),
    };
    let start = start_at_sign_in();
    let start_line = match (&start.on, start.paused, &start.elsewhere) {
        (false, _, _) => "no".to_string(),
        (true, true, _) => "no, switched off in Windows' startup apps".to_string(),
        (true, false, Some(other)) => format!("yes, but another copy of Kuchu: {other}"),
        (true, false, None) => "yes".to_string(),
    };
    let report = relay.as_deref().map(|r| hooks::status(&hooks::settings_path(), r));
    let claude_line = match report {
        None => "unknown".to_string(),
        Some(r) => {
            let mut line = match r.state.as_str() {
                "installed" => format!("connected ({})", r.path),
                "outdated" => format!("connected to an older install ({}): click Update in Kuchu's Settings", r.path),
                "unreadable" => format!("unknown: {}", r.why.unwrap_or_default()),
                _ => format!("not connected ({}): click Connect in Kuchu's Settings", r.path),
            };
            if r.disabled {
                line.push_str(". disableAllHooks is on in that file, so no hook runs, Kuchu's included");
            }
            line
        }
    };
    say(&format!(
        "Kuchu {}\n  Program:                 {exe}\n  Running:                 {}\n  Starts when you sign in: {start_line}\n  \
         Relay:                   {relay_line}\n  Claude Code:             {claude_line}",
        env!("CARGO_PKG_VERSION"),
        if running() {
            "yes"
        } else if taken() {
            "no, and something else holds Kuchu's pipe: no session can reach Kuchu until it is gone"
        } else {
            "no"
        },
    ));
}

fn quit() -> Result<(), String> {
    if !running() {
        say("Kuchu is not running.");
        return Ok(());
    }
    ask_to_quit()?;
    if !gone_within(QUIT_WITHIN) {
        return Err("Kuchu did not quit. Quit it from its tray icon, then try again.".into());
    }
    say("Kuchu has quit.");
    Ok(())
}

fn uninstall() -> Result<(), String> {
    // Kuchu's hooks go first, and only from Kuchu's Settings, where the change is shown before it is made.
    let relay = relay_dir().map(|dir| dir.join(hooks::RELAY_FILE)).unwrap_or_default();
    let report = hooks::status(&hooks::settings_path(), &relay);
    if report.state != "missing" {
        let why = match report.state.as_str() {
            "unreadable" => format!(
                "Kuchu cannot tell whether its hooks are still in {}: {}\nFix the file, or take out the hooks that \
                 run kuchu-relay yourself, then run kuchu uninstall again.",
                report.path,
                report.why.unwrap_or_default()
            ),
            _ => format!(
                "Kuchu's hooks are still in {}. Kuchu's Settings are open: click Disconnect under Claude Code (it shows \
                 the change first), then run kuchu uninstall again.",
                report.path
            ),
        };
        if report.state != "unreadable" {
            start_kuchu(&["--settings"])?;
        }
        return Err(why);
    }
    if running() {
        ask_to_quit()?;
        if !gone_within(QUIT_WITHIN) {
            return Err("Kuchu did not quit, so nothing was removed. Quit it from its tray icon, then try again.".into());
        }
    }
    let mut kept = Vec::new();
    if let Err(why) = set_start_at_sign_in(false) {
        kept.push(why);
    }
    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut folders = vec![data_dir(), config_dir()];
    #[cfg(target_os = "macos")]
    folders.extend([dirs::cache_dir().map(|d| d.join(IDENTIFIER)), dirs::home_dir().map(|d| d.join("Library").join("WebKit").join(IDENTIFIER))]);
    for dir in folders.into_iter().flatten() {
        if let Err(why) = remove_own(&dir) {
            kept.push(why);
        }
    }
    #[cfg(not(windows))]
    if let Some(dir) = Path::new(&kuchu_wire::address()).parent().filter(|d| d.file_name().is_some_and(|n| n == ".kuchu")) {
        let _ = std::fs::remove_dir_all(dir);
    }
    let mut lines = vec![
        "Kuchu is off this account: it no longer starts when you sign in, and its relay, settings and data are gone."
            .to_string(),
        format!(
            "Claude Code's settings.json has no Kuchu hooks. The copies Kuchu kept of it (settings.json.kuchu-backup-…) \
             are still beside it, in {}, for you to keep or delete.",
            hooks::settings_path().parent().map(|d| d.display().to_string()).unwrap_or_default()
        ),
        format!("To finish, delete Kuchu itself: {}", this_program()?.display()),
    ];
    if !kept.is_empty() {
        lines.push(String::new());
        lines.push("Some of it could not be removed:".into());
        lines.extend(kept.into_iter().map(|why| format!("  {why}")));
    }
    say(&lines.join("\n"));
    Ok(())
}

/// Removes one of Kuchu's own folders: only a folder named after Kuchu, and it tries for a few seconds, since the web
/// view may hold its files for a moment after Kuchu quits.
fn remove_own(dir: &Path) -> Result<(), String> {
    if dir.file_name().and_then(|n| n.to_str()) != Some(IDENTIFIER) {
        return Err(format!("{} is not Kuchu's own folder, so it was left as it is.", dir.display()));
    }
    let until = Instant::now() + REMOVE_WITHIN;
    loop {
        match std::fs::remove_dir_all(dir) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) if Instant::now() >= until => return Err(format!("Kuchu could not remove {} ({e}).", dir.display())),
            Err(_) => std::thread::sleep(Duration::from_millis(200)),
        }
    }
}

fn gone_within(limit: Duration) -> bool {
    let until = Instant::now() + limit;
    while running() {
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    true
}

/// Asks the running Kuchu to quit. On Windows a second Kuchu started with `--quit` hands that to the first and goes;
/// on macOS the running one is sent the ordinary signal to end.
fn ask_to_quit() -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut child = Command::new(exe)
            .arg("--quit")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Kuchu could not ask itself to quit: {e}"))?;
        let until = Instant::now() + QUIT_WITHIN;
        while child.try_wait().ok().flatten().is_none() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let name = std::env::current_exe().ok().and_then(|e| e.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_else(|| "kuchu".into());
        let uid = Command::new("id").arg("-u").output().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
        let found = Command::new("pgrep").args(["-x", "-U", &uid, &name]).output().map_err(|e| format!("Kuchu could not look for itself: {e}"))?;
        let me = std::process::id().to_string();
        for pid in String::from_utf8_lossy(&found.stdout).lines().map(str::trim).filter(|p| !p.is_empty() && *p != me) {
            let _ = Command::new("kill").args(["-TERM", pid]).status();
        }
        Ok(())
    }
}

/// Starts Kuchu on its own, outside whatever ran this command (Claude Code's shell waits for its children, and may end
/// them). A Kuchu already running takes the arguments instead (on Windows; on macOS it is brought forward).
fn start_kuchu(args: &[&str]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        /// Its own process group, no console, and out of the job that may hold the shell's children.
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        let spawn = |flags: u32| {
            Command::new(&exe).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).creation_flags(flags).spawn()
        };
        spawn(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB)
            .or_else(|_| spawn(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP))
            .map(drop)
            .map_err(|e| format!("Kuchu could not start: {e}"))
    }
    #[cfg(not(windows))]
    {
        // Inside an app bundle, through the system, which keeps it to one copy.
        if let Some(app) = exe.ancestors().find(|p| p.extension().is_some_and(|e| e == "app")) {
            return Command::new("open")
                .arg("-a")
                .arg(app)
                .arg("--args")
                .args(args)
                .status()
                .map_err(|e| format!("Kuchu could not start: {e}"))
                .and_then(|s| if s.success() { Ok(()) } else { Err("Kuchu could not start.".into()) });
        }
        use std::os::unix::process::CommandExt;
        Command::new(&exe).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).process_group(0).spawn().map(drop).map_err(|e| format!("Kuchu could not start: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kuchus_folders_are_the_ones_tauri_gives_it() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["identifier"], IDENTIFIER);
        assert!(conf["app"].get("appDirectoriesOverride").is_none(), "Tauri would then put Kuchu's folders elsewhere");
        assert!(data_dir().unwrap().ends_with(IDENTIFIER));
        assert!(relay_dir().unwrap().starts_with(data_dir().unwrap()));
    }

    #[test]
    fn only_kuchus_own_commands_are_taken_as_commands() {
        assert_eq!(command(&[]), None);
        assert_eq!(command(&["--settings".into()]), None);
        assert_eq!(command(&["-psn_0_12345".into()]), None, "macOS may start an app with this");
        assert_eq!(command(&["--version".into()]), Some(0));
        assert_eq!(command(&["instal".into()]), Some(2));
    }

    /// Against a key of the test's own, never the real Run key: nothing is set to start.
    #[cfg(windows)]
    #[test]
    fn the_startup_entry_starts_this_kuchu_notices_a_switch_off_and_comes_off_again() {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let user = RegKey::predef(HKEY_CURRENT_USER);
        let base = format!(r"Software\{IDENTIFIER}-test-{}", std::process::id());
        let (run, approved) = (format!(r"{base}\Run"), format!(r"{base}\Approved"));
        let switched_off = winreg::RegValue { bytes: vec![3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0].into(), vtype: winreg::enums::REG_BINARY };
        let (marks, _) = user.create_subkey(&approved).unwrap();

        assert_eq!(sign_in::get_in(&run, &approved), OFF);
        marks.set_raw_value("Kuchu", &switched_off).unwrap();
        sign_in::set_in(&run, &approved, true).unwrap();
        assert_eq!(sign_in::get_in(&run, &approved), Start { on: true, paused: false, elsewhere: None, by_windows: false }, "on here wins over off there");
        marks.set_raw_value("Kuchu", &switched_off).unwrap();
        assert!(sign_in::get_in(&run, &approved).paused);
        user.open_subkey_with_flags(&run, winreg::enums::KEY_SET_VALUE).unwrap().set_value("Kuchu", &r#""C:\Elsewhere\kuchu.exe""#).unwrap();
        assert_eq!(sign_in::get_in(&run, &approved).elsewhere.as_deref(), Some(r"C:\Elsewhere\kuchu.exe"));
        sign_in::set_in(&run, &approved, false).unwrap();
        assert_eq!(sign_in::get_in(&run, &approved), OFF);
        sign_in::set_in(&run, &approved, false).unwrap();

        user.delete_subkey_all(&base).unwrap();
    }

    #[test]
    fn a_folder_not_named_after_kuchu_is_never_removed() {
        let dir = std::env::temp_dir().join(format!("kuchu-not-ours-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(remove_own(&dir).is_err());
        assert!(dir.is_dir());
        let ours = dir.join(IDENTIFIER);
        std::fs::create_dir_all(ours.join("bin")).unwrap();
        remove_own(&ours).unwrap();
        assert!(!ours.exists() && dir.is_dir());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
