//! Claude Code sessions as Kuchu follows them, one per `session_id`, from their hook events: what each is doing now,
//! the pet's mood across all of them, and the card beside the pet.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

/// A session silent this long while it was working is taken as gone quiet (a closed terminal sends nothing).
const QUIET_MS: u64 = 30 * 60 * 1000;
/// A session silent this long is forgotten.
const FORGET_MS: u64 = 12 * 60 * 60 * 1000;
/// Lines kept per session for the card's "a few lines".
const KEPT_STEPS: usize = 3;
/// A session's name is read again from its transcript at most this often.
const NAME_EVERY_MS: u64 = 20_000;
/// How much of the end of a transcript is read for the session's name.
const NAME_TAIL: u64 = 128 * 1024;
/// A session's first message, as its name until Claude Code gives it one, at most this long.
const MAX_NAME: usize = 60;
/// A line on the card at most this long.
const MAX_LINE: usize = 140;

#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Mood {
    Idle,
    Thinking,
    Working,
    Needs,
    Done,
    Failed,
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    /// What the session is called: the name Claude Code gave it (or the one it was renamed to), else its first
    /// message, else its folder.
    pub name: String,
    /// The project folder's name.
    pub project: String,
    pub cwd: String,
    pub mood: Mood,
    /// What it is doing now, or the answer's first line once done.
    pub line: String,
    /// The last few steps, or the answer's first lines once done.
    pub lines: Vec<String>,
    /// The message that started the work, as one line.
    pub prompt: String,
    pub updated: u64,
    /// When its last turn ended, for "done until the pet is clicked".
    pub ended: Option<u64>,
    /// When its current turn started (its last message), so each turn's card is its own.
    #[serde(skip)]
    turn: u64,
    /// Claude Code ended the session while its answer was unseen: it goes once the pet is clicked.
    #[serde(skip)]
    pub closed: bool,
    #[serde(skip)]
    transcript: String,
    #[serde(skip)]
    title: Option<String>,
    #[serde(skip)]
    first_prompt: String,
    #[serde(skip)]
    name_read: u64,
}

impl Session {
    fn rename(&mut self) {
        self.name = self.title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| {
            if self.first_prompt.is_empty() {
                self.project.clone()
            } else {
                self.first_prompt.clone()
            }
        });
    }
}

/// What the pet's window shows.
#[derive(Clone, Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub session: String,
    pub title: String,
    pub line: String,
    pub lines: Vec<String>,
    pub mood: Mood,
    /// When its turn ended, or started while it is under way: a card hidden for one turn comes back for the next.
    pub at: u64,
}

#[derive(Default)]
pub struct Sessions {
    all: HashMap<String, Session>,
    /// When the pet was last clicked: a done or failed session from before is seen.
    seen_at: u64,
}

impl Sessions {
    /// Takes one hook event in. Returns whether anything shown changed.
    pub fn apply(&mut self, event: &str, p: &Value, now: u64) -> bool {
        let Some(id) = p.get("session_id").and_then(Value::as_str) else { return false };
        if event == "SessionEnd" {
            // A session that ends with its answer not yet seen (a run that finishes and exits at once) stays until
            // the pet is clicked; any other is gone.
            return match self.all.get_mut(id) {
                Some(s) if matches!(s.mood, Mood::Done | Mood::Failed) && s.ended.is_some_and(|at| at > self.seen_at) => {
                    s.closed = true;
                    false
                }
                Some(_) => self.all.remove(id).is_some(),
                None => false,
            };
        }
        // Helpers' own ends, and the background helper Claude Code runs after a turn, change nothing shown.
        if matches!(event, "SubagentStart" | "SubagentStop") {
            return false;
        }
        let cwd = text(p, "cwd");
        let s = self.all.entry(id.to_string()).or_insert_with(|| Session {
            id: id.to_string(),
            name: project_of(&cwd),
            project: project_of(&cwd),
            cwd: cwd.clone(),
            mood: Mood::Idle,
            line: String::new(),
            lines: Vec::new(),
            prompt: String::new(),
            updated: now,
            ended: None,
            turn: now,
            closed: false,
            transcript: String::new(),
            title: None,
            first_prompt: String::new(),
            name_read: 0,
        });
        if !cwd.is_empty() && cwd != s.cwd {
            s.project = project_of(&cwd);
            s.cwd = cwd;
        }
        let transcript = text(p, "transcript_path");
        if !transcript.is_empty() {
            s.transcript = transcript;
        }
        if event == "UserPromptSubmit" && s.first_prompt.is_empty() {
            s.first_prompt = cut_to(&one_line(&text(p, "prompt")), MAX_NAME);
        }
        if !s.transcript.is_empty() && now.saturating_sub(s.name_read) >= NAME_EVERY_MS {
            s.name_read = now;
            if let Some(title) = title_in(&s.transcript) {
                s.title = Some(title);
            }
        }
        s.rename();
        s.updated = now;
        match event {
            "SessionStart" => {}
            "UserPromptSubmit" => {
                s.mood = Mood::Thinking;
                s.prompt = one_line(&text(p, "prompt"));
                s.line = "Thinking…".into();
                s.lines.clear();
                s.ended = None;
                s.turn = now;
            }
            "PreToolUse" => {
                let step = step(&text(p, "tool_name"), p.get("tool_input").unwrap_or(&Value::Null));
                s.mood = Mood::Working;
                s.ended = None;
                push_step(s, step);
            }
            "PostToolUse" => {
                if s.mood == Mood::Needs {
                    s.mood = Mood::Working;
                }
            }
            "PostToolUseFailure" => {
                let failed = format!("{} — it failed", s.line);
                s.line = cut(&failed);
            }
            "PermissionRequest" => {
                let step = step(&text(p, "tool_name"), p.get("tool_input").unwrap_or(&Value::Null));
                s.mood = Mood::Needs;
                s.line = cut(&format!("Needs you: {}", lower_first(&step)));
            }
            "Notification" => {
                // Only a waiting permission prompt needs the person; other notices change nothing shown.
                if text(p, "notification_type") == "permission_prompt" {
                    s.mood = Mood::Needs;
                    s.line = cut(&one_line(&text(p, "message")));
                }
            }
            "PreCompact" => {
                s.line = "Making room in its memory…".into();
            }
            "Stop" => {
                let answer = text(p, "last_assistant_message");
                s.mood = Mood::Done;
                s.ended = Some(now);
                s.line = match first_line(&answer) {
                    l if l.is_empty() => "Done".into(),
                    l => cut(&l),
                };
                s.lines = answer.lines().map(first_line).filter(|l| !l.is_empty()).take(KEPT_STEPS).map(|l| cut(&l)).collect();
            }
            "StopFailure" => {
                s.mood = Mood::Failed;
                s.ended = Some(now);
                let why = one_line(&[text(p, "error"), text(p, "message")].concat());
                s.line = if why.is_empty() { "It did not finish".into() } else { cut(&why) };
            }
            _ => return false,
        }
        true
    }

    /// The pet was clicked: done and failed sessions from before now are seen.
    pub fn seen(&mut self, now: u64) -> bool {
        let changed = self.all.values().any(|s| self.unseen(s));
        self.seen_at = now;
        self.all.retain(|_, s| !s.closed);
        changed
    }

    /// A permission request was answered (or gone): the session works again, its line back to its step.
    pub fn resumed(&mut self, id: &str) -> bool {
        match self.all.get_mut(id) {
            Some(s) if s.mood == Mood::Needs => {
                s.mood = Mood::Working;
                s.line = s.lines.last().cloned().unwrap_or_else(|| "Working".into());
                true
            }
            _ => false,
        }
    }

    /// A turn stopped from Kuchu, or one that did not finish there: Claude Code sends no Stop for it, so the session
    /// is no longer busy.
    pub fn stopped(&mut self, id: &str, now: u64) -> bool {
        match self.all.get_mut(id) {
            Some(s) if matches!(s.mood, Mood::Thinking | Mood::Working | Mood::Needs) => {
                s.mood = Mood::Idle;
                s.line = "Stopped".into();
                s.updated = now;
                true
            }
            _ => false,
        }
    }

    /// Forgets a session (from the list: one whose terminal was closed mid-turn sends nothing more).
    pub fn forget(&mut self, id: &str) -> bool {
        self.all.remove(id).is_some()
    }

    /// Sessions gone quiet while busy fall back to idle; long-silent ones are forgotten.
    pub fn expire(&mut self, now: u64) -> bool {
        let before = self.all.len();
        self.all.retain(|_, s| now.saturating_sub(s.updated) < FORGET_MS);
        let mut changed = self.all.len() != before;
        for s in self.all.values_mut() {
            let busy = matches!(s.mood, Mood::Thinking | Mood::Working | Mood::Needs);
            if busy && now.saturating_sub(s.updated) >= QUIET_MS {
                s.mood = Mood::Idle;
                s.line = "Gone quiet".into();
                changed = true;
            }
        }
        changed
    }

    fn unseen(&self, s: &Session) -> bool {
        matches!(s.mood, Mood::Done | Mood::Failed) && s.ended.is_some_and(|at| at > self.seen_at)
    }

    /// The pet's mood across every session: needing you first; then thinking (all busy ones thinking) or working;
    /// then a session that ended since the pet was last clicked; else idle.
    pub fn mood(&self) -> Mood {
        let all: Vec<&Session> = self.all.values().collect();
        if all.iter().any(|s| s.mood == Mood::Needs) {
            return Mood::Needs;
        }
        let busy: Vec<&&Session> = all.iter().filter(|s| matches!(s.mood, Mood::Thinking | Mood::Working)).collect();
        if !busy.is_empty() {
            return if busy.iter().all(|s| s.mood == Mood::Thinking) { Mood::Thinking } else { Mood::Working };
        }
        all.iter()
            .filter(|s| self.unseen(s))
            .max_by_key(|s| s.ended)
            .map(|s| s.mood)
            .unwrap_or(Mood::Idle)
    }

    /// The card beside the pet: the session needing you, else the busiest-latest, else the last one done and unseen.
    pub fn card(&self) -> Option<Card> {
        let mut all: Vec<&Session> = self.all.values().collect();
        all.sort_by_key(|s| std::cmp::Reverse(s.updated));
        let pick = all
            .iter()
            .find(|s| s.mood == Mood::Needs)
            .or_else(|| all.iter().find(|s| matches!(s.mood, Mood::Thinking | Mood::Working)))
            .or_else(|| all.iter().find(|s| self.unseen(s)))?;
        let others = all.iter().filter(|s| s.id != pick.id && matches!(s.mood, Mood::Thinking | Mood::Working | Mood::Needs)).count();
        Some(Card {
            session: pick.id.clone(),
            title: if others > 0 { format!("{} · +{}", pick.name, others) } else { pick.name.clone() },
            line: pick.line.clone(),
            lines: pick.lines.clone(),
            mood: pick.mood,
            at: pick.ended.unwrap_or(pick.turn),
        })
    }

    /// Every session, the latest first.
    pub fn list(&self) -> Vec<Session> {
        let mut all: Vec<Session> = self.all.values().cloned().collect();
        all.sort_by_key(|s| std::cmp::Reverse(s.updated));
        all
    }

    /// What a session is called (its name, else its first message, else its folder).
    pub fn name(&self, id: &str) -> Option<String> {
        self.all.get(id).map(|s| s.name.clone())
    }
}

fn push_step(s: &mut Session, step: String) {
    s.line = cut(&step);
    s.lines.push(s.line.clone());
    if s.lines.len() > KEPT_STEPS {
        s.lines.remove(0);
    }
}

/// What a tool call is doing, as one short line.
pub fn step(tool: &str, input: &Value) -> String {
    let s = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let line = match tool {
        "Bash" | "PowerShell" => match one_line(&s("description")) {
            d if !d.is_empty() => format!("Running: {d}"),
            _ => format!("Running {}", one_line(&s("command"))),
        },
        "Read" => format!("Reading {}", file_name(&s("file_path"))),
        "Write" => format!("Writing {}", file_name(&s("file_path"))),
        "Edit" | "MultiEdit" => format!("Editing {}", file_name(&s("file_path"))),
        "NotebookEdit" => format!("Editing {}", file_name(&s("notebook_path"))),
        "Grep" => format!("Searching for “{}”", s("pattern")),
        "Glob" => format!("Looking for {}", s("pattern")),
        "WebFetch" => format!("Reading {}", host_of(&s("url"))),
        "WebSearch" => format!("Searching the web for “{}”", s("query")),
        "Task" | "Agent" => format!("Asking a helper: {}", one_line(&s("description"))),
        "TodoWrite" => "Updating its to-do list".into(),
        "AskUserQuestion" => "Has a question for you".into(),
        "ExitPlanMode" => "Has a plan ready".into(),
        t if t.starts_with("mcp__") => {
            let mut parts = t.trim_start_matches("mcp__").splitn(2, "__");
            let server = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or("").replace('_', " ");
            format!("Using {server}: {name}")
        }
        "" => "Working".into(),
        other => format!("Using {other}"),
    };
    cut(&line)
}

fn text(p: &Value, key: &str) -> String {
    p.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

fn project_of(cwd: &str) -> String {
    cwd.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().filter(|s| !s.is_empty()).unwrap_or("Claude Code").to_string()
}

fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

fn host_of(url: &str) -> String {
    url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or(url).to_string()
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A line of text without Markdown's marks: no heading, quote or list marks, links as their text.
fn first_line(markdown: &str) -> String {
    let line = markdown.lines().map(str::trim).find(|l| l.chars().any(char::is_alphanumeric)).unwrap_or("");
    let line = line.trim_start_matches(|c: char| matches!(c, '#' | '>' | '-' | '*' | '+' | ' '));
    let line = match line.split_once(['.', ')']) {
        Some((n, rest)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) && rest.starts_with(' ') => rest.trim_start(),
        _ => line,
    };
    unlink(line).chars().filter(|c| !matches!(c, '*' | '_' | '`' | '~')).collect::<String>().trim().to_string()
}

/// Markdown links and images as their text: `[text](address)` reads `text`.
fn unlink(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(open) = rest.find('[') {
        let inside = &rest[open + 1..];
        let Some(close) = inside.find(']') else { break };
        let Some(address) = inside[close + 1..].strip_prefix('(') else {
            out.push_str(&rest[..open + close + 2]);
            rest = &inside[close + 1..];
            continue;
        };
        let Some(end) = address.find(')') else { break };
        out.push_str(rest[..open].strip_suffix('!').unwrap_or(&rest[..open]));
        out.push_str(&inside[..close]);
        rest = &address[end + 1..];
    }
    out.push_str(rest);
    out
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn cut(s: &str) -> String {
    cut_to(s, MAX_LINE)
}

fn cut_to(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    format!("{}…", s.chars().take(max - 1).collect::<String>().trim_end())
}

/// The session's name as Claude Code keeps it at the end of its transcript: the name it was renamed to, else the
/// latest title Claude Code gave it. Only the end of the file is read (Claude Code writes the title again as it goes).
pub fn title_in(transcript: &str) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(transcript).ok()?;
    let len = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(NAME_TAIL))).ok()?;
    let mut bytes = Vec::new();
    file.take(NAME_TAIL).read_to_end(&mut bytes).ok()?;
    let tail = String::from_utf8_lossy(&bytes);
    let field = |line: &str, key: &str| -> Option<String> {
        let v: Value = serde_json::from_str(line).ok()?;
        let t = one_line(v.get(key)?.as_str()?);
        (!t.is_empty()).then(|| cut_to(&t, MAX_NAME))
    };
    let mut given = None;
    for line in tail.lines().rev() {
        if line.contains("\"custom-title\"") {
            if let Some(t) = field(line, "customTitle") {
                return Some(t);
            }
        }
        if given.is_none() && line.contains("\"ai-title\"") {
            given = field(line, "aiTitle");
        }
    }
    given
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/../../fixtures/claude-code-2.1.310/{name}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn a_recorded_session_goes_thinking_working_done() {
        let mut s = Sessions::default();
        assert!(s.apply("SessionStart", &fixture("SessionStart"), 1));
        assert_eq!(s.mood(), Mood::Idle);
        assert_eq!(s.card(), None);
        s.apply("UserPromptSubmit", &fixture("UserPromptSubmit"), 2);
        assert_eq!(s.mood(), Mood::Thinking);
        s.apply("PreToolUse", &fixture("PreToolUse"), 3);
        assert_eq!(s.mood(), Mood::Working);
        let card = s.card().unwrap();
        // No title in a transcript yet: the session is called by its first message.
        assert!(card.title.starts_with("Run the shell command: echo kuchu"), "{}", card.title);
        assert!(card.line.starts_with("Running"), "{}", card.line);
        s.apply("PostToolUse", &fixture("PostToolUse"), 4);
        s.apply("Stop", &fixture("Stop"), 5);
        assert_eq!(s.mood(), Mood::Done);
        assert_eq!(s.card().unwrap().line, "done");
        // Clicked: seen, back to idle; the session stays listed until it ends.
        assert!(s.seen(6));
        assert_eq!(s.mood(), Mood::Idle);
        assert_eq!(s.card(), None);
        assert!(s.apply("SessionEnd", &fixture("SessionEnd"), 7));
        assert!(s.list().is_empty());
    }

    #[test]
    fn a_run_that_ends_at_once_stays_done_until_seen() {
        let mut s = Sessions::default();
        s.apply("UserPromptSubmit", &fixture("UserPromptSubmit"), 1);
        s.apply("Stop", &fixture("Stop"), 2);
        s.apply("SessionEnd", &fixture("SessionEnd"), 3);
        assert_eq!(s.mood(), Mood::Done);
        assert_eq!(s.card().unwrap().line, "done");
        s.seen(4);
        assert!(s.list().is_empty());
    }

    #[test]
    fn a_permission_request_needs_you_until_answered() {
        let mut s = Sessions::default();
        let request = fixture("PermissionRequest");
        s.apply("PreToolUse", &fixture("PreToolUse-2"), 1);
        s.apply("PermissionRequest", &request, 2);
        assert_eq!(s.mood(), Mood::Needs);
        assert_eq!(s.card().unwrap().line, "Needs you: writing note.txt");
        assert!(s.resumed(request["session_id"].as_str().unwrap()));
        assert_eq!(s.mood(), Mood::Working);
        assert_eq!(s.card().unwrap().line, "Writing note.txt");
        assert!(s.forget(request["session_id"].as_str().unwrap()));
        assert_eq!(s.card(), None);
    }

    #[test]
    fn sessions_stay_apart() {
        let mut s = Sessions::default();
        let a = serde_json::json!({ "session_id": "a", "cwd": "C:\\work\\alpha" });
        let b = serde_json::json!({ "session_id": "b", "cwd": "/home/me/beta" });
        s.apply("UserPromptSubmit", &a, 1);
        s.apply("PreToolUse", &serde_json::json!({ "session_id": "b", "cwd": "/home/me/beta", "tool_name": "Read", "tool_input": { "file_path": "/home/me/beta/src/main.rs" } }), 2);
        assert_eq!(s.card().unwrap().title, "beta · +1");
        assert_eq!(s.card().unwrap().line, "Reading main.rs");
        s.apply("Stop", &serde_json::json!({ "session_id": "b", "last_assistant_message": "## Fixed\nThe test passes." }), 3);
        // b is done, but a still thinks: the card follows the busy one.
        assert_eq!(s.mood(), Mood::Thinking);
        assert_eq!(s.card().unwrap().title, "alpha");
        s.apply("SessionEnd", &a, 4);
        assert_eq!(s.card().unwrap().line, "Fixed");
        let _ = b;
    }

    #[test]
    fn quiet_sessions_fall_back_and_old_ones_go() {
        let mut s = Sessions::default();
        s.apply("UserPromptSubmit", &serde_json::json!({ "session_id": "a", "cwd": "/x/a" }), 0);
        assert!(!s.expire(QUIET_MS - 1));
        assert!(s.expire(QUIET_MS));
        assert_eq!(s.mood(), Mood::Idle);
        assert!(s.expire(FORGET_MS));
        assert!(s.list().is_empty());
    }

    #[test]
    fn helpers_ends_change_nothing() {
        let mut s = Sessions::default();
        s.apply("Stop", &serde_json::json!({ "session_id": "a", "last_assistant_message": "ok" }), 1);
        s.seen(2);
        assert!(!s.apply("SubagentStop", &serde_json::json!({ "session_id": "a" }), 3));
        assert_eq!(s.mood(), Mood::Idle);
    }

    #[test]
    fn a_session_is_called_by_its_name_then_its_first_message_then_its_folder() {
        let dir = std::env::temp_dir().join(format!("kuchu-names-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let transcript = dir.join("s.jsonl");
        let t = transcript.display().to_string();
        let mut s = Sessions::default();
        let event = |prompt: &str| serde_json::json!({ "session_id": "a", "cwd": "/x/my-app", "transcript_path": t, "prompt": prompt });
        s.apply("SessionStart", &event(""), 0);
        assert_eq!(s.list()[0].name, "my-app");
        s.apply("UserPromptSubmit", &event("Fix the   login bug\nplease"), NAME_EVERY_MS);
        assert_eq!(s.list()[0].name, "Fix the login bug please");
        std::fs::write(&transcript, concat!(
            r#"{"type":"user","message":"x"}"#, "\n",
            r#"{"type":"ai-title","aiTitle":"Login bug fix","sessionId":"a"}"#, "\n",
            r#"{"type":"ai-title","aiTitle":"Login bug fixed with tests","sessionId":"a"}"#, "\n",
        )).unwrap();
        s.apply("PreToolUse", &event(""), 2 * NAME_EVERY_MS);
        assert_eq!(s.list()[0].name, "Login bug fixed with tests");
        assert_eq!(s.card().unwrap().title, "Login bug fixed with tests");
        let mut text = std::fs::read_to_string(&transcript).unwrap();
        text.push_str(r#"{"type":"custom-title","customTitle":"Auth","sessionId":"a"}"#);
        text.push('\n');
        text.push_str(r#"{"type":"ai-title","aiTitle":"Something newer","sessionId":"a"}"#);
        std::fs::write(&transcript, text).unwrap();
        s.apply("PostToolUse", &event(""), 3 * NAME_EVERY_MS);
        assert_eq!(s.list()[0].name, "Auth");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_answer_reads_without_its_markdown() {
        assert_eq!(first_line("- [Kuchu rules](C:\\Users\\me\\rules.md) — standalone"), "Kuchu rules — standalone");
        assert_eq!(first_line("2. See ![the pet](pet.png) and [b] alone"), "See the pet and [b] alone");
        assert_eq!(first_line("## The **fix**: `cut` in [sessions.rs](src/sessions.rs#L385)"), "The fix: cut in sessions.rs");
        assert_eq!(first_line("3.5 GHz is plenty"), "3.5 GHz is plenty");
    }

    #[test]
    fn a_hidden_card_comes_back_for_the_next_turn() {
        let mut s = Sessions::default();
        let a = |extra: Value| {
            let mut p = serde_json::json!({ "session_id": "a", "cwd": "/x/a" });
            p.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
            p
        };
        s.apply("UserPromptSubmit", &a(serde_json::json!({ "prompt": "one" })), 1);
        s.apply("Stop", &a(serde_json::json!({ "last_assistant_message": "First" })), 2);
        let first = s.card().unwrap();
        s.apply("UserPromptSubmit", &a(serde_json::json!({ "prompt": "two" })), 3);
        assert_eq!(s.card().unwrap().at, 3);
        s.apply("Stop", &a(serde_json::json!({ "last_assistant_message": "Second" })), 4);
        let second = s.card().unwrap();
        assert_eq!((first.mood, second.mood), (Mood::Done, Mood::Done));
        assert_ne!(first.at, second.at);
    }

    #[test]
    fn each_tool_reads_as_a_step() {
        assert_eq!(step("Edit", &serde_json::json!({ "file_path": "C:\\p\\src\\app.ts" })), "Editing app.ts");
        assert_eq!(step("WebFetch", &serde_json::json!({ "url": "https://docs.rs/tauri/latest" })), "Reading docs.rs");
        assert_eq!(step("mcp__github__create_issue", &Value::Null), "Using github: create issue");
        assert_eq!(step("Bash", &serde_json::json!({ "command": "npm   test\n--watch" })), "Running npm test --watch");
    }
}
