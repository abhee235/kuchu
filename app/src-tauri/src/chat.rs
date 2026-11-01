//! Conversations started from Kuchu: one Claude Code process per conversation (`claude -p`, stream-json both ways) in a
//! folder the person picked. Their messages go to its input; what it writes comes back as entries the panel shows:
//! their messages, Claude's text as it streams, each tool as one step. The process stays while the conversation is in
//! use and closes after a quiet while; the next message takes the conversation up again with `--resume`. Claude Code's
//! hooks still report it, so it shows on the card, and its permission requests on Kuchu, like any other session.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use crate::hub::{now, Hub};
use crate::sessions::step;

/// A conversation's process closes after this long without a turn; the next message starts it again.
const QUIET: Duration = Duration::from_secs(10 * 60);
/// Asked to stop, Claude Code has this long to end its turn before its process is ended.
const STOP_GRACE: Duration = Duration::from_secs(5);
/// A conversation is sent to the pages at most this often while Claude writes.
const SEND_EVERY: Duration = Duration::from_millis(60);
/// The end of what Claude Code wrote to stderr, kept for saying why it stopped.
const ERR_TAIL: usize = 2000;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Claude Code is starting, or working on a turn.
    Working,
    /// Waiting for the next message (its process open or not).
    Ready,
    /// Taken on in a terminal: only read here.
    Handed,
}

/// One thing in a conversation.
#[derive(Clone, Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Entry {
    /// A message from the person.
    You { text: String },
    /// Claude's text, growing while it streams.
    Claude { text: String },
    /// A tool Claude used, as one line.
    Step { text: String },
    /// What Kuchu says about the conversation: stopped, did not finish, taken on in a terminal.
    Note { text: String },
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    /// The Claude Code session's id.
    pub id: String,
    pub cwd: String,
    pub project: String,
    pub status: Status,
    pub entries: Vec<Entry>,
    pub updated: u64,
}

/// How a turn ended.
#[derive(Debug, PartialEq)]
enum Ended {
    Done,
    Stopped,
    Failed(String),
}

/// Where a line of Claude Code's output leaves the conversation.
#[derive(Default, Debug, PartialEq)]
struct Change {
    shown: bool,
    /// The session exists in Claude Code now (its first line came), so a later process resumes it.
    began: bool,
    ended: Option<Ended>,
}

/// Claude Code's stream, read into a conversation.
#[derive(Default)]
struct Reader {
    /// The last entry is Claude's text, still streaming.
    open: bool,
}

impl Reader {
    fn take(&mut self, chat: &mut Chat, o: &Value) -> Change {
        let mut change = Change::default();
        // A helper's own messages (an agent Claude started) stay out of the conversation.
        if o.get("parent_tool_use_id").is_some_and(|p| !p.is_null()) {
            return change;
        }
        let kind = o.get("type").and_then(Value::as_str).unwrap_or("");
        match kind {
            "system" if o.get("subtype").and_then(Value::as_str) == Some("init") => change.began = true,
            "stream_event" => {
                let event = &o["event"];
                match event.get("type").and_then(Value::as_str).unwrap_or("") {
                    "message_start" if chat.status == Status::Ready => {
                        chat.status = Status::Working;
                        change.shown = true;
                    }
                    "content_block_start" => self.open = false,
                    "content_block_delta" if event["delta"]["type"] == "text_delta" => {
                        let piece = event["delta"]["text"].as_str().unwrap_or("");
                        if !self.open {
                            chat.entries.push(Entry::Claude { text: String::new() });
                            self.open = true;
                        }
                        if let Some(Entry::Claude { text }) = chat.entries.last_mut() {
                            text.push_str(piece);
                        }
                        change.shown = true;
                    }
                    _ => {}
                }
            }
            "assistant" => {
                for block in o["message"]["content"].as_array().into_iter().flatten() {
                    match block.get("type").and_then(Value::as_str).unwrap_or("") {
                        "text" => {
                            let whole = block["text"].as_str().unwrap_or("").to_string();
                            // The whole text replaces what streamed in, so nothing is lost or doubled.
                            match chat.entries.last_mut() {
                                Some(Entry::Claude { text }) if self.open => *text = whole,
                                _ if !whole.trim().is_empty() => chat.entries.push(Entry::Claude { text: whole }),
                                _ => {}
                            }
                            self.open = false;
                            change.shown = true;
                        }
                        "tool_use" => {
                            let name = block["name"].as_str().unwrap_or("");
                            chat.entries.push(Entry::Step { text: step(name, &block["input"]) });
                            self.open = false;
                            change.shown = true;
                        }
                        _ => {}
                    }
                }
            }
            "result" => {
                self.open = false;
                let ended = if o["subtype"] == "success" {
                    Ended::Done
                } else if o["terminal_reason"] == "aborted_streaming" {
                    Ended::Stopped
                } else {
                    let why = o["result"].as_str().map(str::to_string).filter(|s| !s.trim().is_empty()).or_else(|| {
                        o["errors"].as_array().map(|e| e.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")).filter(|s| !s.is_empty())
                    });
                    Ended::Failed(why.unwrap_or_else(|| o["subtype"].as_str().unwrap_or("an error").replace('_', " ")))
                };
                match &ended {
                    Ended::Done => {}
                    Ended::Stopped => chat.entries.push(Entry::Note { text: "Stopped.".into() }),
                    Ended::Failed(why) => chat.entries.push(Entry::Note { text: format!("It did not finish: {why}") }),
                }
                chat.status = Status::Ready;
                change.shown = true;
                change.ended = Some(ended);
            }
            _ => {}
        }
        change
    }
}

/// A running Claude Code process for one conversation.
struct Run {
    /// Lines for its input; dropping this closes the input, and Claude Code finishes and exits.
    input: mpsc::UnboundedSender<String>,
    pid: u32,
    /// Which run this is, so a late word from an older process is not taken for this one's.
    nth: u64,
}

struct Held {
    chat: Chat,
    reader: Reader,
    run: Option<Run>,
    /// Claude Code has the session, so the next process resumes it.
    began: bool,
    /// When its last turn ended, for closing a quiet process.
    quiet_since: Instant,
    /// Kuchu asked Claude Code to stop this turn.
    stopping: bool,
    stderr: Arc<Mutex<String>>,
}

pub struct Chats {
    app: AppHandle,
    hub: Arc<Hub>,
    all: Mutex<HashMap<String, Held>>,
    /// Conversations changed and about to be sent to the pages.
    due: Mutex<HashSet<String>>,
    runs: std::sync::atomic::AtomicU64,
    /// Extra settings for Kuchu's own sessions (its hooks, when Claude Code's settings do not have them yet).
    pub settings: Box<dyn Fn() -> Option<PathBuf> + Send + Sync>,
}

impl Chats {
    pub fn new(app: AppHandle, hub: Arc<Hub>, settings: Box<dyn Fn() -> Option<PathBuf> + Send + Sync>) -> Arc<Chats> {
        Arc::new(Chats { app, hub, all: Mutex::new(HashMap::new()), due: Mutex::new(HashSet::new()), runs: 0.into(), settings })
    }

    pub fn list(&self) -> Vec<Chat> {
        let mut all: Vec<Chat> = self.all.lock().unwrap().values().map(|h| h.chat.clone()).collect();
        all.sort_by_key(|c| std::cmp::Reverse(c.updated));
        all
    }

    pub fn has(&self, id: &str) -> bool {
        self.all.lock().unwrap().contains_key(id)
    }

    /// A new conversation in `cwd`, starting with `text`. Returns its session id.
    pub fn start(self: &Arc<Self>, cwd: &str, text: &str) -> Result<String, String> {
        if !Path::new(cwd).is_dir() {
            return Err(format!("{cwd} is not a folder Kuchu can open."));
        }
        let id = new_id();
        let chat = Chat { id: id.clone(), cwd: cwd.to_string(), project: project_of(cwd), status: Status::Ready, entries: Vec::new(), updated: now() };
        let held = Held { chat, reader: Reader::default(), run: None, began: false, quiet_since: Instant::now(), stopping: false, stderr: Default::default() };
        self.all.lock().unwrap().insert(id.clone(), held);
        if let Err(why) = self.send(&id, text) {
            self.all.lock().unwrap().remove(&id);
            return Err(why);
        }
        Ok(id)
    }

    /// A message to a conversation: to its process, started (or started again) if it is not running.
    pub fn send(self: &Arc<Self>, id: &str, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        let mut all = self.all.lock().unwrap();
        let held = all.get_mut(id).ok_or("This conversation is gone.")?;
        if held.chat.status == Status::Handed {
            return Err("This conversation goes on in a terminal now.".into());
        }
        if held.run.is_none() {
            held.run = Some(self.spawn(id, &held.chat.cwd, held.began, held.stderr.clone())?);
        }
        let line = json!({ "type": "user", "message": { "role": "user", "content": text } }).to_string();
        let run = held.run.as_ref().expect("just started");
        run.input.send(line).map_err(|_| "Claude Code stopped before the message reached it. Send it again.".to_string())?;
        held.chat.entries.push(Entry::You { text: text.to_string() });
        held.chat.status = Status::Working;
        held.chat.updated = now();
        held.stopping = false;
        drop(all);
        self.changed(id);
        Ok(())
    }

    /// Stops the turn under way: Claude Code is asked first, and its process ended if it does not stop in time.
    pub fn stop(self: &Arc<Self>, id: &str) {
        let mut all = self.all.lock().unwrap();
        let Some(held) = all.get_mut(id) else { return };
        let Some(run) = &held.run else { return };
        if held.chat.status != Status::Working {
            return;
        }
        held.stopping = true;
        let ask = json!({ "type": "control_request", "request_id": format!("kuchu-stop-{}", now()), "request": { "subtype": "interrupt" } });
        let _ = run.input.send(ask.to_string());
        let (pid, nth) = (run.pid, run.nth);
        drop(all);
        let chats = self.clone();
        let id = id.to_string();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(STOP_GRACE).await;
            let still = chats.all.lock().unwrap().get(&id).is_some_and(|h| h.chat.status == Status::Working && h.run.as_ref().is_some_and(|r| r.nth == nth));
            if still {
                end_process(pid);
            }
        });
    }

    /// The conversation goes on in a terminal: Kuchu lets its process go and opens `claude --resume` in the folder.
    pub fn hand_off(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let mut all = self.all.lock().unwrap();
        let held = all.get_mut(id).ok_or("This conversation is gone.")?;
        if held.chat.status == Status::Working {
            return Err("Claude is still working. Wait for it, or stop it first.".into());
        }
        if !held.began {
            return Err("Claude Code has not started this conversation yet.".into());
        }
        let claude = find_claude().ok_or(NO_CLAUDE)?;
        open_terminal(&claude, &held.chat.cwd, id)?;
        // Its input closed, Kuchu's process ends; the terminal has the session from here on.
        held.run = None;
        held.chat.status = Status::Handed;
        held.chat.entries.push(Entry::Note { text: "Goes on in a terminal.".into() });
        held.chat.updated = now();
        drop(all);
        self.changed(id);
        Ok(())
    }

    /// Forgets a conversation; a turn under way is ended with it.
    pub fn forget(&self, id: &str) -> bool {
        let Some(held) = self.all.lock().unwrap().remove(id) else { return false };
        if let Some(run) = held.run.filter(|_| held.chat.status == Status::Working) {
            end_process(run.pid);
        }
        let _ = self.app.emit("kuchu://chat-gone", id);
        true
    }

    /// Processes quiet for a while are let go; the next message starts the conversation again.
    pub fn expire(&self) {
        for held in self.all.lock().unwrap().values_mut() {
            if held.run.is_some() && held.chat.status == Status::Ready && held.quiet_since.elapsed() >= QUIET {
                held.run = None;
            }
        }
    }

    fn spawn(self: &Arc<Self>, id: &str, cwd: &str, resume: bool, stderr: Arc<Mutex<String>>) -> Result<Run, String> {
        let claude = find_claude().ok_or(NO_CLAUDE)?;
        let mut cmd = tokio::process::Command::new(&claude);
        cmd.args(["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose", "--include-partial-messages"]);
        cmd.args([if resume { "--resume" } else { "--session-id" }, id]);
        if let Some(settings) = (self.settings)() {
            cmd.arg("--settings").arg(settings);
        }
        // Started from inside a Claude Code session, Kuchu passes on its marks; this session is its own.
        cmd.env_remove("CLAUDECODE").env_remove("CLAUDE_CODE_ENTRYPOINT");
        cmd.current_dir(cwd).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        #[cfg(unix)]
        cmd.process_group(0);
        let mut child = cmd.spawn().map_err(|e| format!("Kuchu could not start Claude Code ({}): {e}", claude.display()))?;
        let pid = child.id().unwrap_or(0);
        let nth = self.runs.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        let (input, mut lines_in) = mpsc::unbounded_channel::<String>();

        let mut stdin = child.stdin.take().expect("piped");
        tauri::async_runtime::spawn(async move {
            while let Some(line) = lines_in.recv().await {
                if stdin.write_all(format!("{line}\n").as_bytes()).await.is_err() || stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        let mut err = child.stderr.take().expect("piped");
        tauri::async_runtime::spawn(async move {
            let mut buf = [0u8; 1024];
            while let Ok(n) = err.read(&mut buf).await {
                if n == 0 {
                    break;
                }
                let mut tail = stderr.lock().unwrap();
                tail.push_str(&String::from_utf8_lossy(&buf[..n]));
                if tail.len() > ERR_TAIL {
                    let cut = tail.len() - ERR_TAIL;
                    let cut = (cut..tail.len()).find(|&i| tail.is_char_boundary(i)).unwrap_or(tail.len());
                    tail.drain(..cut);
                }
            }
        });

        let out = child.stdout.take().expect("piped");
        let chats = self.clone();
        let id = id.to_string();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(o) = serde_json::from_str::<Value>(&line) {
                    chats.take(&id, nth, &o);
                }
            }
            let code = child.wait().await.ok().and_then(|s| s.code());
            chats.exited(&id, nth, code);
        });
        Ok(Run { input, pid, nth })
    }

    /// One line from a conversation's process.
    fn take(self: &Arc<Self>, id: &str, nth: u64, o: &Value) {
        let mut all = self.all.lock().unwrap();
        let Some(held) = all.get_mut(id) else { return };
        if held.run.as_ref().is_some_and(|r| r.nth != nth) {
            return;
        }
        let change = held.reader.take(&mut held.chat, o);
        held.began |= change.began;
        if change.ended.is_some() {
            held.quiet_since = Instant::now();
            held.stopping = false;
        }
        if !change.shown {
            return;
        }
        held.chat.updated = now();
        drop(all);
        // A stopped or failed turn sends no Stop to the hooks: the session is not busy any more.
        if matches!(change.ended, Some(Ended::Stopped | Ended::Failed(_))) {
            self.hub.stopped(id);
        }
        self.changed(id);
    }

    /// A conversation's process is gone: if it was in a turn, the conversation says so.
    fn exited(self: &Arc<Self>, id: &str, nth: u64, code: Option<i32>) {
        let mut all = self.all.lock().unwrap();
        let Some(held) = all.get_mut(id) else { return };
        if held.run.as_ref().is_some_and(|r| r.nth != nth) {
            return;
        }
        held.run = None;
        held.reader = Reader::default();
        if held.chat.status != Status::Working {
            return;
        }
        let note = if held.stopping {
            "Stopped.".to_string()
        } else {
            let tail = held.stderr.lock().unwrap().trim().lines().last().unwrap_or("").to_string();
            match (tail.is_empty(), code) {
                (false, _) => format!("Claude Code stopped: {tail}"),
                (true, Some(c)) => format!("Claude Code stopped (exit code {c})."),
                (true, None) => "Claude Code stopped.".to_string(),
            }
        };
        held.chat.entries.push(Entry::Note { text: note });
        held.chat.status = Status::Ready;
        held.chat.updated = now();
        held.stopping = false;
        drop(all);
        self.hub.stopped(id);
        self.changed(id);
    }

    /// Sends a conversation to the pages, gathering the changes of the next moment into one.
    fn changed(self: &Arc<Self>, id: &str) {
        if !self.due.lock().unwrap().insert(id.to_string()) {
            return;
        }
        let chats = self.clone();
        let id = id.to_string();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(SEND_EVERY).await;
            chats.due.lock().unwrap().remove(&id);
            let chat = chats.all.lock().unwrap().get(&id).map(|h| h.chat.clone());
            if let Some(chat) = chat {
                let _ = chats.app.emit("kuchu://chat", chat);
            }
        });
    }
}

const NO_CLAUDE: &str = "Kuchu could not find Claude Code on this computer. Install it, then try again.";

/// Where Claude Code is: on the PATH, else where its installers put it.
pub fn find_claude() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) { &["claude.exe", "claude.cmd"] } else { &["claude"] };
    let path = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        if let Some(found) = names.iter().map(|n| dir.join(n)).find(|p| p.is_file()) {
            return Some(found);
        }
    }
    let home = PathBuf::from(std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?);
    let usual: Vec<PathBuf> = if cfg!(windows) {
        vec![home.join(".local").join("bin").join("claude.exe"), home.join("AppData").join("Roaming").join("npm").join("claude.cmd")]
    } else {
        vec![home.join(".local/bin/claude"), home.join(".claude/local/claude"), "/opt/homebrew/bin/claude".into(), "/usr/local/bin/claude".into()]
    };
    usual.into_iter().find(|p| p.is_file())
}

/// Ends a conversation's process and everything it started (its tools' processes too).
fn end_process(pid: u32) {
    if pid == 0 {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).creation_flags(CREATE_NO_WINDOW).status();
    }
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill").args(["-TERM", &format!("-{pid}")]).status();
    }
}

/// Opens a terminal in `cwd` with Claude Code taking up session `id`.
fn open_terminal(claude: &Path, cwd: &str, id: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Through `start`, so Claude Code gets the new console's own input and output (a process Kuchu starts directly
        // would get Kuchu's, find no terminal, and quit). Windows opens it in the person's default terminal.
        std::process::Command::new("cmd")
            .raw_arg(format!("/d /c start \"\" /d \"{cwd}\" \"{}\" --resume {id}", claude.display()))
            .env_remove("CLAUDECODE")
            .env_remove("CLAUDE_CODE_ENTRYPOINT")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(drop)
            .map_err(|e| format!("Kuchu could not open a terminal: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        let quote = |s: &str| format!("'{}'", s.replace('\'', r"'\''"));
        let line = format!("cd {} && {} --resume {}", quote(cwd), quote(&claude.display().to_string()), id);
        let script = format!("tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell", line.replace('\\', "\\\\").replace('"', "\\\""));
        std::process::Command::new("osascript").args(["-e", &script]).spawn().map(drop).map_err(|e| format!("Kuchu could not open Terminal: {e}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("x-terminal-emulator")
            .arg("-e")
            .arg(claude)
            .args(["--resume", id])
            .current_dir(cwd)
            .spawn()
            .map(drop)
            .map_err(|e| format!("Kuchu could not open a terminal: {e}"))
    }
}

/// A new session id (a version 4 UUID). The randomness is the standard library's per-process hash keys, which is
/// enough for an id that must only be new.
fn new_id() -> String {
    let random = || {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        h.finish()
    };
    let mut b = [0u8; 16];
    b[..8].copy_from_slice(&random().to_le_bytes());
    b[8..].copy_from_slice(&random().to_le_bytes());
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

fn project_of(cwd: &str) -> String {
    cwd.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().filter(|s| !s.is_empty()).unwrap_or("Claude Code").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> Chat {
        Chat { id: "a".into(), cwd: String::new(), project: String::new(), status: Status::Working, entries: Vec::new(), updated: 0 }
    }

    fn replay(name: &str) -> (Chat, Vec<Change>) {
        let path = format!("{}/../../fixtures/claude-code-2.1.310/stream/{name}.jsonl", env!("CARGO_MANIFEST_DIR"));
        let (mut chat, mut reader) = (blank(), Reader::default());
        let text = std::fs::read_to_string(path).unwrap();
        let changes = text.lines().map(|l| reader.take(&mut chat, &serde_json::from_str(l).unwrap())).collect();
        (chat, changes)
    }

    fn ends(changes: &[Change]) -> Vec<&Ended> {
        changes.iter().filter_map(|c| c.ended.as_ref()).collect()
    }

    #[test]
    fn a_recorded_conversation_reads_as_its_steps_and_its_answer() {
        // Two messages sent at once: Claude Code takes the second into the same turn.
        let (chat, changes) = replay("two-messages");
        assert!(changes.iter().any(|c| c.began));
        assert_eq!(chat.entries, vec![Entry::Step { text: "Reading note.txt".into() }, Entry::Claude { text: "bye".into() }]);
        assert_eq!(chat.status, Status::Ready);
        assert_eq!(ends(&changes), vec![&Ended::Done]);
    }

    #[test]
    fn a_stopped_turn_says_so_and_the_next_message_is_answered() {
        let (chat, changes) = replay("interrupted");
        assert_eq!(ends(&changes), vec![&Ended::Stopped, &Ended::Done]);
        let Some(Entry::Claude { text }) = chat.entries.first() else { panic!("{:?}", chat.entries) };
        assert!(text.starts_with("one"), "{text}");
        assert_eq!(&chat.entries[1..], &[Entry::Note { text: "Stopped.".into() }, Entry::Claude { text: "still here".into() }]);
    }

    #[test]
    fn streamed_text_is_replaced_by_the_whole_never_doubled() {
        let (mut chat, mut r) = (blank(), Reader::default());
        let delta = |t: &str| json!({ "type": "stream_event", "event": { "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": t } } });
        r.take(&mut chat, &json!({ "type": "stream_event", "event": { "type": "content_block_start", "index": 0, "content_block": { "type": "text" } } }));
        r.take(&mut chat, &delta("Hel"));
        r.take(&mut chat, &delta("lo"));
        assert_eq!(chat.entries, vec![Entry::Claude { text: "Hello".into() }]);
        r.take(&mut chat, &json!({ "type": "assistant", "message": { "content": [{ "type": "text", "text": "Hello there" }] } }));
        // Text that never streamed (no partial messages) still shows once.
        r.take(&mut chat, &json!({ "type": "assistant", "message": { "content": [{ "type": "text", "text": "More" }] } }));
        assert_eq!(chat.entries, vec![Entry::Claude { text: "Hello there".into() }, Entry::Claude { text: "More".into() }]);
    }

    #[test]
    fn a_helpers_messages_stay_out_and_a_failed_turn_says_why() {
        let (mut chat, mut r) = (blank(), Reader::default());
        let helper = json!({ "type": "assistant", "parent_tool_use_id": "toolu_1", "message": { "content": [{ "type": "text", "text": "inside" }] } });
        assert_eq!(r.take(&mut chat, &helper), Change::default());
        let failed = json!({ "type": "result", "subtype": "error_max_turns", "is_error": true });
        assert_eq!(r.take(&mut chat, &failed).ended, Some(Ended::Failed("error max turns".into())));
        assert_eq!(chat.entries, vec![Entry::Note { text: "It did not finish: error max turns".into() }]);
        assert_eq!(chat.status, Status::Ready);
    }

    #[test]
    fn ids_are_new_version_4_uuids() {
        let (a, b) = (new_id(), new_id());
        assert_ne!(a, b);
        assert_eq!(a.len(), 36);
        assert_eq!(&a[14..15], "4");
        assert!(matches!(&a[19..20], "8" | "9" | "a" | "b"), "{a}");
    }
}
