//! Kuchu's hook relay. Claude Code runs it on each hook event with the event's JSON on stdin.
//!
//! For now it has one mode, `record`, used to learn exactly what Claude Code sends (step 0 of PLAN.md):
//!
//!   kuchu-relay record <event> [shell-probe]
//!
//! It appends one JSON line to the file named by `KUCHU_RECORD` (default: kuchu-record.jsonl in the temp folder) with
//! the event name it was given, the probe (`"$0"` in the hook command: a shell's name under bash, the literal text
//! under cmd), a few environment variables, and the payload as Claude Code sent it. It prints nothing and always exits
//! 0: a relay must never change what Claude Code does.

use std::io::{Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

/// The environment that tells where and how the hook ran.
const ENV_KEYS: &[&str] = &[
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_PROJECT_DIR",
    "CLAUDE_CONFIG_DIR",
    "MSYSTEM",
    "SHELL",
    "ComSpec",
    "TERM_PROGRAM",
    "WT_SESSION",
    "VSCODE_PID",
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("record") {
        record(&args[1..]);
    }
    // Every path ends here: no output, exit 0.
}

fn record(args: &[String]) {
    let mut raw = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw);
    let text = text.trim_start_matches('\u{feff}');
    let payload = serde_json::from_str::<serde_json::Value>(text).unwrap_or_else(|_| serde_json::Value::String(text.to_string()));

    let env: serde_json::Map<String, serde_json::Value> = ENV_KEYS
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), serde_json::Value::String(v))))
        .collect();
    let at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let line = serde_json::json!({
        "at": at,
        "event": args.first(),
        "probe": args.get(1),
        "env": env,
        "payload": payload,
    });

    let path = std::env::var_os("KUCHU_RECORD")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("kuchu-record.jsonl"));
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}
