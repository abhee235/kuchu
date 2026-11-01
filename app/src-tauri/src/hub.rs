//! Where the relay's events arrive: the pipe (a socket on macOS and Linux), the sessions they update, and permission
//! requests waiting for a click. Every change is sent to the windows as one view.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;

use crate::alive::Running;
use crate::sessions::{step, Card, Mood, Session, Sessions};

/// At most this many requests wait at once; more go straight back to the terminal.
const MAX_WAITING: usize = 5;

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
        let inner = self.inner.lock().unwrap();
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

    pub fn event(&self, event: &str, payload: &Value) {
        let changed = self.inner.lock().unwrap().sessions.apply(event, payload, now());
        if changed {
            self.publish();
        }
        if let Some(tone) = tone_for(event, payload) {
            let _ = self.app.emit("kuchu://tone", tone);
        }
    }

    pub fn seen(&self) {
        if self.inner.lock().unwrap().sessions.seen(now()) {
            self.publish();
        }
    }

    pub fn stopped(&self, session: &str) {
        if self.inner.lock().unwrap().sessions.stopped(session, now()) {
            self.publish();
        }
    }

    /// Busy sessions silent for `quiet` ms: worth asking Claude Code whether they still run.
    pub fn silent(&self, quiet: u64) -> Vec<String> {
        self.inner.lock().unwrap().sessions.silent(now(), quiet)
    }

    /// Claude Code's answer about the sessions asked: those it no longer runs go, those it says are idle stop.
    pub fn check_running(&self, asked: &[String], running: &[Running], quiet: u64) {
        if self.inner.lock().unwrap().sessions.check_running(asked, running, now(), quiet) {
            self.publish();
        }
    }

    /// The card switched to `session`, or back to the latest.
    pub fn follow(&self, session: Option<String>) {
        if self.inner.lock().unwrap().sessions.follow(session) {
            self.publish();
        }
    }

    pub fn forget(&self, session: &str) {
        if self.inner.lock().unwrap().sessions.forget(session) {
            self.publish();
        }
    }

    pub fn expire(&self) {
        if self.inner.lock().unwrap().sessions.expire(now()) {
            self.publish();
        }
    }

    /// The card for request `id` is on the screen.
    pub fn shown(&self, id: u64) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(tx) = inner.waiting.iter_mut().find(|w| w.card.id == id).and_then(|w| w.shown.take()) {
            let _ = tx.send(());
        }
    }

    /// A click on request `id`'s card, as the decision Claude Code reads (none: the terminal asks).
    pub fn decide(&self, id: u64, choice: Choice) {
        let mut inner = self.inner.lock().unwrap();
        let Some(w) = inner.waiting.iter_mut().find(|w| w.card.id == id) else { return };
        let decision = decision(&w.request, &choice);
        if let Some(tx) = w.decide.take() {
            let _ = tx.send(decision);
        }
    }

    /// A permission request arrives: shown as a card, unless approvals are off or too many wait.
    fn register(&self, request: &Value) -> Option<(u64, oneshot::Receiver<()>, oneshot::Receiver<Option<Value>>)> {
        if !self.approvals_on.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }
        let mut inner = self.inner.lock().unwrap();
        if inner.waiting.len() >= MAX_WAITING {
            return None;
        }
        inner.next += 1;
        let id = inner.next;
        inner.sessions.apply("PermissionRequest", request, now());
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
        let mut inner = self.inner.lock().unwrap();
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
        let event = message.get("event").and_then(Value::as_str).unwrap_or("");
        let payload = message.get("payload").cloned().unwrap_or(Value::Null);
        if event != "PermissionRequest" {
            self.event(event, &payload);
            return;
        }
        let Some((id, shown, decide)) = self.register(&payload) else { return };
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

/// Listens for relays for as long as Kuchu runs. A failed connection never stops it.
pub async fn serve(hub: Arc<Hub>) -> Result<(), String> {
    #[cfg(windows)]
    {
        use tokio::net::windows::named_pipe::ServerOptions;
        let name = kuchu_wire::address();
        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&name)
            .map_err(|e| format!("Kuchu could not open its pipe (is another Kuchu running?): {e}"))?;
        loop {
            if server.connect().await.is_err() {
                if let Ok(next) = ServerOptions::new().reject_remote_clients(true).create(&name) {
                    server = next;
                }
                continue;
            }
            let next = loop {
                match ServerOptions::new().reject_remote_clients(true).create(&name) {
                    Ok(next) => break next,
                    Err(_) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
                }
            };
            let connected = std::mem::replace(&mut server, next);
            tokio::spawn(hub.clone().handle(connected));
        }
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = std::path::PathBuf::from(kuchu_wire::address());
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        if std::os::unix::net::UnixStream::connect(&path).is_ok() {
            return Err("Another Kuchu is already running.".into());
        }
        let _ = std::fs::remove_file(&path);
        let listener = tokio::net::UnixListener::bind(&path).map_err(|e| e.to_string())?;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(hub.clone().handle(stream));
            }
        }
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
