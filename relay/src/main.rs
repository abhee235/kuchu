//! Kuchu's hook relay. Claude Code runs it on each hook event with the event's JSON on stdin:
//!
//!   kuchu-relay <event>                    hand the event to Kuchu
//!   kuchu-relay record <event> [probe]     keep the event as it came, to learn what Claude Code sends
//!
//! It never holds Claude Code up: all the work runs on a second thread while the first waits with a deadline (2 s, or
//! 110 s for a permission request), it reaches Kuchu within 300 ms or gives up, and every path ends with exit code 0.
//! It speaks only to a pipe this account made (`kuchu_wire::open`), and prints only one thing: Kuchu's answer to a
//! permission request, checked against the request and in the shape Claude Code reads. Anything else (no answer,
//! Kuchu closed, a timeout, a garbled reply, an answer the request never offered) prints nothing, and Claude Code asks
//! as it would have.

use std::io::{BufRead, BufReader, Read, Write};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use kuchu_wire::Pipe;
use serde_json::{json, Map, Value};

/// Strings longer than this are cut before they leave the relay (a file being written, a long command).
const MAX_TEXT: usize = 2048;
/// A permission request keeps its input whole (it may be sent back as `updatedInput`), up to this much.
const MAX_REQUEST: usize = 512 * 1024;
/// Fields Kuchu never needs: a tool's whole output.
const DROPPED: &[&str] = &["tool_response", "tool_output"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("record") => record(&args[1..]),
        Some(event) => relay(event),
        None => {}
    }
}

fn relay(event: &str) {
    let asking = event == "PermissionRequest";
    let budget = if asking { kuchu_wire::PERMISSION } else { kuchu_wire::EVENT };
    let event = event.to_string();
    let (done, outcome) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(hand_over(&event));
    });
    if let Ok(Some(text)) = outcome.recv_timeout(budget) {
        let mut out = std::io::stdout();
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    }
}

/// Reads the event, sends it to Kuchu, and for a permission request returns what to print.
fn hand_over(event: &str) -> Option<String> {
    let mut raw = Vec::new();
    std::io::stdin().read_to_end(&mut raw).ok()?;
    let text = String::from_utf8_lossy(&raw);
    let payload: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let asking = event == "PermissionRequest";
    let payload = if asking { payload } else { slim(payload) };
    let line = json!({ "v": kuchu_wire::VERSION, "event": event, "payload": payload }).to_string();
    if line.len() > MAX_REQUEST {
        return None;
    }

    let Pipe::Kuchu(mut conn) = kuchu_wire::open(kuchu_wire::CONNECT) else { return None };
    conn.write_all(line.as_bytes()).ok()?;
    conn.write_all(b"\n").ok()?;
    conn.flush().ok()?;
    if !asking {
        return None;
    }
    let mut reply = String::new();
    BufReader::new(conn).take(kuchu_wire::MAX_LINE as u64).read_line(&mut reply).ok()?;
    answer(&reply, &payload)
}

/// Kuchu's decision, checked against the request it answers, in the shape Claude Code reads; nothing for anything
/// else. Only what the request offered may come back: the input as it was (a plan approved), or with the answers to
/// its questions added; a rule or mode Claude Code itself suggested. So whatever answers at the pipe can say yes or
/// no to what Claude Code asked, and cannot change it.
fn answer(reply: &str, request: &Value) -> Option<String> {
    let decision: Value = serde_json::from_str(reply.trim()).ok()?;
    let fields = decision.as_object()?;
    let allowed: &[&str] = match fields.get("behavior")?.as_str()? {
        "allow" => &["behavior", "updatedInput", "updatedPermissions"],
        "deny" => &["behavior", "message"],
        _ => return None,
    };
    if fields.keys().any(|k| !allowed.contains(&k.as_str())) {
        return None;
    }
    if fields.get("message").is_some_and(|m| !m.is_string()) {
        return None;
    }
    if fields.get("updatedInput").is_some_and(|input| !offered_input(request, input)) {
        return None;
    }
    if let Some(rules) = fields.get("updatedPermissions") {
        let offered = request.get("permission_suggestions").and_then(Value::as_array);
        let rules = rules.as_array()?;
        if rules.is_empty() || !rules.iter().all(|r| offered.is_some_and(|o| o.contains(r))) {
            return None;
        }
    }
    Some(json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": decision } }).to_string())
}

/// Whether `updated` is the request's own input: the same, or, for a question, the same with only its answers added.
fn offered_input(request: &Value, updated: &Value) -> bool {
    let input = request.get("tool_input").unwrap_or(&Value::Null);
    if updated == input {
        return true;
    }
    if request.get("tool_name").and_then(Value::as_str) != Some("AskUserQuestion") {
        return false;
    }
    let Some(mut fields) = updated.as_object().cloned() else { return false };
    let answered = fields.remove("answers").is_some_and(|a| a.as_object().is_some_and(|a| a.values().all(Value::is_string)));
    answered && Value::Object(fields) == *input
}

/// The event without what Kuchu never shows: tools' outputs dropped, long strings cut.
fn slim(mut payload: Value) -> Value {
    if let Some(map) = payload.as_object_mut() {
        for key in DROPPED {
            map.remove(*key);
        }
    }
    cut(payload)
}

fn cut(value: Value) -> Value {
    match value {
        Value::String(s) if s.len() > MAX_TEXT => {
            let mut end = MAX_TEXT;
            while !s.is_char_boundary(end) {
                end -= 1;
            }
            Value::String(format!("{}…", &s[..end]))
        }
        Value::Array(items) => Value::Array(items.into_iter().map(cut).collect()),
        Value::Object(map) => Value::Object(map.into_iter().map(|(k, v)| (k, cut(v))).collect::<Map<_, _>>()),
        other => other,
    }
}

/// `record`: one JSON line per event in `KUCHU_RECORD` (default: kuchu-record.jsonl in the temp folder), with the
/// probe (`"$0"` in the hook command: the shell's name under bash), a few environment variables, and the payload.
fn record(args: &[String]) {
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
    let mut raw = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw);
    let text = text.trim_start_matches('\u{feff}');
    let payload = serde_json::from_str::<Value>(text).unwrap_or_else(|_| Value::String(text.to_string()));
    let env: Map<String, Value> = ENV_KEYS
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), Value::String(v))))
        .collect();
    let at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let line = json!({ "at": at, "event": args.first(), "probe": args.get(1), "env": env, "payload": payload });
    let path = std::env::var_os("KUCHU_RECORD")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("kuchu-record.jsonl"));
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A request as Claude Code sends it: a Write, with a mode on offer.
    fn request() -> Value {
        json!({
            "tool_name": "Write",
            "tool_input": { "file_path": "note.txt", "content": "hi" },
            "permission_suggestions": [{ "type": "setMode", "mode": "acceptEdits", "destination": "session" }]
        })
    }

    #[test]
    fn only_allow_or_deny_reaches_claude_code() {
        let r = request();
        let allow: Value = serde_json::from_str(&answer(r#"{"behavior":"allow"}"#, &r).unwrap()).unwrap();
        assert_eq!(allow, json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": { "behavior": "allow" } } }));
        assert!(answer(r#"{"behavior":"deny","message":"Not now"}"#, &r).unwrap().contains(r#""message":"Not now""#));
        assert_eq!(answer("", &r), None);
        assert_eq!(answer("allow", &r), None);
        assert_eq!(answer("[]", &r), None);
        assert_eq!(answer(r#"{"behavior":"ask"}"#, &r), None);
        assert_eq!(answer(r#"{"behavior":"allow","interrupt":true}"#, &r), None);
        assert_eq!(answer(r#"{"behavior":"allow","message":"x"}"#, &r), None);
        assert_eq!(answer(r#"{"behavior":"deny","message":1}"#, &r), None);
        assert_eq!(answer(r#"{"behavior":"deny","updatedInput":{}}"#, &r), None);
    }

    #[test]
    fn only_what_the_request_offered_comes_back() {
        let r = request();
        let decide = |d: Value| answer(&d.to_string(), &r);
        // The input as it was (a plan approved): offered. Any other input: not.
        assert!(decide(json!({ "behavior": "allow", "updatedInput": r["tool_input"] })).is_some());
        assert_eq!(decide(json!({ "behavior": "allow", "updatedInput": { "file_path": "../.bashrc", "content": "hi" } })), None);
        assert_eq!(decide(json!({ "behavior": "allow", "updatedInput": { "file_path": "note.txt", "content": "hi", "answers": { "a": "b" } } })), None);
        // A mode or rule Claude Code itself suggested: offered. Another, or none: not.
        assert!(decide(json!({ "behavior": "allow", "updatedPermissions": [r["permission_suggestions"][0]] })).is_some());
        let other = json!({ "type": "addRules", "rules": [{ "toolName": "Bash" }], "destination": "userSettings" });
        assert_eq!(decide(json!({ "behavior": "allow", "updatedPermissions": [other] })), None);
        assert_eq!(decide(json!({ "behavior": "allow", "updatedPermissions": [] })), None);
        assert_eq!(decide(json!({ "behavior": "allow", "updatedPermissions": "all" })), None);

        // A question's input may come back with only its answers added.
        let q = json!({ "tool_name": "AskUserQuestion", "tool_input": { "questions": [{ "question": "Which?", "options": [{ "label": "A" }] }] } });
        let answered = |answers: Value, questions: Option<Value>| {
            let mut input = q["tool_input"].clone();
            input["answers"] = answers;
            if let Some(qs) = questions {
                input["questions"] = qs;
            }
            answer(&json!({ "behavior": "allow", "updatedInput": input }).to_string(), &q)
        };
        assert!(answered(json!({ "Which?": "A" }), None).is_some());
        assert_eq!(answered(json!({ "Which?": "A" }), Some(json!([]))), None);
        assert_eq!(answered(json!("A"), None), None);
        assert_eq!(answered(json!({ "Which?": ["A"] }), None), None);
    }

    #[test]
    fn outputs_are_dropped_and_long_text_cut() {
        let long = "é".repeat(MAX_TEXT);
        let slimmed = slim(json!({ "tool_response": { "stdout": "x" }, "tool_input": { "content": long } }));
        assert!(slimmed.get("tool_response").is_none());
        let content = slimmed["tool_input"]["content"].as_str().unwrap();
        assert!(content.len() <= MAX_TEXT + '…'.len_utf8());
        assert!(content.ends_with('…'));
    }

    #[test]
    fn the_recorded_payloads_still_read() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/claude-code-2.1.310");
        // The hook payloads; `stream/` holds what `claude -p` prints, which the relay never sees.
        for path in std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_file()) {
            let text = std::fs::read_to_string(path).unwrap();
            let payload: Value = serde_json::from_str(&text).unwrap();
            assert!(payload.get("session_id").is_some());
            assert!(slim(payload).get("tool_response").is_none());
        }
    }
}
