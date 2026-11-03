//! Where the relay's events arrive: the pipe (a socket on macOS and Linux), the sessions they update, and permission
//! requests waiting for a click. Every change is sent to the windows as one view.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kuchu_wire::Pipe;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{oneshot, Semaphore};

use crate::alive::Running;
use crate::locks::Hold;
use crate::sessions::{step, tail_of, Card, Mood, Session, Sessions};

/// At most this many requests wait at once; more go straight back to the terminal.
const MAX_WAITING: usize = 5;
/// How long reading the end of a session's transcript may take before the event goes on without it.
const TAIL_WAIT: Duration = Duration::from_millis(500);
/// At most this many connections are taken in at once (each costs a task and up to a line of 1 MiB); more are closed
/// unread, so a flood of connections cannot tie Kuchu up. Real hooks come one or a few at a time per session.
const MAX_CONNECTIONS: usize = 64;

/// A permission request as a card shows it.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: u64,
    pub session: String,
    /// What the session is called.
    pub project: String,
    /// "permission", "question" or "plan".
    pub kind: String,
    /// What it wants to do, as one line.
    pub title: String,
    /// The command, file or address it is about.
    pub detail: String,
    /// The label for Claude Code's own "don't ask again" offer, when it made one Kuchu can pass on.
    pub always: Option<String>,
    /// A question with one choice to make: its text and options.
    pub question: Option<Question>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Question {
    pub text: String,
    pub options: Vec<String>,
}

/// What a window shows.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub mood: Mood,
    pub card: Option<Card>,
    pub approvals: Vec<Approval>,
    pub sessions: Vec<Session>,
}

/// A click on a card.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    /// "allow", "deny", "always", "answer" or "terminal".
    pub kind: String,
    pub answer: Option<String>,
}

struct Waiting {
    card: Approval,
    request: Value,
    shown: Option<oneshot::Sender<()>>,
    decide: Option<oneshot::Sender<Option<Value>>>,
}

#[derive(Default)]
struct Inner {
    sessions: Sessions,
    waiting: Vec<Waiting>,
    next: u64,
}

pub struct Hub {
    app: AppHandle,
    inner: Mutex<Inner>,
    /// Whether permission requests are shown at all (Settings).
    pub approvals_on: std::sync::atomic::AtomicBool,
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Hub {
    pub fn new(app: AppHandle, approvals_on: bool) -> Arc<Hub> {
        Arc::new(Hub { app, inner: Mutex::new(Inner::default()), approvals_on: approvals_on.into() })
    }

    pub fn view(&self) -> View {
        let inner = self.inner.hold();
        View {
            mood: inner.sessions.mood(),
            card: inner.sessions.card(),
            approvals: inner.waiting.iter().map(|w| w.card.clone()).collect(),
            sessions: inner.sessions.list(),
        }
    }

    fn publish(&self) {
        let _ = self.app.emit("kuchu://view", self.view());
    }

    /// An event, with the end of its session's transcript as read beforehand (`tail_for`).
    pub fn event(&self, event: &str, payload: &Value, tail: Option<&str>) {
        let mut inner = self.inner.hold();
        // A request answered in Claude Code's own window: its wait ends with no answer, and its card goes.
        for w in inner.waiting.iter_mut().filter(|w| answered_elsewhere(event, payload, &w.request)) {
            w.shown.take();
            w.decide.take();
        }
        let changed = inner.sessions.apply_with(event, payload, now(), tail);
        drop(inner);
        if changed {
            self.publish();
        }
        if let Some(tone) = tone_for(event, payload) {
            let _ = self.app.emit("kuchu://tone", tone);
        }
    }

    pub fn seen(&self) {
        if self.inner.hold().sessions.seen(now()) {
            self.publish();
        }
    }

    pub fn stopped(&self, session: &str) {
        if self.inner.hold().sessions.stopped(session, now()) {
            self.publish();
        }
    }

    /// Busy sessions silent for `quiet` ms: worth asking Claude Code whether they still run.
    pub fn silent(&self, quiet: u64) -> Vec<String> {
        self.inner.hold().sessions.silent(now(), quiet)
    }

    /// Claude Code's answer about the sessions asked: those it no longer runs go, those it says are idle stop.
    pub fn check_running(&self, asked: &[String], running: &[Running], quiet: u64) {
        if self.inner.hold().sessions.check_running(asked, running, now(), quiet) {
            self.publish();
        }
    }

    /// The card switched to `session`, or back to the latest.
    pub fn follow(&self, session: Option<String>) {
        if self.inner.hold().sessions.follow(session) {
            self.publish();
        }
    }

    pub fn forget(&self, session: &str) {
        if self.inner.hold().sessions.forget(session) {
            self.publish();
        }
    }

    pub fn expire(&self) {
        if self.inner.hold().sessions.expire(now()) {
            self.publish();
        }
    }

    /// The card for request `id` is on the screen.
    pub fn shown(&self, id: u64) {
        let mut inner = self.inner.hold();
        if let Some(tx) = inner.waiting.iter_mut().find(|w| w.card.id == id).and_then(|w| w.shown.take()) {
            let _ = tx.send(());
        }
    }

    /// A click on request `id`'s card, as the decision Claude Code reads (none: the terminal asks).
    pub fn decide(&self, id: u64, choice: Choice) {
        let mut inner = self.inner.hold();
        let Some(w) = inner.waiting.iter_mut().find(|w| w.card.id == id) else { return };
        let decision = decision(&w.request, &choice);
        if let Some(tx) = w.decide.take() {
            let _ = tx.send(decision);
        }
    }

    /// A permission request arrives: shown as a card, unless approvals are off or too many wait.
    fn register(&self, request: &Value, tail: Option<&str>) -> Option<(u64, oneshot::Receiver<()>, oneshot::Receiver<Option<Value>>)> {
        if !self.approvals_on.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let mut inner = self.inner.hold();
        if inner.waiting.len() >= MAX_WAITING {
            return None;
        }
        inner.next += 1;
        let id = inner.next;
        inner.sessions.apply_with("PermissionRequest", request, now(), tail);
        let session = request.get("session_id").and_then(Value::as_str).unwrap_or("").to_string();
        let project = inner.sessions.name(&session).unwrap_or_else(|| "Claude Code".into());
        let (shown_tx, shown_rx) = oneshot::channel();
        let (decide_tx, decide_rx) = oneshot::channel();
        inner.waiting.push(Waiting { card: card(id, session, project, request), request: request.clone(), shown: Some(shown_tx), decide: Some(decide_tx) });
        drop(inner);
        self.publish();
        let _ = self.app.emit("kuchu://tone", Mood::Needs);
        Some((id, shown_rx, decide_rx))
    }

    /// The request is answered, gone or timed out: its card goes, and its session works again.
    fn finish(&self, id: u64) {
        let mut inner = self.inner.hold();
        let Some(at) = inner.waiting.iter().position(|w| w.card.id == id) else { return };
        let w = inner.waiting.remove(at);
        if !inner.waiting.iter().any(|o| o.card.session == w.card.session) {
            inner.sessions.resumed(&w.card.session);
        }
        drop(inner);
        self.publish();
    }

    /// One relay's connection: an event to take in, or a permission request to answer.
    async fn handle<S: AsyncRead + AsyncWrite + Unpin>(self: Arc<Self>, stream: S) {
        let (r, mut w) = tokio::io::split(stream);
        let mut reader = BufReader::new(r);
        let mut line = Vec::new();
        let mut limited = (&mut reader).take(kuchu_wire::MAX_LINE as u64);
        let read = limited.read_until(b'\n', &mut line);
        if !matches!(tokio::time::timeout(kuchu_wire::EVENT, read).await, Ok(Ok(n)) if n > 0) {
            return;
        }
        let Ok(message) = serde_json::from_slice::<Value>(&line) else { return };
        let Some((event, payload)) = accepted(&message) else { return };
        let tail = tail_for(&payload).await;
        if event != "PermissionRequest" {
            self.event(event, &payload, tail.as_deref());
            return;
        }
        let Some((id, shown, decide)) = self.register(&payload, tail.as_deref()) else { return };
        let answer = tokio::select! {
            answer = wait(shown, decide) => answer,
            // The relay went away (Claude Code stopped waiting): the card goes at once.
            _ = gone(&mut reader) => None,
        };
        self.finish(id);
        if let Some(decision) = answer {
            let _ = w.write_all(format!("{decision}\n").as_bytes()).await;
            let _ = w.flush().await;
        }
    }
}

/// The end of the session's transcript the payload names, read on a thread of its own and within `TAIL_WAIT`, before
/// any lock is taken: a file that does not answer costs this one event its words, not Kuchu.
async fn tail_for(payload: &Value) -> Option<String> {
    let path = payload.get("transcript_path").and_then(Value::as_str)?.to_string();
    tokio::time::timeout(TAIL_WAIT, tokio::task::spawn_blocking(move || tail_of(&path))).await.ok()?.ok()?
}

/// The event and payload of a message in Kuchu's own protocol; none for anything else, which is dropped unread.
fn accepted(message: &Value) -> Option<(&str, Value)> {
    if message.get("v").and_then(Value::as_u64) != Some(kuchu_wire::VERSION as u64) {
        return None;
    }
    let event = message.get("event")?.as_str()?;
    Some((event, message.get("payload").cloned().unwrap_or(Value::Null)))
}

/// The tone an event rings, for each session on its own (the pet's mood is everyone's: one session done while another
/// works would never ring): done when a turn ends, failed when it could not finish, needs when Claude Code waits on a
/// permission prompt. A request shown on Kuchu rings needs as it is registered.
fn tone_for(event: &str, payload: &Value) -> Option<Mood> {
    match event {
        "Stop" => Some(Mood::Done),
        "StopFailure" => Some(Mood::Failed),
        "Notification" if payload.get("notification_type").and_then(Value::as_str) == Some("permission_prompt") => Some(Mood::Needs),
        _ => None,
    }
}

/// Whether `event` shows that Claude Code no longer waits on `request`: the call ran or failed (answered in Claude
/// Code's own window), or the turn is over. The relay's connection can stay open after that, and the card with it.
/// The call is matched by its step's line, which the relay's cutting of long text leaves whole.
fn answered_elsewhere(event: &str, payload: &Value, request: &Value) -> bool {
    let text = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let call = |v: &Value| (text(v, "tool_name"), step(&text(v, "tool_name"), v.get("tool_input").unwrap_or(&Value::Null)));
    if text(payload, "session_id") != text(request, "session_id") {
        return false;
    }
    match event {
        "PostToolUse" | "PostToolUseFailure" => call(payload) == call(request),
        "UserPromptSubmit" | "Stop" | "StopFailure" | "SessionEnd" => true,
        _ => false,
    }
}

/// The card must be on screen within `SHOWN`, then a click within `DECIDE`; anything else leaves it to the terminal.
async fn wait(shown: oneshot::Receiver<()>, decide: oneshot::Receiver<Option<Value>>) -> Option<Value> {
    if !matches!(tokio::time::timeout(kuchu_wire::SHOWN, shown).await, Ok(Ok(()))) {
        return None;
    }
    tokio::time::timeout(kuchu_wire::DECIDE, decide).await.ok()?.ok()?
}

/// Ends when the other side closes the connection (or sends what it never should).
async fn gone<R: AsyncRead + Unpin>(reader: &mut R) {
    let mut byte = [0u8; 1];
    let _ = reader.read(&mut byte).await;
}

fn card(id: u64, session: String, project: String, request: &Value) -> Approval {
    let tool = request.get("tool_name").and_then(Value::as_str).unwrap_or("");
    let input = request.get("tool_input").cloned().unwrap_or(Value::Null);
    let s = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let detail = match tool {
        "Bash" | "PowerShell" => s("command"),
        "Write" | "Edit" | "MultiEdit" | "Read" => s("file_path"),
        "NotebookEdit" => s("notebook_path"),
        "WebFetch" => s("url"),
        "WebSearch" => s("query"),
        "ExitPlanMode" => s("plan"),
        _ => match &input {
            Value::Null => String::new(),
            other => other.to_string(),
        },
    };
    let detail: String = detail.chars().take(600).collect();
    let (kind, question) = match tool {
        "AskUserQuestion" => ("question", single_question(&input)),
        "ExitPlanMode" => ("plan", None),
        _ => ("permission", None),
    };
    Approval {
        id,
        session,
        project,
        kind: kind.into(),
        title: step(tool, &input),
        detail,
        always: if kind == "permission" { always_label(request) } else { None },
        question,
    }
}

/// A question Kuchu can answer with one click: one question, one choice.
fn single_question(input: &Value) -> Option<Question> {
    let questions = input.get("questions")?.as_array()?;
    let [q] = questions.as_slice() else { return None };
    if q.get("multiSelect").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }
    let options: Vec<String> =
        q.get("options")?.as_array()?.iter().filter_map(|o| o.get("label").and_then(Value::as_str).map(str::to_string)).collect();
    (!options.is_empty()).then(|| Question { text: q.get("question").and_then(Value::as_str).unwrap_or("").to_string(), options })
}

/// Claude Code's first "don't ask again" offer, named for a button; none for an offer to stop asking altogether.
fn always_label(request: &Value) -> Option<String> {
    let first = request.get("permission_suggestions")?.as_array()?.first()?;
    match first.get("type").and_then(Value::as_str)? {
        "addRules" => Some("Always allow".into()),
        "addDirectories" => Some("Allow this folder".into()),
        "setMode" => match first.get("mode").and_then(Value::as_str)? {
            "acceptEdits" => Some("Allow edits for now".into()),
            _ => None,
        },
        _ => None,
    }
}

/// The decision Claude Code reads for a click; none leaves the question to the terminal.
fn decision(request: &Value, choice: &Choice) -> Option<Value> {
    let tool = request.get("tool_name").and_then(Value::as_str).unwrap_or("");
    let input = request.get("tool_input").cloned().unwrap_or(Value::Null);
    match choice.kind.as_str() {
        // A plan is approved only with the plan sent back.
        "allow" if tool == "ExitPlanMode" => Some(json!({ "behavior": "allow", "updatedInput": input })),
        "allow" => Some(json!({ "behavior": "allow" })),
        "deny" => Some(json!({ "behavior": "deny", "message": "Declined from Kuchu." })),
        "always" => {
            always_label(request)?;
            let first = request.get("permission_suggestions")?.as_array()?.first()?.clone();
            Some(json!({ "behavior": "allow", "updatedPermissions": [first] }))
        }
        "answer" => {
            let question = single_question(&input)?;
            let answer = choice.answer.clone().filter(|a| question.options.contains(a))?;
            let mut updated = input.as_object()?.clone();
            updated.insert("answers".into(), json!({ question.text: answer }));
            Some(json!({ "behavior": "allow", "updatedInput": updated }))
        }
        _ => None,
    }
}

/// Listens for relays for as long as Kuchu runs. A failed connection never stops it. Only a process of this account
/// is heard, and only so many at once; any other connection is closed unread, which leaves its request to the
/// terminal.
pub async fn serve(hub: Arc<Hub>) -> Result<(), String> {
    let permits = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use tokio::net::windows::named_pipe::ServerOptions;
        let name = kuchu_wire::address();
        // Made for this account alone: no other account can open it, send it events, or hold the name's next instance.
        let only_me = kuchu_wire::OnlyMe::new().ok_or("Kuchu could not read which account it runs as.")?;
        let make = |first: bool| {
            let mut options = ServerOptions::new();
            options.first_pipe_instance(first).reject_remote_clients(true);
            // Safety: the attributes point into `only_me`, which outlives the call.
            only_me.with_attributes(|attributes| unsafe { options.create_with_security_attributes_raw(&name, attributes) })
        };
        let mut server = make(true).map_err(|_| taken())?;
        loop {
            if server.connect().await.is_err() {
                if let Ok(next) = make(false) {
                    server = next;
                }
                continue;
            }
            let next = loop {
                match make(false) {
                    Ok(next) => break next,
                    Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                }
            };
            let connected = std::mem::replace(&mut server, next);
            let me = kuchu_wire::client_is_me(connected.as_raw_handle());
            take_in(&hub, &permits, connected, me);
        }
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::io::AsRawFd;
        let path = std::path::PathBuf::from(kuchu_wire::address());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        if matches!(kuchu_wire::open(Duration::ZERO), Pipe::Kuchu(_) | Pipe::Busy) {
            return Err(taken());
        }
        let _ = std::fs::remove_file(&path);
        let listener = tokio::net::UnixListener::bind(&path).map_err(|e| e.to_string())?;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let me = kuchu_wire::client_is_me(stream.as_raw_fd());
                take_in(&hub, &permits, stream, me);
            }
        }
    }
}

/// A connection just made: heard if it is this account's (`me`) and there is room, else closed unread.
fn take_in<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(hub: &Arc<Hub>, permits: &Arc<Semaphore>, stream: S, me: bool) {
    if !me {
        return;
    }
    let Ok(permit) = permits.clone().try_acquire_owned() else { return };
    let hub = hub.clone();
    tokio::spawn(async move {
        let _permit = permit;
        hub.handle(stream).await;
    });
}

/// Why the pipe could not be made, for the person.
fn taken() -> String {
    match kuchu_wire::open(Duration::ZERO) {
        Pipe::Kuchu(_) | Pipe::Busy => "Another Kuchu is already running, or another program of this account holds its pipe.".into(),
        Pipe::Someone => "Another account's program holds Kuchu's pipe. No session can reach Kuchu until it is gone.".into(),
        Pipe::Nobody => "Kuchu could not open its pipe.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/../../fixtures/claude-code-2.1.310/{name}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn choice(kind: &str) -> Choice {
        Choice { kind: kind.into(), answer: None }
    }

    #[test]
    fn each_session_rings_its_own_tone() {
        assert_eq!(tone_for("Stop", &fixture("Stop")), Some(Mood::Done));
        assert_eq!(tone_for("StopFailure", &json!({ "session_id": "a" })), Some(Mood::Failed));
        assert_eq!(tone_for("Notification", &json!({ "notification_type": "permission_prompt" })), Some(Mood::Needs));
        assert_eq!(tone_for("Notification", &json!({ "notification_type": "idle_prompt" })), None);
        assert_eq!(tone_for("PreToolUse", &fixture("PreToolUse")), None);
    }

    #[tokio::test]
    async fn the_transcript_is_read_before_the_lock_or_not_at_all() {
        let dir = std::env::temp_dir().join(format!("kuchu-hub-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("s.jsonl");
        std::fs::write(&file, "{\"type\":\"ai-title\",\"aiTitle\":\"Named\"}\n").unwrap();
        assert!(tail_for(&json!({ "transcript_path": file.display().to_string() })).await.unwrap().contains("Named"));
        assert!(tail_for(&json!({ "transcript_path": dir.display().to_string() })).await.is_none());
        assert!(tail_for(&json!({ "transcript_path": 7 })).await.is_none());
        assert!(tail_for(&json!({})).await.is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_the_protocols_own_messages_are_heard() {
        let message = json!({ "v": 1, "event": "Stop", "payload": { "session_id": "a" } });
        let (event, payload) = accepted(&message).unwrap();
        assert_eq!((event, payload["session_id"].as_str()), ("Stop", Some("a")));
        assert!(accepted(&json!({ "v": 2, "event": "Stop", "payload": {} })).is_none());
        assert!(accepted(&json!({ "v": "1", "event": "Stop", "payload": {} })).is_none());
        assert!(accepted(&json!({ "event": "Stop", "payload": {} })).is_none());
        assert!(accepted(&json!({ "v": 1, "payload": {} })).is_none());
        assert!(accepted(&json!([1, "Stop"])).is_none());
    }

    #[test]
    fn a_recorded_request_reads_as_a_card() {
        let request = fixture("PermissionRequest");
        let c = card(1, "s".into(), "demo".into(), &request);
        assert_eq!(c.kind, "permission");
        assert_eq!(c.title, "Writing note.txt");
        assert!(c.detail.ends_with("note.txt"));
        assert_eq!(c.always.as_deref(), Some("Allow edits for now"));
        // A read outside the session's folder: Claude Code offers a rule, not a mode.
        let read = card(2, "s".into(), "demo".into(), &fixture("PermissionRequest-2"));
        assert_eq!((read.title.as_str(), read.always.as_deref()), ("Reading compact", Some("Always allow")));
    }

    #[test]
    fn each_click_is_the_decision_claude_code_reads() {
        let request = fixture("PermissionRequest");
        assert_eq!(decision(&request, &choice("allow")), Some(json!({ "behavior": "allow" })));
        assert_eq!(decision(&request, &choice("deny")).unwrap()["behavior"], "deny");
        assert_eq!(decision(&request, &choice("terminal")), None);
        let always = decision(&request, &choice("always")).unwrap();
        assert_eq!(always["updatedPermissions"][0]["mode"], "acceptEdits");
    }

    #[test]
    fn a_card_goes_once_its_request_is_answered_in_claude_code() {
        let request = fixture("PermissionRequest");
        let mut ran = request.clone();
        ran["hook_event_name"] = json!("PostToolUse");
        // The file's text as the relay sends it in an ordinary event: cut.
        ran["tool_input"]["content"] = json!("cut…");
        assert!(answered_elsewhere("PostToolUse", &ran, &request));
        assert!(answered_elsewhere("PostToolUseFailure", &ran, &request));
        assert!(answered_elsewhere("Stop", &json!({ "session_id": request["session_id"] }), &request));
        // Another call, another session, or a call only starting: the card stays.
        let mut other = ran.clone();
        other["tool_input"]["file_path"] = json!("other.txt");
        assert!(!answered_elsewhere("PostToolUse", &other, &request));
        let mut elsewhere = ran.clone();
        elsewhere["session_id"] = json!("another");
        assert!(!answered_elsewhere("PostToolUse", &elsewhere, &request));
        assert!(!answered_elsewhere("PreToolUse", &ran, &request));
    }

    #[test]
    fn a_plan_is_approved_with_the_plan_sent_back() {
        let request = json!({ "tool_name": "ExitPlanMode", "tool_input": { "plan": "1. Do it" } });
        assert_eq!(decision(&request, &choice("allow")).unwrap()["updatedInput"]["plan"], "1. Do it");
    }

    #[test]
    fn a_question_is_answered_with_one_of_its_options_only() {
        let request = json!({
            "tool_name": "AskUserQuestion",
            "tool_input": { "questions": [{ "question": "Which colour?", "header": "Colour", "multiSelect": false,
                "options": [{ "label": "Blue", "description": "" }, { "label": "Green", "description": "" }] }] }
        });
        let pick = |a: &str| decision(&request, &Choice { kind: "answer".into(), answer: Some(a.into()) });
        let d = pick("Green").unwrap();
        assert_eq!(d["updatedInput"]["answers"]["Which colour?"], "Green");
        assert_eq!(d["updatedInput"]["questions"][0]["header"], "Colour");
        assert_eq!(pick("Red"), None);
    }
}
