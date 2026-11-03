//! Claude Code sessions as Kuchu follows them, one per `session_id`, from their hook events: what each is doing now,
//! the pet's mood across all of them, and the card beside the pet.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

use crate::alive::Running;

/// A session silent this long while it was working is taken as gone quiet (a closed terminal sends nothing).
const QUIET_MS: u64 = 30 * 60 * 1000;
/// A session silent this long is forgotten.
const FORGET_MS: u64 = 12 * 60 * 60 * 1000;
/// At most this many sessions are followed (each is listed in every view); past that the one silent longest goes to
/// make room for a new one.
const MAX_SESSIONS: usize = 32;
/// Lines kept per session for the card's "a few lines".
const KEPT_STEPS: usize = 3;
/// A session's name is read again from its transcript at most this often.
const NAME_EVERY_MS: u64 = 20_000;
/// How much of the end of a transcript is read: enough for the session's name, and for what Claude said before a tool
/// (a tool's result can be long).
const TAIL: u64 = 256 * 1024;
/// How many of the things Claude said are remembered as shown.
const SAID_KEPT: usize = 16;
/// A session's first message, as its name until Claude Code gives it one, at most this long.
const MAX_NAME: usize = 60;
/// A line on the card at most this long.
const MAX_LINE: usize = 140;
/// The line while Claude works out its next step: after a message, and between steps once every one has ended, so a
/// card of finished steps never reads as a finished turn.
const THINKING: &str = "Thinking…";

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

/// Where a step stands: its tool running, finished, or failed (or cut short).
#[derive(Clone, Copy, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Progress {
    Running,
    Done,
    Failed,
}

/// One line of a session: a step (with where it stands), or a line of its answer (without).
#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub progress: Option<Progress>,
    /// The tool call it is (Claude Code's `tool_use_id`), so its end finds it.
    #[serde(skip)]
    pub tool: String,
}

impl Line {
    fn said(text: String) -> Line {
        Line { text, progress: None, tool: String::new() }
    }
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
    pub lines: Vec<Line>,
    /// The message that started the work, as one line.
    pub prompt: String,
    pub updated: u64,
    /// When its last turn ended, for "done until the pet is clicked".
    pub ended: Option<u64>,
    /// When its current turn started (its last message), so each turn's card is its own, and the pet waves hello.
    turn: u64,
    /// Claude Code ended the session while its answer was unseen: it goes once the pet is clicked.
    #[serde(skip)]
    pub closed: bool,
    #[serde(skip)]
    title: Option<String>,
    #[serde(skip)]
    first_prompt: String,
    #[serde(skip)]
    name_read: u64,
    /// The transcript lines of what Claude said that are already shown, the latest last, so each shows once.
    #[serde(skip)]
    said: Vec<String>,
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
    /// What the session is called.
    pub title: String,
    pub line: String,
    /// Where `line` stands, when it is a step.
    pub progress: Option<Progress>,
    pub lines: Vec<Line>,
    pub mood: Mood,
    /// When its turn ended, or started while it is under way: a card hidden for one turn comes back for the next.
    pub at: u64,
    /// The other sessions at work or waiting on you, the latest first: the card can be switched to any of them.
    pub others: Vec<Other>,
    /// The card shows this session because it was chosen, not because it is the latest.
    pub pinned: bool,
}

/// Another session at work, as the card lists it: with "a few lines", its steps too.
#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct Other {
    pub session: String,
    pub name: String,
    pub mood: Mood,
    pub line: String,
    pub progress: Option<Progress>,
    pub lines: Vec<Line>,
}

#[derive(Default)]
pub struct Sessions {
    all: HashMap<String, Session>,
    /// When the pet was last clicked: a done or failed session from before is seen.
    seen_at: u64,
    /// The session the card was switched to: it stays there while that session works.
    followed: Option<String>,
}

impl Sessions {
    /// Takes one hook event in, with no transcript to read. Returns whether anything shown changed.
    #[cfg(test)]
    pub fn apply(&mut self, event: &str, p: &Value, now: u64) -> bool {
        self.apply_with(event, p, now, None)
    }

    /// Takes one hook event in, with the end of the session's transcript as read beforehand (`tail_of`): the
    /// session's name and what Claude said before a tool are in there, not in the event.
    pub fn apply_with(&mut self, event: &str, p: &Value, now: u64, tail: Option<&str>) -> bool {
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
        if !self.all.contains_key(id) && self.all.len() >= MAX_SESSIONS {
            if let Some(oldest) = self.all.values().min_by_key(|s| s.updated).map(|s| s.id.clone()) {
                self.all.remove(&oldest);
            }
        }
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
            title: None,
            first_prompt: String::new(),
            name_read: 0,
            said: Vec::new(),
        });
        if !cwd.is_empty() && cwd != s.cwd {
            s.project = project_of(&cwd);
            s.cwd = cwd;
        }
        if event == "UserPromptSubmit" && s.first_prompt.is_empty() {
            s.first_prompt = cut_to(&one_line(&text(p, "prompt")), MAX_NAME);
        }
        if let Some(tail) = tail.filter(|_| now.saturating_sub(s.name_read) >= NAME_EVERY_MS) {
            s.name_read = now;
            if let Some(title) = title_in(tail) {
                s.title = Some(title);
            }
        }
        s.rename();
        s.updated = now;
        match event {
            "SessionStart" => {}
            "UserPromptSubmit" => {
                s.mood = Mood::Thinking;
                s.prompt = plain(&one_line(&text(p, "prompt")));
                s.line = THINKING.into();
                s.lines.clear();
                s.ended = None;
                s.turn = now;
            }
            "PreToolUse" => {
                let step = step(&text(p, "tool_name"), p.get("tool_input").unwrap_or(&Value::Null));
                s.mood = Mood::Working;
                s.ended = None;
                push_step(s, step, text(p, "tool_use_id"));
                note_said(s, &text(p, "tool_use_id"), tail);
            }
            "PostToolUse" => {
                if s.mood == Mood::Needs {
                    s.mood = Mood::Working;
                }
                finish_step(s, &text(p, "tool_use_id"), Progress::Done);
                note_said(s, &text(p, "tool_use_id"), tail);
                between_steps(s);
            }
            "PostToolUseFailure" => {
                finish_step(s, &text(p, "tool_use_id"), Progress::Failed);
                note_said(s, &text(p, "tool_use_id"), tail);
                between_steps(s);
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
                s.lines = answer.lines().map(first_line).filter(|l| !l.is_empty()).take(KEPT_STEPS).map(|l| Line::said(cut(&l))).collect();
            }
            "StopFailure" => {
                s.mood = Mood::Failed;
                s.ended = Some(now);
                cut_short(s);
                let why = one_line(&[text(p, "error"), text(p, "message")].concat());
                s.line = if why.is_empty() { "It did not finish".into() } else { cut(&why) };
            }
            _ => return false,
        }
        // The session the card was switched to is done with its turn: the card follows the latest again.
        if matches!(event, "Stop" | "StopFailure") && self.followed.as_deref() == Some(id) {
            self.followed = None;
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
                s.line = s.lines.last().map(|l| l.text.clone()).unwrap_or_else(|| "Working".into());
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
                cut_short(s);
                s.updated = now;
                true
            }
            _ => false,
        }
    }

    /// Busy sessions that have sent nothing for `quiet` ms: worth asking Claude Code whether they still run.
    pub fn silent(&self, now: u64, quiet: u64) -> Vec<String> {
        let busy = |s: &&Session| matches!(s.mood, Mood::Thinking | Mood::Working | Mood::Needs);
        self.all.values().filter(busy).filter(|s| now.saturating_sub(s.updated) >= quiet).map(|s| s.id.clone()).collect()
    }

    /// What Claude Code says still runs, for the sessions asked about (if still silent): one it no longer lists had its
    /// terminal closed and is forgotten; one it lists as idle was stopped without a Stop (Esc) and is busy no more.
    pub fn check_running(&mut self, asked: &[String], running: &[Running], now: u64, quiet: u64) -> bool {
        let mut changed = false;
        for id in asked {
            let Some(s) = self.all.get(id) else { continue };
            if now.saturating_sub(s.updated) < quiet {
                continue;
            }
            match running.iter().find(|r| &r.id == id) {
                None => changed |= self.all.remove(id).is_some(),
                Some(r) if !r.busy && matches!(s.mood, Mood::Thinking | Mood::Working) => changed |= self.stopped(id, now),
                Some(_) => {}
            }
        }
        changed
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
        let busy = |s: &&&Session| matches!(s.mood, Mood::Thinking | Mood::Working | Mood::Needs);
        let followed = self.followed.as_ref().and_then(|id| all.iter().filter(busy).find(|s| &s.id == id));
        let pick = all
            .iter()
            .find(|s| s.mood == Mood::Needs)
            .or(followed)
            .or_else(|| all.iter().find(|s| matches!(s.mood, Mood::Thinking | Mood::Working)))
            .or_else(|| all.iter().find(|s| self.unseen(s)))?;
        let others = all
            .iter()
            .filter(busy)
            .filter(|s| s.id != pick.id)
            .map(|s| Other {
                session: s.id.clone(),
                name: s.name.clone(),
                mood: s.mood,
                line: s.line.clone(),
                progress: progress_of(s),
                lines: s.lines.clone(),
            })
            .collect();
        Some(Card {
            session: pick.id.clone(),
            title: pick.name.clone(),
            others,
            pinned: self.followed.as_deref() == Some(pick.id.as_str()),
            line: pick.line.clone(),
            progress: progress_of(pick),
            lines: pick.lines.clone(),
            mood: pick.mood,
            at: pick.ended.unwrap_or(pick.turn),
        })
    }

    /// The card switched to session `id` (it stays there while that session works), or back to the latest (`None`).
    pub fn follow(&mut self, id: Option<String>) -> bool {
        let id = id.filter(|id| self.all.contains_key(id));
        let changed = self.followed != id;
        self.followed = id;
        changed
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

fn push_step(s: &mut Session, step: String, tool: String) {
    s.line = cut(&step);
    s.lines.push(Line { text: s.line.clone(), progress: Some(Progress::Running), tool });
    let extra = s.lines.len().saturating_sub(KEPT_STEPS);
    s.lines.drain(..extra);
}

/// What Claude said before tool call `tool`, put just before its step, once. Hooks do not carry Claude's words; its
/// transcript does, written a moment after the hook runs, so this is tried when the tool starts and again when it ends.
fn note_said(s: &mut Session, tool: &str, tail: Option<&str>) {
    if tool.is_empty() {
        return;
    }
    let Some(at) = s.lines.iter().position(|l| l.tool == tool) else { return };
    let Some((id, said)) = tail.and_then(|tail| said_before(tail, tool)).filter(|(id, _)| !s.said.contains(id)) else { return };
    s.said.push(id);
    let extra = s.said.len().saturating_sub(SAID_KEPT);
    s.said.drain(..extra);
    s.lines.insert(at, Line::said(cut(&said)));
    let extra = s.lines.len().saturating_sub(KEPT_STEPS);
    s.lines.drain(..extra);
}

/// A tool call ended: its step is done or failed. Without an id, the last step still running is the one.
fn finish_step(s: &mut Session, tool: &str, progress: Progress) {
    let running = |l: &&mut Line| l.progress == Some(Progress::Running);
    let found = if tool.is_empty() { s.lines.iter_mut().rev().find(running) } else { s.lines.iter_mut().find(|l| l.tool == tool) };
    if let Some(line) = found {
        line.progress = Some(progress);
    }
}

/// Where a session's line stands, when it is a step or Claude working out the next one.
fn progress_of(s: &Session) -> Option<Progress> {
    if s.line == THINKING && matches!(s.mood, Mood::Thinking | Mood::Working) {
        return Some(Progress::Running);
    }
    s.lines.last().filter(|l| l.text == s.line).and_then(|l| l.progress)
}

/// Every step has ended but the turn has not: Claude is thinking again, working out what comes next.
fn between_steps(s: &mut Session) {
    if s.mood == Mood::Working && s.lines.iter().all(|l| l.progress != Some(Progress::Running)) {
        s.mood = Mood::Thinking;
        s.line = THINKING.into();
    }
}

/// A turn ended before its steps did: those still running did not finish.
fn cut_short(s: &mut Session) {
    for line in s.lines.iter_mut().filter(|l| l.progress == Some(Progress::Running)) {
        line.progress = Some(Progress::Failed);
    }
}

/// What a tool call is doing, as one short line.
pub fn step(tool: &str, input: &Value) -> String {
    let s = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or("").to_string();
    let line = match tool {
        // What the command is for, as Claude Code describes it, else the command itself; its mark says it runs.
        "Bash" | "PowerShell" => match one_line(&s("description")) {
            d if !d.is_empty() => d,
            _ => one_line(&s("command")),
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
    plain(cwd.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().filter(|s| !s.is_empty()).unwrap_or("Claude Code"))
}

/// Text as it is shown: what cannot be seen, or changes how the rest reads, is spelled out instead, so that nothing
/// on a card hides or reorders what is approved. Direction marks (`⟨RLO⟩`), zero-width spaces, line and paragraph
/// separators, soft hyphens, tags and the like, and control characters but for the tab and the line break
/// (`⟨U+0007⟩`). A carriage return before a line break is dropped: the line break says it all.
pub fn plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        let mark = match c {
            '\r' if chars.peek() == Some(&'\n') => continue,
            '\n' | '\t' => {
                out.push(c);
                continue;
            }
            '\u{202A}' => "LRE",
            '\u{202B}' => "RLE",
            '\u{202C}' => "PDF",
            '\u{202D}' => "LRO",
            '\u{202E}' => "RLO",
            '\u{2066}' => "LRI",
            '\u{2067}' => "RLI",
            '\u{2068}' => "FSI",
            '\u{2069}' => "PDI",
            '\u{200E}' => "LRM",
            '\u{200F}' => "RLM",
            '\u{061C}' => "ALM",
            '\u{200B}' => "ZWSP",
            '\u{2060}' => "WJ",
            '\u{FEFF}' => "BOM",
            '\u{2028}' => "LS",
            '\u{2029}' => "PS",
            '\u{00AD}' => "SHY",
            c if c.is_control() || matches!(c, '\u{034F}' | '\u{180E}' | '\u{2061}'..='\u{2064}' | '\u{206A}'..='\u{206F}' | '\u{E0000}'..='\u{E007F}') => {
                out.push_str(&format!("⟨U+{:04X}⟩", c as u32));
                continue;
            }
            c => {
                out.push(c);
                continue;
            }
        };
        out.push_str(&format!("⟨{mark}⟩"));
    }
    out
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

/// At most `max` characters of `s`, made plain, with a mark where it was cut.
pub(crate) fn cut_to(s: &str, max: usize) -> String {
    let s = plain(s);
    if s.chars().count() <= max {
        return s;
    }
    format!("{}…", s.chars().take(max - 1).collect::<String>().trim_end())
}

/// What Claude said just before tool call `tool`, if anything, in the end of the transcript (`tail`): the transcript
/// line's id and the text's first line. Found by the tool call's line, then read back from there, stopping at a
/// tool's result or at the person's message (words from before those were said for something else).
pub fn said_before(tail: &str, tool: &str) -> Option<(String, String)> {
    if tool.is_empty() {
        return None;
    }
    let call = format!("\"id\":\"{tool}\"");
    let mut lines = tail.lines().rev().skip_while(|l| !(l.contains(&call) && l.contains("\"tool_use\"")));
    lines.next()?;
    for line in lines {
        let is_user = line.contains("\"type\":\"user\"");
        if !is_user && !line.contains("\"type\":\"assistant\"") {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        if v.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        if is_user {
            return None;
        }
        let blocks = v["message"]["content"].as_array().cloned().unwrap_or_default();
        if let Some(said) = blocks.iter().rev().filter(|b| b["type"] == "text").map(|b| first_line(b["text"].as_str().unwrap_or(""))).find(|t| !t.is_empty()) {
            return Some((v["uuid"].as_str().unwrap_or("").to_string(), said));
        }
    }
    None
}

/// The last `TAIL` bytes of the transcript at `path`, as text (a line cut at the start is dropped by whoever reads
/// it). Every read of a transcript goes through here, so what may be opened is decided in one place: the path comes
/// from a payload, and is read only if it is absolute, not a path to another machine or a pipe (`\\`, `//`), and names
/// a plain file itself, not through a link, so that nothing can be made to wait on the other end of it.
pub fn tail_of(path: &str) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    if path.starts_with("\\\\") || path.starts_with("//") || !std::path::Path::new(path).is_absolute() {
        return None;
    }
    if !std::fs::symlink_metadata(path).ok()?.is_file() {
        return None;
    }
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(TAIL))).ok()?;
    let mut buf = Vec::new();
    file.take(TAIL).read_to_end(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// The session's name as Claude Code keeps it at the end of its transcript (`tail`): the name it was renamed to, else
/// the latest title Claude Code gave it (it writes the title again as it goes, so the end of the file has it).
pub fn title_in(tail: &str) -> Option<String> {
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
        assert_eq!(card.line, "Print kuchu");
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
    fn the_page_learns_when_a_message_was_sent_so_the_pet_can_wave() {
        let mut s = Sessions::default();
        s.apply("UserPromptSubmit", &fixture("UserPromptSubmit"), 7);
        s.apply("PreToolUse", &fixture("PreToolUse"), 9);
        let sent = |s: &Sessions| serde_json::to_value(&s.list()[0]).unwrap()["turn"].clone();
        // Work on that message does not move it; the next message does.
        assert_eq!(sent(&s), 7);
        s.apply("UserPromptSubmit", &fixture("UserPromptSubmit"), 12);
        assert_eq!(sent(&s), 12);
    }

    #[test]
    fn sessions_stay_apart() {
        let mut s = Sessions::default();
        let a = serde_json::json!({ "session_id": "a", "cwd": "C:\\work\\alpha" });
        let b = serde_json::json!({ "session_id": "b", "cwd": "/home/me/beta" });
        s.apply("UserPromptSubmit", &a, 1);
        s.apply("PreToolUse", &serde_json::json!({ "session_id": "b", "cwd": "/home/me/beta", "tool_name": "Read", "tool_input": { "file_path": "/home/me/beta/src/main.rs" } }), 2);
        let card = s.card().unwrap();
        assert_eq!((card.title.as_str(), card.others.len(), card.others[0].name.as_str()), ("beta", 1, "alpha"));
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
    fn the_card_stays_on_a_chosen_session_while_it_works() {
        let mut s = Sessions::default();
        let tool = |id: &str, file: &str| serde_json::json!({ "session_id": id, "cwd": format!("/x/{id}"), "tool_name": "Read", "tool_input": { "file_path": file } });
        for (i, id) in ["a", "b", "c"].iter().enumerate() {
            s.apply("PreToolUse", &tool(id, "x.rs"), i as u64 + 1);
        }
        assert_eq!(s.card().unwrap().title, "c");
        assert!(s.follow(Some("a".into())));
        s.apply("PreToolUse", &tool("b", "y.rs"), 10);
        let card = s.card().unwrap();
        assert_eq!((card.title.as_str(), card.pinned, card.others.len()), ("a", true, 2));
        // Its turn over, the card follows the latest again; a session that is gone cannot be chosen.
        s.apply("Stop", &serde_json::json!({ "session_id": "a", "last_assistant_message": "ok" }), 11);
        assert_eq!((s.card().unwrap().title.as_str(), s.card().unwrap().pinned), ("b", false));
        assert!(!s.follow(Some("nobody".into())));
    }

    #[test]
    fn a_recorded_failure_helper_and_compaction_read_right() {
        let mut s = Sessions::default();
        let failure = fixture("PostToolUseFailure");
        // The failed Read as it started, then its failure: the step fails, found by its tool_use_id.
        s.apply("PreToolUse", &failure, 1);
        s.apply("PostToolUseFailure", &failure, 2);
        assert_eq!(s.card().unwrap().lines.last().unwrap().progress, Some(Progress::Failed));
        // A helper's start and stop change nothing shown: the helper's last message is not the session's answer.
        assert!(!s.apply("SubagentStart", &fixture("SubagentStart"), 3));
        assert!(!s.apply("SubagentStop", &fixture("SubagentStop"), 4));
        assert_eq!(s.mood(), Mood::Thinking);
        // Compacting says so on the card, and the session starting again after it is the same session.
        s.apply("PreCompact", &fixture("PreCompact"), 5);
        assert_eq!(s.card().unwrap().line, "Making room in its memory…");
        s.apply("SessionStart", &fixture("SessionStart-3"), 6);
        assert_eq!(s.list().len(), 1);
    }

    #[test]
    fn claude_code_says_which_silent_sessions_still_run() {
        let mut s = Sessions::default();
        let tool = |id: &str| serde_json::json!({ "session_id": id, "cwd": format!("/x/{id}"), "tool_name": "Read", "tool_input": { "file_path": "a.rs" } });
        for id in ["closed", "stopped", "long", "chatty"] {
            s.apply("PreToolUse", &tool(id), 0);
        }
        s.apply("PreToolUse", &tool("chatty"), 20_000);
        let mut asked = s.silent(20_000, 15_000);
        asked.sort();
        assert_eq!(asked, ["closed", "long", "stopped"]);
        // "chatty" spoke while Claude Code was asked: its answer is not taken for it.
        s.apply("PreToolUse", &tool("long"), 21_000);
        let running = [Running { id: "stopped".into(), busy: false }, Running { id: "long".into(), busy: true }, Running { id: "chatty".into(), busy: true }];
        assert!(s.check_running(&asked, &running, 22_000, 15_000));
        let left: Vec<(String, Mood)> = {
            let mut l: Vec<_> = s.list().into_iter().map(|x| (x.id, x.mood)).collect();
            l.sort_by(|a, b| a.0.cmp(&b.0));
            l
        };
        assert_eq!(left, [("chatty".into(), Mood::Working), ("long".into(), Mood::Working), ("stopped".into(), Mood::Idle)]);
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
    fn only_so_many_sessions_are_followed_the_quietest_making_room() {
        let mut s = Sessions::default();
        for i in 0..MAX_SESSIONS + 1 {
            s.apply("SessionStart", &serde_json::json!({ "session_id": format!("s{i}"), "cwd": "/x/a" }), i as u64);
        }
        let ids: Vec<String> = s.list().into_iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), MAX_SESSIONS);
        assert!(!ids.contains(&"s0".to_string()));
        assert!(ids.contains(&"s1".to_string()));
        assert!(ids.contains(&format!("s{MAX_SESSIONS}")));
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
        let read = |s: &mut Sessions, event: &str, p: &Value, now: u64| s.apply_with(event, p, now, tail_of(&t).as_deref());
        read(&mut s, "SessionStart", &event(""), 0);
        assert_eq!(s.list()[0].name, "my-app");
        read(&mut s, "UserPromptSubmit", &event("Fix the   login bug\nplease"), NAME_EVERY_MS);
        assert_eq!(s.list()[0].name, "Fix the login bug please");
        std::fs::write(&transcript, concat!(
            r#"{"type":"user","message":"x"}"#, "\n",
            r#"{"type":"ai-title","aiTitle":"Login bug fix","sessionId":"a"}"#, "\n",
            r#"{"type":"ai-title","aiTitle":"Login bug fixed with tests","sessionId":"a"}"#, "\n",
        )).unwrap();
        read(&mut s, "PreToolUse", &event(""), 2 * NAME_EVERY_MS);
        assert_eq!(s.list()[0].name, "Login bug fixed with tests");
        assert_eq!(s.card().unwrap().title, "Login bug fixed with tests");
        let mut text = std::fs::read_to_string(&transcript).unwrap();
        text.push_str(r#"{"type":"custom-title","customTitle":"Auth","sessionId":"a"}"#);
        text.push('\n');
        text.push_str(r#"{"type":"ai-title","aiTitle":"Something newer","sessionId":"a"}"#);
        std::fs::write(&transcript, text).unwrap();
        read(&mut s, "PostToolUse", &event(""), 3 * NAME_EVERY_MS);
        assert_eq!(s.list()[0].name, "Auth");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn only_a_plain_local_file_is_read_as_a_transcript() {
        let dir = std::env::temp_dir().join(format!("kuchu-tail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("s.jsonl");
        std::fs::write(&file, "{\"type\":\"ai-title\",\"aiTitle\":\"Named\"}\n").unwrap();
        assert_eq!(title_in(&tail_of(&file.display().to_string()).unwrap()).as_deref(), Some("Named"));
        // A folder, a path to another machine or a pipe, a relative path, nothing: never opened.
        assert!(tail_of(&dir.display().to_string()).is_none());
        assert!(tail_of(r"\\server\share\s.jsonl").is_none());
        assert!(tail_of(r"\\.\pipe\kuchu-test").is_none());
        assert!(tail_of("//server/share/s.jsonl").is_none());
        assert!(tail_of("s.jsonl").is_none());
        assert!(tail_of("").is_none());
        #[cfg(unix)]
        {
            // A socket where the file should be: not a plain file.
            let sock = dir.join("s.sock");
            let _listener = std::os::unix::net::UnixListener::bind(&sock).unwrap();
            assert!(tail_of(&sock.display().to_string()).is_none());
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn every_recorded_payload_and_any_garbage_goes_through_without_a_panic() {
        let mut s = Sessions::default();
        let dir = format!("{}/../../fixtures/claude-code-2.1.310", env!("CARGO_MANIFEST_DIR"));
        let mut n = 0;
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                let p: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
                let event = text(&p, "hook_event_name");
                s.apply_with(&event, &p, n, tail_of(&text(&p, "transcript_path")).as_deref());
                n += 1;
            }
        }
        assert!(n > 10);
        let garbage = [
            serde_json::json!(null),
            serde_json::json!([]),
            serde_json::json!({ "session_id": 5 }),
            serde_json::json!({ "session_id": "g", "cwd": null, "prompt": [], "tool_name": 3, "tool_input": "x", "tool_use_id": {} }),
            serde_json::json!({ "session_id": "g", "transcript_path": std::env::temp_dir().display().to_string(), "tool_name": "Bash", "tool_use_id": "t" }),
            serde_json::json!({ "session_id": "g", "last_assistant_message": 7, "error": false, "notification_type": 1 }),
        ];
        let events = ["SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse", "PostToolUseFailure", "PermissionRequest", "Notification", "PreCompact", "Stop", "StopFailure", "SessionEnd", "Nothing"];
        for p in &garbage {
            for event in events {
                s.apply_with(event, p, n, Some("not json\n{\"type\":\"assistant\",\"message\":5}\n{\"type\":\"ai-title\"}\n"));
                n += 1;
            }
        }
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
    fn each_step_shows_whether_it_runs_is_done_or_failed() {
        let mut s = Sessions::default();
        s.apply("PreToolUse", &fixture("PreToolUse"), 1);
        assert_eq!(s.card().unwrap().progress, Some(Progress::Running));
        s.apply("PostToolUse", &fixture("PostToolUse"), 2);
        // The step is done but the turn is not: Claude thinks again, and the card says it is still at it.
        let card = s.card().unwrap();
        assert_eq!(card.lines.last().unwrap().progress, Some(Progress::Done));
        assert_eq!((card.line.as_str(), card.progress), (THINKING, Some(Progress::Running)));
        assert_eq!((s.mood(), card.mood), (Mood::Thinking, Mood::Thinking));
        // Two at once, the second failing; a turn cut short leaves none running.
        let id = fixture("PreToolUse")["session_id"].as_str().unwrap().to_string();
        let call = |tool: &str| serde_json::json!({ "session_id": id, "tool_name": "Read", "tool_input": { "file_path": "a.rs" }, "tool_use_id": tool });
        s.apply("PreToolUse", &call("t1"), 3);
        s.apply("PreToolUse", &call("t2"), 4);
        s.apply("PostToolUseFailure", &call("t2"), 5);
        let progress: Vec<_> = s.card().unwrap().lines.iter().map(|l| l.progress).collect();
        assert_eq!(progress, vec![Some(Progress::Done), Some(Progress::Running), Some(Progress::Failed)]);
        // One of them still runs: it is still working, not thinking yet.
        assert_eq!(s.mood(), Mood::Working);
        assert!(s.stopped(&id, 6));
        assert!(s.list()[0].lines.iter().all(|l| l.progress != Some(Progress::Running)));
    }

    #[test]
    fn what_claude_said_before_a_tool_shows_once_between_the_steps() {
        let dir = std::env::temp_dir().join(format!("kuchu-said-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let transcript = dir.join("s.jsonl");
        let t = transcript.display().to_string();
        let write = |lines: &[&str]| std::fs::write(&transcript, lines.join("\n") + "\n").unwrap();
        let prompt = r#"{"type":"user","uuid":"u1","message":{"role":"user","content":"Fix the tests"}}"#;
        let think = r#"{"type":"assistant","uuid":"a1","message":{"content":[{"type":"thinking","thinking":""}]}}"#;
        let said = r#"{"type":"assistant","uuid":"a2","message":{"content":[{"type":"text","text":"I'll run the **tests** first.\nThen fix them."}]}}"#;
        let call = r#"{"type":"assistant","uuid":"a3","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{}}]}}"#;
        let call1b = r#"{"type":"assistant","uuid":"a3b","message":{"content":[{"type":"tool_use","id":"t1b","name":"Bash","input":{}}]}}"#;
        let result = r#"{"type":"user","uuid":"u2","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"ok"}]}}"#;
        let call2 = r#"{"type":"assistant","uuid":"a4","message":{"content":[{"type":"tool_use","id":"t2","name":"Read","input":{}}]}}"#;
        let said3 = r#"{"type":"assistant","uuid":"a5","message":{"content":[{"type":"text","text":"Now the [lint](x.md)."}]}}"#;
        let call3 = r#"{"type":"assistant","uuid":"a6","message":{"content":[{"type":"tool_use","id":"t3","name":"Bash","input":{}}]}}"#;
        let tool = |id: &str, cmd: &str| serde_json::json!({ "session_id": "a", "cwd": "/x/a", "transcript_path": t, "tool_name": "Bash", "tool_input": { "command": cmd }, "tool_use_id": id });
        let texts = |s: &Sessions| s.list()[0].lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>();
        let read = |s: &mut Sessions, event: &str, p: &Value, now: u64| s.apply_with(event, p, now, tail_of(&t).as_deref());
        let mut s = Sessions::default();
        // Words, then two tools at once: the words show once, before the first.
        write(&[prompt, think, said, call, call1b]);
        read(&mut s, "PreToolUse", &tool("t1", "npm test"), 1);
        read(&mut s, "PreToolUse", &tool("t1b", "npm run lint"), 2);
        assert_eq!(texts(&s), ["I'll run the tests first.", "npm test", "npm run lint"]);
        assert_eq!(s.list()[0].lines[0].progress, None);
        // A tool right after another's result, nothing said between: no words, and the old ones are not shown again.
        write(&[prompt, think, said, call, call1b, result, call2]);
        read(&mut s, "PostToolUse", &tool("t1", "npm test"), 3);
        read(&mut s, "PreToolUse", &tool("t2", "git status"), 4);
        assert_eq!(texts(&s), ["npm test", "npm run lint", "git status"]);
        // The transcript written only after the tool started: its words come when it ends, before its step.
        read(&mut s, "PreToolUse", &tool("t3", "npm run lint --fix"), 5);
        write(&[prompt, think, said, call, call1b, result, call2, said3, call3]);
        read(&mut s, "PostToolUse", &tool("t3", "npm run lint --fix"), 6);
        assert_eq!(texts(&s), ["git status", "Now the lint.", "npm run lint --fix"]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn what_cannot_be_seen_is_spelled_out() {
        assert_eq!(plain("a\u{202E}b\u{200B}c\u{7}d\u{E0041}"), "a⟨RLO⟩b⟨ZWSP⟩c⟨U+0007⟩d⟨U+E0041⟩");
        assert_eq!(plain("one\r\ntwo\tthree\rfour"), "one\ntwo\tthree⟨U+000D⟩four");
        assert_eq!(plain("naïve 日本 👨‍👩‍👧 ok"), "naïve 日本 👨‍👩‍👧 ok");
        assert_eq!(step("Bash", &serde_json::json!({ "command": "echo \u{202E}gnp.eruces" })), "echo ⟨RLO⟩gnp.eruces");
    }

    #[test]
    fn each_tool_reads_as_a_step() {
        assert_eq!(step("Edit", &serde_json::json!({ "file_path": "C:\\p\\src\\app.ts" })), "Editing app.ts");
        assert_eq!(step("WebFetch", &serde_json::json!({ "url": "https://docs.rs/tauri/latest" })), "Reading docs.rs");
        assert_eq!(step("mcp__github__create_issue", &Value::Null), "Using github: create issue");
        assert_eq!(step("Bash", &serde_json::json!({ "command": "npm   test\n--watch" })), "npm test --watch");
    }
}
