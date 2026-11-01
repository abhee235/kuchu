//! Which Claude Code sessions still run, as Claude Code itself lists them (`claude agents --json`: every session it has
//! running, interactive, background or `-p`, with its id and whether it is busy). A session whose terminal was closed
//! sends no more events, and one stopped with Esc sends no Stop: without asking, Kuchu would show either as working
//! until it went quiet.

use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;

/// How long Claude Code has to answer.
const ASK: Duration = Duration::from_secs(10);

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A session Claude Code has running.
#[derive(Debug, PartialEq)]
pub struct Running {
    pub id: String,
    pub busy: bool,
}

/// The sessions Claude Code has running; nothing when it cannot be asked (not found, an error, no answer in time), so
/// a failed question never ends a session.
pub async fn running() -> Option<Vec<Running>> {
    let claude = crate::chat::find_claude()?;
    let mut cmd = tokio::process::Command::new(claude);
    cmd.args(["agents", "--json"]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).kill_on_drop(true);
    cmd.env_remove("CLAUDECODE").env_remove("CLAUDE_CODE_ENTRYPOINT");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = tokio::time::timeout(ASK, cmd.output()).await.ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    read(&out.stdout)
}

/// The listing, read: entries without a session id are left out; anything but a list is no answer.
fn read(bytes: &[u8]) -> Option<Vec<Running>> {
    let list: Vec<Value> = serde_json::from_slice(bytes).ok()?;
    Some(
        list.iter()
            .filter_map(|a| Some(Running { id: a.get("sessionId")?.as_str()?.to_string(), busy: a.get("status").and_then(Value::as_str) == Some("busy") }))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recorded_listing_reads_as_running_sessions() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/claude-code-2.1.310/cli/agents.json");
        let running = read(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(running.len(), 6);
        assert_eq!(running.iter().filter(|r| r.busy).count(), 2);
        assert_eq!(running[0], Running { id: "9564d32e-cff6-482b-8927-5f2ef7692c72".into(), busy: false });
        assert_eq!(read(b"not json"), None);
        assert_eq!(read(b"{}"), None);
    }
}
