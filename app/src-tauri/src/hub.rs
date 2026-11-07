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
use crate::host::{self, Host};
use crate::locks::Hold;
use crate::sessions::{cut_to, plain, step, tail_of, Card, Mood, Session, Sessions};

/// At most this many requests wait at once; more go straight back to the terminal.
const MAX_WAITING: usize = 5;
/// A card's title at most this long: the detail under it has the whole of it, and the card must fit the window.
const MAX_TITLE: usize = 90;
/// The most of an input a card shows whole. Past that, Allow is not offered: what cannot be read cannot be approved.
const MAX_DETAIL: usize = 20_000;
/// Blank lines in a row from which they are counted instead of shown, and spaces likewise: padding meant to push the
/// end of a command out of sight shows as what it is.
const BLANK_RUN: usize = 3;
const SPACE_RUN: usize = 40;
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
    /// The session's folder, always shown: a name is the model's to choose, a folder is not.
    pub cwd: String,
    /// "permission", "question" or "plan".
    pub kind: String,
    /// What it wants to do, as one line.
    pub title: String,
    /// The command, file or address it is about, whole (`whole`), or cut and marked as cut when too long to read.
    pub detail: String,
    /// Whether `detail` is all of it. When it is not, Allow is not offered.
    pub whole: bool,
    /// Claude Code's own "don't ask again" offer, when it made one Kuchu can pass on: the button, and what it does.
    pub always: Option<Always>,
    /// A question with one choice to make: its text and options.
    pub question: Option<Question>,
}

/// What "Always" would do: the button's label, what it allows, and where that is kept. `code` says that `what` is a rule
/// to show as it is, not words.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Always {
    pub label: String,
    pub what: String,
    pub scope: String,
    pub code: bool,
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
    /// The pet is off the screen, and its page has nothing to draw.
    pub hidden: bool,
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
    /// The pet is off the screen: nobody would see a card or hear a tone.
    hidden: bool,
}

impl Inner {
    /// Whether a new request can have a card.
    fn takes_requests(&self) -> bool {
        !self.hidden && self.waiting.len() < MAX_WAITING
    }

    /// The pet goes off the screen or comes back. Going, every request waiting on a card goes back to the terminal.
    fn hide(&mut self, hidden: bool) {
        self.hidden = hidden;
        if hidden {
            for w in &mut self.waiting {
                w.shown.take();
                w.decide.take();
            }
        }
    }
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
            hidden: inner.hidden,
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
            self.ring(tone);
        }
    }

    /// Rings `tone`, unless the pet is off the screen: a Kuchu put away for a meeting stays quiet.
    fn ring(&self, tone: Mood) {
        if !self.inner.hold().hidden {
            let _ = self.app.emit("kuchu://tone", tone);
        }
    }

    /// The pet goes off the screen or comes back. The window alone is not enough: a page can go on saying its card is on
    /// the screen, and Claude Code would wait on a card nobody can see. So requests are the terminal's at once, not after
    /// `SHOWN`, until it is back.
    pub fn set_hidden(&self, hidden: bool) {
        self.inner.hold().hide(hidden);
        self.publish();
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

    /// The app `session` runs in, found from its relay's parents. The card offers it once this is known.
    fn hosted_in(&self, session: &str, host: Option<Host>) {
        if self.inner.hold().sessions.hosted_in(session, host) {
            self.publish();
        }
    }

    /// A click on a card's button: the app the session runs in comes forward. False if it has gone, or Windows would not.
    pub fn open_app(&self, session: &str) -> bool {
        let Some((host, project)) = self.inner.hold().sessions.host_of(session) else { return false };
        host::raise(&host, &project)
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
        let decision = clicked(&w.card, &w.request, &choice);
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
        if !inner.takes_requests() {
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
        self.ring(Mood::Needs);
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

    /// One relay's connection: an event to take in, or a permission request to answer. `chain` is the process at the other end and
    /// its parents, as the system knows them.
    async fn handle<S: AsyncRead + AsyncWrite + Unpin>(self: Arc<Self>, stream: S, chain: Vec<host::Proc>) {
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
        let session = payload.get("session_id").and_then(Value::as_str).unwrap_or("").to_string();
        let look = self.inner.hold().sessions.wants_host(&session, event);
        if event != "PermissionRequest" {
            self.event(event, &payload, tail.as_deref());
            if look {
                self.hosted_in(&session, hosted(chain).await);
            }
            return;
        }
        let registered = self.register(&payload, tail.as_deref());
        if look {
            self.hosted_in(&session, hosted(chain).await);
        }
        let Some((id, shown, decide)) = registered else { return };
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

/// The app in a relay's chain of processes, found on a thread of its own: it asks the system about every window.
async fn hosted(chain: Vec<host::Proc>) -> Option<Host> {
    tokio::task::spawn_blocking(move || host::find(&chain)).await.ok().flatten()
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
    let (detail, whole) = shown(&detail);
    let (kind, question) = match tool {
        "AskUserQuestion" => ("question", single_question(&input)),
        "ExitPlanMode" => ("plan", None),
        _ => ("permission", None),
    };
    Approval {
        id,
        session,
        project,
        cwd: plain(request.get("cwd").and_then(Value::as_str).unwrap_or("")),
        kind: kind.into(),
        title: cut_to(&step(tool, &input), MAX_TITLE),
        detail,
        whole,
        always: if kind == "permission" && whole { always_for(request) } else { None },
        question: question.map(|q| Question { text: plain(&q.text), options: q.options.iter().map(|o| plain(o)).collect() }),
    }
}

/// The detail as the card shows it: made plain; blank lines and spaces in a run counted rather than shown; and whole,
/// unless longer than `MAX_DETAIL`, in which case it is cut, marked as cut, and not whole.
fn shown(detail: &str) -> (String, bool) {
    let mut lines: Vec<String> = Vec::new();
    let mut blank = 0;
    let push_blank = |lines: &mut Vec<String>, blank: usize| {
        if blank >= BLANK_RUN {
            lines.push(format!("⟨{blank} blank lines⟩"));
        } else {
            lines.extend(std::iter::repeat_n(String::new(), blank));
        }
    };
    for line in plain(detail).split('\n') {
        if line.chars().all(is_blank) {
            blank += 1;
            continue;
        }
        push_blank(&mut lines, blank);
        blank = 0;
        lines.push(spaces_counted(line));
    }
    push_blank(&mut lines, blank);
    let mut out = lines.join("\n");
    let length = out.chars().count();
    let whole = length <= MAX_DETAIL;
    if !whole {
        out = out.chars().take(MAX_DETAIL).collect();
        out.push_str(&format!("⟨… {} more characters, not shown⟩", length - MAX_DETAIL));
    }
    (out, whole)
}

/// Whether `c` shows as nothing but room: a space or tab, any other space (no-break, wide, thin…), or one of the
/// letters drawn blank. Padding made of any of them pushes the end of a command out of sight as well as spaces do.
fn is_blank(c: char) -> bool {
    c != '\n' && (c.is_whitespace() || matches!(c, '\u{115F}' | '\u{1160}' | '\u{2800}' | '\u{3164}' | '\u{FFA0}'))
}

/// A line with each run of `SPACE_RUN` or more blank characters (`is_blank`) counted instead of shown.
fn spaces_counted(line: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    let flush = |out: &mut String, run: &mut String| {
        if run.chars().count() >= SPACE_RUN {
            out.push_str(&format!("⟨{} spaces⟩", run.chars().count()));
        } else {
            out.push_str(run);
        }
        run.clear();
    };
    for c in line.chars() {
        if is_blank(c) {
            run.push(c);
        } else {
            flush(&mut out, &mut run);
            out.push(c);
        }
    }
    flush(&mut out, &mut run);
    out
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

/// Claude Code's first "don't ask again" offer, as a button and what it does: the rule, folder or mode it would add,
/// and where that is kept. None for an offer to stop asking altogether, or one Kuchu does not know. A rule for just the
/// command on the card says so instead of repeating it: the card already shows that command whole, and a rule as long as
/// the command is more than a card can hold.
fn always_for(request: &Value) -> Option<Always> {
    let first = request.get("permission_suggestions")?.as_array()?.first()?;
    if first.get("behavior").and_then(Value::as_str).is_some_and(|b| b != "allow") {
        return None;
    }
    let scope = match first.get("destination").and_then(Value::as_str).unwrap_or("") {
        "session" => "for this session".to_string(),
        "localSettings" => "in this project, for you".to_string(),
        "projectSettings" => "in this project, for everyone".to_string(),
        "userSettings" => "for you, in every project".to_string(),
        other => plain(other),
    };
    let (label, what, code) = match first.get("type").and_then(Value::as_str)? {
        "addRules" => {
            let rule = |r: &Value| {
                let tool = r.get("toolName")?.as_str()?;
                Some(match r.get("ruleContent").and_then(Value::as_str) {
                    Some(content) => format!("{tool}({content})"),
                    None => tool.to_string(),
                })
            };
            let rules: Vec<String> = first.get("rules")?.as_array()?.iter().filter_map(rule).collect();
            if rules.is_empty() {
                return None;
            }
            if is_this_command(request, first) {
                ("Always allow", "this exact command".to_string(), false)
            } else {
                ("Always allow", rules.join(", "), true)
            }
        }
        "addDirectories" => {
            let folders: Vec<&str> = first.get("directories")?.as_array()?.iter().filter_map(Value::as_str).collect();
            if folders.is_empty() {
                return None;
            }
            ("Allow this folder", format!("work in {}", folders.join(", ")), false)
        }
        "setMode" => match first.get("mode").and_then(Value::as_str)? {
            "acceptEdits" => ("Allow edits for now", "edits to files without asking".to_string(), false),
            _ => return None,
        },
        _ => return None,
    };
    Some(Always { label: label.into(), what: plain(&what), scope, code })
}

/// Whether an offer is one rule for the very command the request asks about, by the same tool, and nothing more.
fn is_this_command(request: &Value, offer: &Value) -> bool {
    let Some([rule]) = offer.get("rules").and_then(Value::as_array).map(Vec::as_slice) else { return false };
    let command = request.get("tool_input").and_then(|input| input.get("command")).and_then(Value::as_str);
    command.is_some() && rule.get("ruleContent").and_then(Value::as_str) == command && rule.get("toolName") == request.get("tool_name")
}

/// The decision for a click on `card`. A card that could not show the whole input takes only a "no", whatever the
/// page sends: the page leaves Allow off such a card, and this holds even if it did not.
fn clicked(card: &Approval, request: &Value, choice: &Choice) -> Option<Value> {
    if !card.whole && choice.kind != "deny" {
        return None;
    }
    decision(request, choice)
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
            always_for(request)?;
            let first = request.get("permission_suggestions")?.as_array()?.first()?.clone();
            Some(json!({ "behavior": "allow", "updatedPermissions": [first] }))
        }
        "answer" => {
            // The option as the card showed it (made plain) names the option as Claude Code gave it.
            let question = single_question(&input)?;
            let answer = question.options.iter().find(|o| choice.answer.as_deref() == Some(plain(o).as_str()))?.clone();
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
            // First of all: a relay writes and ends at once, and what it runs in is only known while it, or its parents, can be asked.
            let chain = kuchu_wire::client_pid(server.as_raw_handle()).map(host::chain).unwrap_or_default();
            let next = loop {
                match make(false) {
                    Ok(next) => break next,
                    Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
                }
            };
            let connected = std::mem::replace(&mut server, next);
            let me = kuchu_wire::client_is_me(connected.as_raw_handle());
            take_in(&hub, &permits, connected, me, chain);
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
                take_in(&hub, &permits, stream, me, Vec::new());
            }
        }
    }
}

/// A connection just made: heard if it is this account's (`me`) and there is room, else closed unread. `chain` is the process at its
/// other end and its parents, where the system can say.
fn take_in<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(hub: &Arc<Hub>, permits: &Arc<Semaphore>, stream: S, me: bool, chain: Vec<host::Proc>) {
    if !me {
        return;
    }
    let Ok(permit) = permits.clone().try_acquire_owned() else { return };
    let hub = hub.clone();
    tokio::spawn(async move {
        let _permit = permit;
        hub.handle(stream, chain).await;
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

    /// A request as `register` leaves it, and the wait its connection is in.
    fn waiting(inner: &mut Inner, id: u64) -> (oneshot::Receiver<()>, oneshot::Receiver<Option<Value>>) {
        let (shown_tx, shown_rx) = oneshot::channel();
        let (decide_tx, decide_rx) = oneshot::channel();
        let request = fixture("PermissionRequest");
        inner.waiting.push(Waiting { card: card(id, "s".into(), "p".into(), &request), request, shown: Some(shown_tx), decide: Some(decide_tx) });
        (shown_rx, decide_rx)
    }

    #[test]
    fn a_hidden_pet_takes_no_requests_until_it_is_back() {
        let mut inner = Inner::default();
        assert!(inner.takes_requests());
        inner.hide(true);
        assert!(!inner.takes_requests());
        inner.hide(false);
        assert!(inner.takes_requests());
    }

    /// One that is on a card, one that has not reached the screen: hiding must not leave either to run out its time.
    #[tokio::test]
    async fn hiding_the_pet_hands_what_waits_back_to_the_terminal_at_once() {
        let mut inner = Inner::default();
        let (shown, decide) = waiting(&mut inner, 1);
        let (not_yet, not_decided) = waiting(&mut inner, 2);
        inner.waiting[0].shown.take().unwrap().send(()).unwrap();
        let on_card = tokio::spawn(wait(shown, decide));
        let unseen = tokio::spawn(wait(not_yet, not_decided));
        tokio::task::yield_now().await;
        inner.hide(true);
        let quickly = Duration::from_millis(100);
        assert_eq!(tokio::time::timeout(quickly, on_card).await.unwrap().unwrap(), None);
        assert_eq!(tokio::time::timeout(quickly, unseen).await.unwrap().unwrap(), None);
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
        assert!(c.whole);
        assert_eq!(c.cwd, r"C:\Users\you\projects\demo");
        assert_eq!(c.always.as_ref().map(|a| a.label.as_str()), Some("Allow edits for now"));
        // A read outside the session's folder: Claude Code offers a rule, not a mode.
        let read = card(2, "s".into(), "demo".into(), &fixture("PermissionRequest-2"));
        assert_eq!((read.title.as_str(), read.always.as_ref().map(|a| a.label.as_str())), ("Reading compact", Some("Always allow")));
    }

    #[test]
    fn the_always_button_says_what_it_allows_and_where() {
        let a = always_for(&fixture("PermissionRequest")).unwrap();
        assert_eq!((a.label.as_str(), a.what.as_str(), a.scope.as_str()), ("Allow edits for now", "edits to files without asking", "for this session"));
        let a = always_for(&fixture("PermissionRequest-2")).unwrap();
        assert_eq!((a.label.as_str(), a.what.as_str()), ("Always allow", "Read(//c/Program Files/Git/**)"));
        let offer = |suggestion: Value| always_for(&json!({ "permission_suggestions": [suggestion] }));
        let rule = offer(json!({ "type": "addRules", "behavior": "allow", "destination": "localSettings", "rules": [{ "toolName": "Bash", "ruleContent": "git commit:*" }] })).unwrap();
        assert_eq!((rule.what.as_str(), rule.scope.as_str()), ("Bash(git commit:*)", "in this project, for you"));
        let folder = offer(json!({ "type": "addDirectories", "destination": "userSettings", "directories": ["D:\\work"] })).unwrap();
        assert_eq!((folder.label.as_str(), folder.what.as_str(), folder.scope.as_str()), ("Allow this folder", "work in D:\\work", "for you, in every project"));
        // A deny rule, a mode Kuchu does not know, an empty offer: no button.
        assert!(offer(json!({ "type": "addRules", "behavior": "deny", "rules": [{ "toolName": "Bash" }] })).is_none());
        assert!(offer(json!({ "type": "setMode", "mode": "bypassPermissions" })).is_none());
        assert!(offer(json!({ "type": "addRules", "rules": [] })).is_none());
    }

    #[test]
    fn a_rule_for_just_the_command_on_the_card_says_so_and_any_other_rule_shows_itself() {
        let command = "docker compose up -d && curl -s localhost:3000/health";
        let offer = |tool: &str, rules: Value| {
            always_for(&json!({ "tool_name": tool, "tool_input": { "command": command }, "permission_suggestions": [{ "type": "addRules", "behavior": "allow", "destination": "localSettings", "rules": rules }] })).unwrap()
        };
        let same = json!({ "toolName": "Bash", "ruleContent": command });
        let exact = offer("Bash", json!([same]));
        assert_eq!((exact.what.as_str(), exact.code, exact.scope.as_str()), ("this exact command", false, "in this project, for you"));
        // Anything wider than that command, or more than it, must show what it is: "exact" would be untrue.
        let wider = offer("Bash", json!([{ "toolName": "Bash", "ruleContent": "docker compose up:*" }]));
        assert_eq!((wider.what.as_str(), wider.code), ("Bash(docker compose up:*)", true));
        let two = offer("Bash", json!([same, { "toolName": "Bash", "ruleContent": "rm:*" }]));
        assert!(two.code && two.what.ends_with("Bash(rm:*)"));
        let whole_tool = offer("Bash", json!([{ "toolName": "Bash" }]));
        assert_eq!((whole_tool.what.as_str(), whole_tool.code), ("Bash", true));
        assert_eq!(offer("PowerShell", json!([same])).what, format!("Bash({command})"));
    }

    #[test]
    fn a_long_or_padded_input_is_shown_whole_or_not_allowed() {
        let bash = |command: String| card(1, "s".into(), "demo".into(), &json!({ "tool_name": "Bash", "tool_input": { "command": command }, "permission_suggestions": [{ "type": "setMode", "mode": "acceptEdits" }] }));
        // Padding counted, the tail in sight, and the offer still made.
        let padded = bash(format!("echo ok{}{}; rm -rf ~", "\n".repeat(60), " ".repeat(100)));
        assert_eq!(padded.detail, "echo ok\n⟨59 blank lines⟩\n⟨100 spaces⟩; rm -rf ~");
        assert!(padded.whole && padded.always.is_some());
        // Padding of other blank characters (no-break, wide, thin spaces, a blank letter) counts the same.
        let other = bash(format!("echo ok{}; rm -rf ~", "\u{A0}\u{3000}\u{2009}\u{2800}".repeat(500)));
        assert_eq!(other.detail, "echo ok⟨2000 spaces⟩; rm -rf ~");
        let lines = bash(format!("echo ok{}rm -rf ~", "\n\u{A0}\u{3164}".repeat(30)));
        assert_eq!(lines.detail, "echo ok\n⟨29 blank lines⟩\n\u{A0}\u{3164}rm -rf ~");
        // Too long to read: cut, marked as cut, and not to be allowed from here, whatever the page sends.
        let long = bash(format!("echo {}; rm -rf ~", "x".repeat(MAX_DETAIL)));
        assert!(!long.whole && long.always.is_none());
        let request = json!({ "tool_name": "Bash", "tool_input": { "command": "x" } });
        let click = |kind: &str| clicked(&long, &request, &Choice { kind: kind.into(), answer: None });
        assert_eq!((click("allow"), click("always"), click("answer")), (None, None, None));
        assert_eq!(click("deny").unwrap()["behavior"], "deny");
        assert_eq!(clicked(&padded, &request, &Choice { kind: "allow".into(), answer: None }), Some(json!({ "behavior": "allow" })));
        assert!(long.title.chars().count() <= MAX_TITLE && long.title.ends_with('…'));
        assert!(long.detail.ends_with("⟨… 15 more characters, not shown⟩"));
        assert_eq!(long.detail.chars().count(), MAX_DETAIL + "⟨… 15 more characters, not shown⟩".chars().count());
        // Ordinary indentation and a blank line or two stay as they are.
        let code = bash("if x:\n    y()\n\n    z()\n".into());
        assert_eq!(code.detail, "if x:\n    y()\n\n    z()\n");
    }

    #[test]
    fn an_answer_is_the_option_as_claude_code_gave_it() {
        let request = json!({
            "tool_name": "AskUserQuestion",
            "tool_input": { "questions": [{ "question": "Which?", "header": "Pick", "multiSelect": false,
                "options": [{ "label": "A\u{202E}B", "description": "" }, { "label": "Plain", "description": "" }] }] }
        });
        let shown = card(1, "s".into(), "demo".into(), &request).question.unwrap();
        assert_eq!(shown.options, ["A⟨RLO⟩B", "Plain"]);
        let d = decision(&request, &Choice { kind: "answer".into(), answer: Some("A⟨RLO⟩B".into()) }).unwrap();
        assert_eq!(d["updatedInput"]["answers"]["Which?"], "A\u{202E}B");
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
