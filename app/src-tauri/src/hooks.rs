//! Claude Code's settings.json, changed only for Kuchu's own hooks.
//!
//! The file is the user's: one that cannot be read or parsed is refused, never taken as empty; the change is shown
//! first, as lines added and removed; it is written only after a click, and only if the file is the same as when it
//! was shown; a dated copy is kept beside it first; and the new file is written beside the old one and renamed over
//! it. Only entries whose command runs `kuchu-relay` are ever added or removed, so the user's own hooks, and other
//! tools' hooks, stay exactly as they were.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{json, Map, Value};

/// The events Kuchu listens to, with each hook's timeout (seconds).
pub const EVENTS: &[(&str, u32)] = &[
    ("SessionStart", kuchu_wire::HOOK_TIMEOUT),
    ("SessionEnd", kuchu_wire::HOOK_TIMEOUT),
    ("UserPromptSubmit", kuchu_wire::HOOK_TIMEOUT),
    ("PreToolUse", kuchu_wire::HOOK_TIMEOUT),
    ("PostToolUse", kuchu_wire::HOOK_TIMEOUT),
    ("PostToolUseFailure", kuchu_wire::HOOK_TIMEOUT),
    ("PermissionRequest", kuchu_wire::HOOK_TIMEOUT_PERMISSION),
    ("Notification", kuchu_wire::HOOK_TIMEOUT),
    ("Stop", kuchu_wire::HOOK_TIMEOUT),
    ("StopFailure", kuchu_wire::HOOK_TIMEOUT),
    ("PreCompact", kuchu_wire::HOOK_TIMEOUT),
];

/// What marks a hook as Kuchu's: its command runs the relay.
const MARK: &str = "kuchu-relay";

#[cfg(windows)]
pub const RELAY_FILE: &str = "kuchu-relay.exe";
#[cfg(not(windows))]
pub const RELAY_FILE: &str = "kuchu-relay";

/// Where Claude Code keeps the user's settings: `$CLAUDE_CONFIG_DIR`, or `.claude` in the home folder.
pub fn settings_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir).join("settings.json");
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    PathBuf::from(home).join(".claude").join("settings.json")
}

/// The hook command for one event. Claude Code runs hooks through a POSIX shell (Git Bash on Windows), so the path is
/// quoted, with forward slashes on Windows.
pub fn command(relay: &Path, event: &str) -> String {
    let path = relay.display().to_string();
    if cfg!(windows) {
        format!("\"{}\" {event}", path.replace('\\', "/"))
    } else {
        format!("'{}' {event}", path.replace('\'', r"'\''"))
    }
}

/// How Kuchu's hooks stand in the file.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// "installed", "missing", "outdated" (some, or an older relay's path) or "unreadable".
    pub state: String,
    pub why: Option<String>,
    pub path: String,
    /// `disableAllHooks` is on in this file: no hook runs, Kuchu's included.
    pub disabled: bool,
}

/// A change to show before it is made.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub install: bool,
    pub path: String,
    /// Where the copy of the file as it is now will be kept.
    pub backup: String,
    pub diff: Vec<Line>,
    /// The file as it was shown: the change is written only if it is still this.
    pub fingerprint: String,
    /// Nothing to change.
    pub unchanged: bool,
    /// The file is spaced differently from how Claude Code writes it, and will be written its way (what it holds
    /// changes only as the diff shows).
    pub respaced: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Line {
    /// '+' added, '-' removed, ' ' kept (context), '…' kept lines left out.
    pub kind: char,
    pub text: String,
}

/// The file as it is: its bytes and its settings. A missing or empty file is `{}`; anything unreadable is refused.
fn read(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((Vec::new(), json!({}))),
        Err(e) => return Err(format!("Kuchu could not read {}: {e}", path.display())),
    };
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim_start_matches('\u{feff}');
    if text.trim().is_empty() {
        return Ok((bytes, json!({})));
    }
    let value: Value = serde_json::from_str(text).map_err(|e| format!("{} is not valid JSON ({e}). Kuchu leaves it as it is.", path.display()))?;
    if !value.is_object() {
        return Err(format!("{} does not hold settings (an object). Kuchu leaves it as it is.", path.display()));
    }
    match value.get("hooks") {
        None => {}
        Some(Value::Object(events)) => {
            if let Some((name, _)) = events.iter().find(|(_, v)| !v.is_array()) {
                return Err(format!("\"hooks.{name}\" in {} is not a list. Kuchu leaves the file as it is.", path.display()));
            }
        }
        Some(_) => return Err(format!("\"hooks\" in {} is not an object. Kuchu leaves the file as it is.", path.display())),
    }
    Ok((bytes, value))
}

fn is_ours(hook: &Value) -> bool {
    hook.get("command").and_then(Value::as_str).is_some_and(|c| c.contains(MARK))
}

/// The settings with Kuchu's hooks taken out, then (to install) put back for the relay at `relay`.
fn changed(settings: &Value, relay: &Path, install: bool) -> Value {
    let mut root = settings.clone();
    let Some(obj) = root.as_object_mut() else { return root };
    let had_hooks = obj.contains_key("hooks");
    let hooks = obj.entry("hooks").or_insert_with(|| Value::Object(Map::new()));
    if let Some(events) = hooks.as_object_mut() {
        let names: Vec<String> = events.keys().cloned().collect();
        for name in names {
            let Some(groups) = events.get_mut(&name).and_then(Value::as_array_mut) else { continue };
            let before = groups.len();
            let mut touched = false;
            for group in groups.iter_mut() {
                if let Some(list) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                    let n = list.len();
                    list.retain(|h| !is_ours(h));
                    touched |= list.len() != n;
                }
            }
            if touched {
                groups.retain(|g| g.get("hooks").and_then(Value::as_array).is_none_or(|l| !l.is_empty()));
                if groups.is_empty() && before > 0 {
                    events.remove(&name);
                }
            }
        }
        if install {
            for (event, timeout) in EVENTS {
                let group = json!({ "hooks": [{ "type": "command", "command": command(relay, event), "timeout": timeout }] });
                match events.get_mut(*event).and_then(Value::as_array_mut) {
                    Some(groups) => groups.push(group),
                    None => {
                        events.insert(event.to_string(), json!([group]));
                    }
                }
            }
        }
        if events.is_empty() && !had_hooks {
            obj.remove("hooks");
        }
    }
    root
}

/// How Kuchu's hooks stand.
pub fn status(path: &Path, relay: &Path) -> Report {
    let shown = path.display().to_string();
    let (_, settings) = match read(path) {
        Ok(r) => r,
        Err(why) => return Report { state: "unreadable".into(), why: Some(why), path: shown, disabled: false },
    };
    let disabled = settings.get("disableAllHooks").and_then(Value::as_bool).unwrap_or(false);
    let ours: Vec<String> = settings
        .get("hooks")
        .and_then(Value::as_object)
        .map(|events| {
            events
                .values()
                .filter_map(Value::as_array)
                .flatten()
                .filter_map(|g| g.get("hooks").and_then(Value::as_array))
                .flatten()
                .filter(|h| is_ours(h))
                .filter_map(|h| h.get("command").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let wanted: Vec<String> = EVENTS.iter().map(|(e, _)| command(relay, e)).collect();
    let state = if ours.is_empty() {
        "missing"
    } else if wanted.iter().all(|w| ours.contains(w)) && ours.len() == wanted.len() {
        "installed"
    } else {
        "outdated"
    };
    Report { state: state.into(), why: None, path: shown, disabled }
}

/// The change, to show before it is made.
pub fn plan(path: &Path, relay: &Path, install: bool) -> Result<Plan, String> {
    let (bytes, settings) = read(path)?;
    let after = changed(&settings, relay, install);
    let before_text = String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string();
    // The diff compares what the file holds, both sides written the same way: spacing alone never shows as a change.
    let before_settings = render(&settings, &before_text);
    let after_text = render(&after, &before_text);
    let unchanged = after == settings && !bytes.is_empty() || (bytes.is_empty() && !install);
    Ok(Plan {
        install,
        path: path.display().to_string(),
        backup: backup_path(path).display().to_string(),
        diff: diff(if bytes.is_empty() { "" } else { &before_settings }, &after_text),
        fingerprint: fingerprint(&bytes),
        unchanged,
        respaced: !bytes.is_empty() && before_text.trim_end() != before_settings.trim_end(),
    })
}

/// Makes the change shown in `plan`, if the file is still as it was then. Returns where the copy was kept.
pub fn apply(path: &Path, relay: &Path, install: bool, seen: &str) -> Result<Option<String>, String> {
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let (bytes, settings) = read(&target)?;
    if fingerprint(&bytes) != seen {
        return Err("settings.json changed since it was shown. Look at the change again.".into());
    }
    let after = changed(&settings, relay, install);
    if after == settings && !bytes.is_empty() {
        return Ok(None);
    }
    let before_text = String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string();
    let text = render(&after, &before_text);
    let mut kept = None;
    if !bytes.is_empty() {
        let backup = backup_path(&target);
        std::fs::write(&backup, &bytes).map_err(|e| format!("Kuchu could not keep a copy at {} ({e}), so it changed nothing.", backup.display()))?;
        kept = Some(backup.display().to_string());
    }
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = target.with_extension(format!("json.kuchu-{}", std::process::id()));
    std::fs::write(&tmp, text.as_bytes()).map_err(|e| format!("Kuchu could not write {} ({e}).", tmp.display()))?;
    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(&target) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    std::fs::rename(&tmp, &target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("Kuchu could not replace {} ({e}).", target.display())
    })?;
    Ok(kept)
}

/// The settings as text, two spaces deep like Claude Code writes them, ending as the old file did.
fn render(settings: &Value, before: &str) -> String {
    let mut text = serde_json::to_string_pretty(settings).unwrap_or_else(|_| "{}".into());
    if before.ends_with('\n') || before.is_empty() {
        text.push('\n');
    }
    text
}

fn fingerprint(bytes: &[u8]) -> String {
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    format!("{:016x}-{}", h.finish(), bytes.len())
}

fn backup_path(path: &Path) -> PathBuf {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("settings.json");
    let mut candidate = path.with_file_name(format!("{name}.kuchu-backup-{secs}"));
    let mut n = 1;
    while candidate.exists() {
        candidate = path.with_file_name(format!("{name}.kuchu-backup-{secs}-{n}"));
        n += 1;
    }
    candidate
}

/// Lines added and removed, with two lines kept around each change and the rest left out.
fn diff(before: &str, after: &str) -> Vec<Line> {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    if a.len() * b.len() > 4_000_000 {
        return b.iter().map(|l| Line { kind: '+', text: l.to_string() }).collect();
    }
    // Longest common subsequence, then walked forwards.
    let mut lcs = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut all = Vec::new();
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            all.push((' ', a[i]));
            i += 1;
            j += 1;
        } else if j < b.len() && (i == a.len() || lcs[i][j + 1] >= lcs[i + 1][j]) {
            all.push(('+', b[j]));
            j += 1;
        } else {
            all.push(('-', a[i]));
            i += 1;
        }
    }
    const AROUND: usize = 2;
    let near = |k: usize| all[k.saturating_sub(AROUND)..(k + AROUND + 1).min(all.len())].iter().any(|(c, _)| *c != ' ');
    let mut out: Vec<Line> = Vec::new();
    for (k, (kind, text)) in all.iter().enumerate() {
        if *kind != ' ' || near(k) {
            out.push(Line { kind: *kind, text: text.to_string() });
        } else if out.last().is_none_or(|l| l.kind != '…') {
            out.push(Line { kind: '…', text: String::new() });
        }
    }
    out
}

/// Puts the relay where the hooks point, at a path that stays the same across updates: copied beside and renamed into
/// place (a running copy is moved aside first, which Windows allows).
pub fn place_relay(bundled: &Path, dir: &Path) -> std::io::Result<PathBuf> {
    let dest = dir.join(RELAY_FILE);
    let new = std::fs::read(bundled)?;
    if std::fs::read(&dest).is_ok_and(|old| old == new) {
        return Ok(dest);
    }
    std::fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{RELAY_FILE}.new"));
    std::fs::write(&tmp, &new)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    if dest.exists() {
        let old = dir.join(format!("{RELAY_FILE}.old"));
        let _ = std::fs::remove_file(&old);
        let _ = std::fs::rename(&dest, &old);
    }
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kuchu-hooks-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("settings.json")
    }

    const RELAY: &str = if cfg!(windows) { r"C:\Users\Ana María\AppData\Local\Kuchu\bin\kuchu-relay.exe" } else { "/home/ana maria/.kuchu/bin/kuchu-relay" };

    #[test]
    fn install_keeps_the_users_own_hooks_and_settings() {
        let path = temp("keep");
        let mine = r#"{
  "model": "opus",
  "hooks": {
    "PreToolUse": [
      { "matcher": "Bash", "hooks": [{ "type": "command", "command": "./check.sh" }] }
    ]
  }
}
"#;
        std::fs::write(&path, mine).unwrap();
        let relay = Path::new(RELAY);
        assert_eq!(status(&path, relay).state, "missing");
        let plan = plan(&path, relay, true).unwrap();
        assert!(!plan.unchanged);
        assert!(plan.diff.iter().any(|l| l.kind == '+' && l.text.contains("kuchu-relay")));
        assert!(plan.diff.iter().all(|l| l.kind != '-'), "nothing of the user's is removed: {:?}", plan.diff);
        assert!(plan.respaced, "the hand-spaced line is said to be rewritten Claude Code's way");
        let kept = apply(&path, relay, true, &plan.fingerprint).unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(kept).unwrap(), mine);
        assert_eq!(status(&path, relay).state, "installed");
        let after: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(after["model"], "opus");
        assert_eq!(after["hooks"]["PreToolUse"][0]["hooks"][0]["command"], "./check.sh");
        assert_eq!(after["hooks"]["PermissionRequest"][0]["hooks"][0]["timeout"], 120);
        // Installing again changes nothing; removing gives back exactly what the user had.
        assert!(super::plan(&path, relay, true).unwrap().unchanged);
        let removal = super::plan(&path, relay, false).unwrap();
        apply(&path, relay, false, &removal.fingerprint).unwrap();
        let back: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back, serde_json::from_str::<Value>(mine).unwrap());
    }

    #[test]
    fn a_broken_file_is_refused_never_replaced() {
        let path = temp("broken");
        std::fs::write(&path, "{ \"hooks\": ").unwrap();
        assert_eq!(status(&path, Path::new(RELAY)).state, "unreadable");
        assert!(plan(&path, Path::new(RELAY), true).is_err());
        std::fs::write(&path, r#"{ "hooks": [] }"#).unwrap();
        assert!(plan(&path, Path::new(RELAY), true).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), r#"{ "hooks": [] }"#);
    }

    #[test]
    fn a_file_changed_since_it_was_shown_is_not_written() {
        let path = temp("changed");
        std::fs::write(&path, "{}").unwrap();
        let plan = plan(&path, Path::new(RELAY), true).unwrap();
        std::fs::write(&path, r#"{ "model": "sonnet" }"#).unwrap();
        assert!(apply(&path, Path::new(RELAY), true, &plan.fingerprint).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), r#"{ "model": "sonnet" }"#);
    }

    #[test]
    fn no_file_yet_is_made_and_hooks_switched_off_are_noticed() {
        let path = temp("none");
        let plan = plan(&path, Path::new(RELAY), true).unwrap();
        assert_eq!(apply(&path, Path::new(RELAY), true, &plan.fingerprint).unwrap(), None);
        assert_eq!(status(&path, Path::new(RELAY)).state, "installed");
        std::fs::write(&path, r#"{ "disableAllHooks": true }"#).unwrap();
        assert!(status(&path, Path::new(RELAY)).disabled);
    }

    #[test]
    fn a_moved_relay_reads_as_outdated() {
        let path = temp("moved");
        let plan = plan(&path, Path::new("/old/place/kuchu-relay"), true).unwrap();
        apply(&path, Path::new("/old/place/kuchu-relay"), true, &plan.fingerprint).unwrap();
        assert_eq!(status(&path, Path::new(RELAY)).state, "outdated");
    }

    #[test]
    fn the_command_is_quoted_for_a_posix_shell() {
        let c = command(Path::new(RELAY), "Stop");
        if cfg!(windows) {
            assert_eq!(c, "\"C:/Users/Ana María/AppData/Local/Kuchu/bin/kuchu-relay.exe\" Stop");
        } else {
            assert_eq!(c, "'/home/ana maria/.kuchu/bin/kuchu-relay' Stop");
        }
    }
}
