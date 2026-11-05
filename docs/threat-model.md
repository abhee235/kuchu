# Kuchu: threat model

Kuchu stands between Claude Code and the person: it sees every hook event, and a click on it can let a tool run. So
this is what Kuchu must hold against, piece by piece, what already holds, and what is still to do. It is the security
record of the project, reviewed again as the code grows and kept current as the fixes land.

## How this record is kept

- Every threat has a number (`T1`, `T2`, …) that never changes and is never reused; a new review adds the next
  numbers. The register at the end lists them all with a status: **Open**, **Fixed** (with the commit), **Accepted**
  (a risk taken on purpose, with the reason), or **Watching** (nothing to do yet, checked each review).
- Each review is logged at the end with its date, the commit it looked at, and the pieces it read. A review happens
  after each piece lands and before each release; it rereads the pieces that changed and the register in full.
- A fix closes its threat only with a test that would fail if it came back (the list under "Checks to keep").

## The pieces, and who trusts whom

```
Claude Code ──hook──▶ kuchu-relay ──pipe / socket──▶ Kuchu's core ──IPC──▶ the pages (webview)
                             ◀── decision ──              │
                                                          ├──▶ ~/.claude/settings.json      (only on a click)
                                                          ├──▶ claude -p (stream-json)      (conversations)
                                                          └──▶ <app data>/bin/kuchu-relay   (placed at start)
```

**Trusted.** The operating system; Claude Code's own program; Kuchu's own pages (they are Kuchu's code); and the
person's account. Anything already running as the person can do whatever the person can, including editing
`settings.json` to put any hook it likes in. Kuchu cannot stop that, and does not try. What Kuchu must do is **never
widen it**: never give another account, or another machine, a way in; never run with more rights than the person;
and never make Claude Code easier to steer than it already is.

**Half-trusted: what Claude Code sends.** The shape comes from Claude Code; much of the content does not. Everything
the model writes is shaped by whatever it read, and a file or a web page it read may have been written to steer it
(prompt injection). So these are data, never instructions, and never to be trusted on the card:
`tool_input` (the command, the file, the plan, a question's text and options), `description`, `last_assistant_message`,
tool names from MCP servers, and the session's name from the transcript (`ai-title`). Only the person's own `prompt`,
`cwd`, `session_id`, `transcript_path` and `permission_suggestions` come from Claude Code itself.

**Untrusted.** Whatever connects to the pipe (on Windows a named pipe can be opened by any process of any account
on the machine, and its name can be taken by anyone first); whatever a Kuchu-started `claude` process prints; any
path named in a payload; folder names; and the relay's own stdin beyond "it is JSON".

**Out of scope.** An attacker already running as the person; an administrator or root; a compromised Claude Code;
a compromised operating system; someone at the unlocked keyboard.

## What must stay true

1. **Nothing is allowed without a click.** The only way a decision reaches Claude Code is a click on the card for
   that very request. Running out of time, a crash, a closed window or a bad message never allows anything.
2. **The card shows what would happen.** If Allow is on the screen, the whole command, path or plan is on the screen
   with it, as it is, and the session is named by something the model cannot choose.
3. **Kuchu opens no new door.** It listens on nothing but one local pipe for one account; it runs as the person and
   never higher; it makes no network connections; it runs no code it did not ship.
4. **Claude Code never waits on Kuchu's mistakes.** Every path in the relay ends within its deadline, with exit code 0,
   and says nothing unless it has a checked decision. Kuchu being absent, hung or hostile leaves Claude Code exactly as
   it would be without Kuchu.
5. **The person's settings change only as shown, and only on a click.** Only Kuchu's own entries are ever added or
   removed.
6. **Nothing from a session goes to disk.** What Kuchu keeps is in memory, bounded, and gone when it quits.

## Threats, piece by piece

Severity says how much a threat *widens* what an attacker already has. **High**: another account or machine gets
in, or Claude Code can be steered without a click. **Medium**: the person can be misled into a click, or Kuchu can
be hung or filled up from outside. **Low**: a nuisance, or something that only matters once a bigger line is crossed.

### 1. The pipe (hub.rs `serve`, wire.rs `address`)

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T1 | **The name is taken first.** Named pipes are machine-wide. A process of *any* account that creates `\\.\pipe\kuchu-<user>` before Kuchu does receives every hook event of that person (prompts, commands, file paths, the whole input of every permission request) and can answer permission requests. Kuchu, started afterwards, cannot open its pipe and quits with a message that goes nowhere (`eprintln!` in a windowed app). | High | Kuchu refuses a second instance (`first_pipe_instance`) and remote clients. Nothing checks who owns the name. | The relay checks the pipe's owner before sending anything (T6). Kuchu creates the pipe with a DACL for its own account only (`create_with_security_attributes_raw` with `D:P(A;;GA;;;<own SID>)`), so a stale or foreign instance under the name is at least its own account's. When the pipe cannot be opened, a dialog says so instead of a silent exit. |
| T2 | **Anyone on the machine can send events.** Fake sessions and fake cards (five can wait at once, each holding its slot up to 108 s), a fake "done" or "needs you" on a real session, a fake end of a step or turn that takes a real card away (the terminal then asks), an unbounded number of sessions (each view lists them all). Real requests then "go back to the terminal", which is safe, but the feature is denied, and a run of fake cards trains the reflex to click Allow. | Medium | Five waiting at most; one line of at most 1 MiB per connection; 2 s to send it. | Only the relay is heard: the client's process is checked (`GetNamedPipeClientProcessId` → same account, and its image is Kuchu's relay; `peer_cred` on Unix → same uid). Live sessions are capped (oldest dropped), waiting requests capped per session, connections rate-limited. An unknown `v` is dropped. |
| T3 | **Many connections at once.** Each costs a task and up to 1 MiB; a flood from a local process could tie Kuchu up. | Low | Each times out in 2 s. | A cap on connections in flight; a stress test of hundreds of connections (already in the plan). |
| T4 | **Another account reads the pipe.** The default DACL lets Everyone open a named pipe for reading. | Low | The relay needs read and write, so a foreign client gets nothing; Kuchu writes only a decision, and only to the connection that asked. | Covered by the DACL in T1. |

On macOS and Linux the socket lives in `~/.kuchu/` (0700) as a 0600 file, so T1 and T4 reduce to "the same
account", which is out of scope. T2 and T3 apply there as they do on Windows.

What fix 2 did: the pipe is made with `D:P(A;;GA;;;<own SID>)` (`wire::OnlyMe`), so no other account can open it or
hold its next instance; each connection's client process must run as this account (`wire::client_is_me`, from
`GetNamedPipeClientProcessId` and the process's token, `SO_PEERCRED` or `getpeereid` on Unix) or is closed unread;
a message without `v` 1 is dropped; at most 32 sessions are followed (the quietest goes) and 64 connections are open
at once; a pipe that cannot be made is said in a box, naming whether a Kuchu or program of this account, or another
account's program, holds it. What it left out, and why: checking the client's image against the relay's path proves
nothing, since a program of this account can run the relay itself with any input; a cap on waiting requests per
session adds nothing to the cap of five, since session ids are free to invent; a rate limit beyond the connection
cap would only slow this account's own programs. The account is the boundary; inside it, nothing is checked. And
the client is checked by its process, so a relay that has already exited, once Claude Code lets go of it, cannot
be checked and its event is dropped. A relay for an ordinary event exits as soon as it has written; this has not
been seen, and T3's stress test will show whether it happens.

### 2. The relay (relay/main.rs)

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T5 | **Opening the pipe lets the server impersonate the relay.** The relay opens the pipe with the standard library's defaults, which do not set `SECURITY_SQOS_PRESENT`. A server at that name may then impersonate the client, that is, the person, at impersonation level. Together with T1, another account on the machine gets the person's token. `kuchu status`, `quit` and `uninstall` open the pipe the same way (`install::running`). | High | Nothing. | `OpenOptionsExt::security_qos_flags(SECURITY_IDENTIFICATION)` on every open, in one `connect` in `wire` that the relay and the app share, with its own test. |
| T6 | **The relay trusts whoever answers.** It checks only that `behavior` is `allow` or `deny` and passes the rest of the decision through: `updatedInput` replaces the tool's input (a rogue server rewrites the file Claude writes, or the command it runs), `updatedPermissions` writes rules into the person's settings, `message` is shown to the model. | High | `behavior` is checked. | Two checks, both cheap. **Who:** before sending, the pipe's owner SID equals the relay's own (`GetSecurityInfo` on the handle, or `GetNamedPipeServerProcessId` → the process's token user); on Unix, the socket file is owned by the current uid with no group or other bits. **What:** the relay holds the request, so it checks the answer against it: `updatedInput` may differ from `tool_input` only by an `answers` key (a question) or be equal to it (a plan); `updatedPermissions` must be one of the request's own `permission_suggestions`; no other keys. Anything else prints nothing. |
| T7 | **Record mode logs everything.** `kuchu-relay record` writes raw payloads, tool outputs included, to a file in the temp folder. It exists to learn what Claude Code sends; it must never be reachable from a shipped hook by accident. | Low | Only a hand-written settings entry reaches it; `fixtures/raw/` is ignored by git and the kept fixtures are scrubbed. PRIVACY.md says Kuchu writes nothing from sessions to disk, which a release relay that can `record` makes true only by convention. | Build `record` only behind a Cargo feature (`--features record`) so a release relay has no such mode. |
| T8 | **A huge event.** Claude Code's `tool_response` can be megabytes; the relay reads all of stdin before slimming. | Low | Bounded by what Claude Code sends; the line is capped at 512 KiB after slimming; the hub reads at most 1 MiB. | Nothing more. |

### 3. The card: the person in the loop (hub.rs `card`, PetWindow.tsx `ApprovalCard`)

This is the one place where a click has consequences, so the card is a security control and is held to that.

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T9 | **The card hides part of what it approves.** The detail is cut at 600 characters with no mark. A command padded with spaces or blank lines shows a harmless head and nothing of its tail; one longer than 600 characters loses its tail entirely. | High | The detail is a `<pre>` with wrapping and a scroll bar, so what is within 600 characters is all on the card. | Either the whole input is on the card (scrollable), or Allow is not offered and the card says "Too long for the card: answer it in Claude Code". Runs of blank lines and spaces are collapsed with a visible mark, and every cut is marked. |
| T10 | **Characters that lie.** Bidirectional overrides (U+202A to U+202E, U+2066 to U+2069), zero-width characters and C0/C1 controls in a command can reorder or hide what the card shows. | Medium | Rendered as they are. | Anything on an approval card is rendered with these shown as visible marks (for example `⟨RLO⟩`), never as themselves. Shared with the sessions list. |
| T11 | **The session is named by the model.** The card's heading is the session's name, which may be the model's own `ai-title`. A steered session can call itself after a project the person trusts. | Medium | Nothing stops it. | The card shows the folder (`cwd`) under the name, always. |
| T12 | **"Always allow" says neither what nor where.** The button adds Claude Code's first `permission_suggestions` entry without showing it: a rule such as `Bash(curl:*)`, or a folder, with a destination (session, project or user settings). | Medium | The button's label names only the kind (`addRules`, `addDirectories`, `setMode`). | The rule and its scope are on the button or beside it: "Always allow `git commit:*` in this project". |
| T13 | **A click meant for something else.** The card appears at a fixed place, above every window, and takes the mouse as it appears. | Low | A click in the card's first 600 ms does not count; the window is never focusable, so no key can answer; each request is its own card with its own clock. | Keep these; add a test that `approvalShown` precedes any `decide`. |
| T14 | **Answers the card never offered.** A page or a bug sends a choice the request did not have. | Low | A question's answer must be one of its options; a plan is approved only with the plan sent back; "always" needs an offer to exist. | Keep; the relay's own check (T6) covers it a second time. |

What fix 4 did: the detail is no longer cut at 600 characters; it is shown whole (scrolling) up to 20,000, and past
that it is cut with a mark saying how much is missing, the card says "Too long for the card: answer it in Claude
Code", and neither Allow nor the "Always" offer is on it (Deny and "In Claude Code" are); the core holds to that as
well, taking only a "no" for such a card whatever the page sends (`clicked`). Three or more blank lines in a row show
as `⟨59 blank lines⟩`, forty or more blank characters as `⟨100 spaces⟩` (spaces and tabs, and also no-break, wide
and thin spaces and the letters drawn blank: `is_blank`), so the tail of a padded command is in sight. Everything a
card or the sessions list shows passes through `sessions::plain`, which spells out direction marks, zero-width
spaces, line and paragraph separators, soft hyphens, tags and control characters (`⟨RLO⟩`, `⟨U+0007⟩`), the tab and
the line break excepted; an answer to a question is matched against the option as shown and sent as Claude Code gave
it. The session's folder is under its name on every card. The "Always" button keeps its short label, and under the
buttons the card says what it does and where: "“Always allow” allows `Bash(git commit:*)`, in this project, for
you." (session, local, project and user settings each have their words; a deny rule, an empty offer or a mode Kuchu
does not know makes no button). What it left out: the detail for a Write or Edit is still the path, not the content;
the card approves where Claude writes, as Claude Code's own prompt does, and the content is in the terminal. And
20,000 characters is some 500 lines at the card's width, in a box that shows five: all of it is on the card, but only
for someone who scrolls. A lower limit is worth weighing once real commands show how long they run.

### 4. Paths named in payloads (sessions.rs `tail_of`)

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T15 | **`transcript_path` is opened as given, under the hub's lock.** Anything that reaches the pipe (T2) names a path; Kuchu opens it and reads its tail while holding the lock every view and every command needs: 128 KiB for the session's name every 20 s, and since fb00db1 256 KiB for what Claude said, at every tool start and end. A named pipe or FIFO that never answers hangs all of Kuchu: nothing shows on the pet, every permission request waits its full 110 s before going back to the terminal, and once the runtime's threads are all blocked every other hook spends its 300 ms trying to connect. A UNC path makes Kuchu reach another machine with the person's credentials. | Medium | At most 256 KiB is read; only `custom-title`, `ai-title` and the text before a tool call are used; every read goes through `tail_of`, so one guard covers them all. | A path is read only if it is a regular file (`metadata().is_file()`, no reparse point) under Claude Code's own folder (`CLAUDE_CONFIG_DIR` or `~/.claude`), and never a `\\` or `//` path. The read runs on a blocking task outside the lock, with a time limit. |

What fix 3 did: every transcript read goes through `sessions::tail_of`, which reads only an absolute path that is
not `\\` or `//` (another machine, or a pipe) and names a plain file itself (not through a link, not a folder, pipe
or socket), and at most 256 KiB of it; the hub reads it on a thread of its own with a 500 ms limit (`tail_for`),
before it takes its lock, and hands the text in (`Sessions::apply_with`), so nothing is opened under the lock. What
it left out, and why: "under Claude Code's own folder" would make Kuchu blind to a session run with a
`CLAUDE_CONFIG_DIR` that Kuchu, started at sign-in, never sees; and it would guard only against a process of this
account naming some other plain file, which is out of scope and, read as JSON lines for two fields, harmless.

### 5. The person's settings.json (hooks.rs)

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T16 | **The copy is more readable than the original.** `settings.json` may hold secrets (an `env` block, an API key helper). The backup is created with default permissions; on Unix a 0600 original gets a 0644 copy. The new file itself keeps the original's mode. | Medium (Unix) | The new file copies the mode; Windows files inherit the folder's ACL. | The backup is created with the original's mode (0600 when in doubt). |
| T17 | **Copies pile up.** Every Connect or Disconnect leaves one more full copy beside the file, forever. | Low | Each is named with its time. | Keep the last few; say where they are in Settings. |
| T18 | **"Ours" is a substring.** A hook whose command merely contains `kuchu-relay` counts as Kuchu's: it is shown as "installed", protected from the person's view as their own, and removed on uninstall. | Low | The mark is `kuchu-relay`. | Ours is exactly the command Kuchu would write for the relay's path (or one that starts with the quoted relay path, for an older location, which is then "outdated"). |
| T19 | **Written while changed.** The file changes between the diff and the click. | Low | A fingerprint of the bytes shown must match at write time; the write is a temp file renamed over the original, following a symlink to its target. | Keep. |

What fix 5 did: the copy is made for the owner alone and then given the original's mode (`keep_copy`; on Windows
both take the folder's permissions); after each copy, all but the newest five (`KEPT_COPIES`) of Kuchu's own copies
beside the file go (`prune_copies`, which touches nothing but files named `settings.json.kuchu-backup-…`), and
Settings says so where it names the copy; a hook is Kuchu's only if its command is exactly the one Kuchu writes for
that event, or a quoted path to a file named `kuchu-relay` / `kuchu-relay.exe` followed by that event and nothing
else (an older place: "outdated"). A hook that merely mentions the relay is the user's: never counted, never removed.

### 6. Running Claude Code: conversations, and asking what still runs (chat.rs, alive.rs)

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T20 | **What the person types reaches Claude Code's stdin.** | Low | Each message is serialised as one JSON line; a typed `control_request` is just text. | Keep. |
| T21 | **Handing a conversation to a terminal on Windows goes through `cmd /c start`** with the folder in double quotes. A folder name holding `%name%` is expanded by cmd; `"` cannot occur in a Windows name, so a quote cannot be broken out of, but the path can be rewritten. | Low | The session id is Kuchu's own hex; the folder is the one the person picked. | Start Claude Code directly with `CREATE_NEW_CONSOLE` and no shell; failing that, refuse folders holding `%` or `!`. The macOS path escapes for both the shell and AppleScript; the Linux path passes arguments without a shell. |
| T22 | **Ending a turn by pid.** `taskkill /PID … /T /F` five seconds after a stop is asked; if Claude Code exited and the pid was reused, something else dies with its tree. | Low | The run's number is checked first, so the window is tiny. | Keep the `Child` and end it by handle; on Windows put the process in a job object so its tree goes with it. |
| T23 | **Finding `claude`.** The first `claude` on `PATH` is run. | Low | Same account; out of scope. | Say in Settings which one was found. |
| T24 | **Growth.** A conversation's entries grow without bound and the whole conversation is sent to the pages on every change. | Low | Sent at most every 60 ms. | Cap entries kept per conversation; send only what changed. |
| T35 | **Kuchu runs `claude` on its own.** While a busy session has been silent, Kuchu runs `claude agents --json` every 15 s, the first `claude` on `PATH`, with no click behind it; until 84d6f8f Claude Code was only ever started from a click. A relative or empty `PATH` entry would make that a program next to wherever Kuchu was started. | Low | Same account, so out of scope; what it prints is read as a list and nothing else; a hang is cut at 10 s and the process ended. | Skip relative `PATH` entries; resolve `claude` once at start and say in Settings which one. |

What fix 6 did: the hand-off still goes through `cmd /c start` (a process started directly would not get the new
console's input and output), but the folder and Claude Code's path reach cmd as environment variables
(`%KUCHU_CWD%`, `%KUCHU_CLAUDE%`), each in double quotes, with delayed expansion off (`/v:off`). Cmd expands them
once, so a `%` or a `!` in either is just that; it does read what they expand to, and the quotes, which no Windows
name can hold, keep an `&` or a `^` a character of the name. One test shows neither is on the line
(`terminal_command`), another has cmd echo such a folder back as it is. Each conversation's process is put in a
job object as it starts (`Job::holding`); a stop that runs out of its five seconds, or forgetting a busy
conversation, ends the job by handle (`TerminateJobObject`), once the lock on the conversations is let go: the
process and everything it started, never a number another process has by then (if the system refuses the job,
`taskkill` by number stays as the fallback). Closing the job ends nothing: a process let go still finishes by itself
once its input closes. On Unix the process leads its own group and the group is ended, as before. A conversation
keeps its last 400 entries (`MAX_ENTRIES`); the whole conversation is still sent on each change, now bounded by
that. What it left out: the process joins its job a moment after it starts, so a program it starts in that moment
(cmd.exe starting node, for a `claude.cmd` installed with npm) would stay outside it. Kuchu wins that race in
practice; starting the process suspended and joining it first is the full answer.

### 7. The pages (app/src, tauri.conf.json)

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T25 | **Script in a page.** A page can call every command: allow every request, start Claude Code in any folder with any message, rewrite `settings.json`. So any injection into a page is everything. | High if it ever happens | No `dangerouslySetInnerHTML`, no HTML from text (Markdown is reduced to text, never turned into elements); CSP `default-src 'self'` with no `unsafe-eval` and nothing remote; pages load only `index.html`; capabilities are the three the pages need; no `withGlobalTauri`. | Keep every one of these; a test greps the pages for `dangerouslySetInnerHTML`, `innerHTML` and `eval`. Commands that decide, write settings or start Claude Code check the calling window's label. |

### 8. The relay on disk, installing, starting with the session

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T26 | **The hook runs whatever is at the relay's path.** The path is in the person's own folder, so any process of theirs can replace the file and then see every event and answer every request. This is true of any hook, and of Claude Code's own install. | Medium | Kuchu compares the file with its bundled copy at every start and puts it back. Installed from the Store, the bundled copy sits in a folder Windows protects; the placed copy, at `~/.kuchu/bin`, is still the person's, and is there only while Claude Code's settings run it (it goes on Disconnect, at start and on quitting), so an uninstall after Disconnect leaves none of Kuchu behind. | A per-machine install (Program Files, admin-only) with the hooks pointing at the relay there is the only real answer; it costs an elevated update. Offered as an option, not the default. Signed builds either way. |
| T27 | **Running as a service.** Kuchu must never be a Windows service, a launch daemon or anything elevated. As SYSTEM or root it could not show its window, and its pipe would become a step up for every account on the machine: untrusted JSON, paths read, files written, all as SYSTEM. | High (design) | Done: Kuchu starts from the current user's Run key or a LaunchAgent (`install.rs`), the installer installs for the current user, the Store package is a per-user startup task, and the README says it is never a service (be8f21a, 75153eb, aaa38b2, 50c1f9a). | A check that keeps it so: `installMode` is `currentUser`, and no manifest names a service. |
| T28 | **The install itself.** A download swapped on the way. | Medium | The workflow builds from a tag, with pinned actions, and attaches SHA256SUMS, which the README's install checks. Nothing is signed yet, and the README teaches "Run anyway" and "Open Anyway", which also teaches a habit. | Signed Windows and macOS builds before the first public release; build provenance from the release workflow; the future updater verifies signatures and never runs an unsigned file. |
| T34 | **The release and its checksum share an origin.** The README's install downloads the archive and `SHA256SUMS` from the same release page, so the check catches a broken download, not a swapped release; and `releases/latest` moves, so whoever can publish a release reaches every person who asks Claude Code to install or update Kuchu. | Medium | The workflow alone builds releases, from a tag, and leaves a draft to look over first; the README tells Claude Code to stop on a mismatch, never to skip the check, never to use administrator rights, and never to touch `settings.json`. | Provenance attested by the workflow for each file, and verified at install when `gh` is there (`gh attestation verify`); code signing (T28), which ties the file to a key the release page cannot change. |
| T36 | **A second channel: the single-instance hand-off.** On Windows a second `kuchu.exe` hands its arguments to the running one through a window message, which any process in the session can send: `--quit` ends Kuchu, `--settings` opens Settings, anything else shows the pet. | Low | Same account, so out of scope; the arguments do nothing but quit or show windows; nothing is decided, written or started. | Keep it to those; never add an argument that decides, writes or starts Claude Code. A test pins `again` to them. |

What fix 7 did: `kuchu-relay record` exists only in a relay built with `--features record` (T7); a test reads
`tauri.conf.json`, the Store manifest and the installer's steps and fails if the install is not per-user or anything
names a service (T27); a workflow runs `cargo audit` and `npm audit` on every push, pull request and week (T33); the
release workflow attests provenance for every file and for `SHA256SUMS`, and the README's install verifies it with
`gh attestation verify` when GitHub's CLI is signed in, stopping if it fails (T34; proven by 0.1.2);
`claude` is looked for in absolute `PATH` entries only, and `kuchu status` names the program found (T35); the
single-instance hand-off's arguments are three (`handed`: quit, settings, show) and a test pins them (T36). What it
left out, and why: signing (T28) needs a Windows and an Apple signing identity, which is a purchase and an account,
not code; Settings naming the `claude` found (T35) is a page change for later.

### 9. Availability

Kuchu being slow or dead must never slow Claude Code down (the relay fails open, in silence) and should rarely
happen.

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T29 | **One panic ends Kuchu.** The workspace's release profile aborts on panic, and every lock is `lock().unwrap()`: a panic while a lock is held poisons it and the next `unwrap` panics too. | Medium | No `unsafe`; serde_json bounds nesting; every cut is on a character boundary. | Locks are taken with the poison swallowed; tasks that take untrusted input catch unwinds; the hub cannot be hung from outside (T15). A test feeds every fixture, and malformed JSON, through `Sessions::apply` and the relay. |
| T30 | **Kuchu is gone and nobody knows.** Hooks then do nothing, which is right, but the person loses the pet without a word. | Low | Nothing. | Autostart restarts it at login; a crash leaves a note in Kuchu's own folder (never a payload). |

What fixes 3 and 7 did for T29: every lock is taken through `locks::Hold::hold`, which takes a poisoned lock all the
same; a test feeds every recorded payload, and garbage of every shape, through `Sessions::apply_with`. The release
profile no longer aborts on a panic: a panic unwinds, so a task of Kuchu's can fail on its own, and the relay's
`main` catches one, prints nothing (its panic hook is silent) and exits 0, as its rule says.

### 10. What Kuchu keeps and shows

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T31 | **Secrets on the card.** A command on the card may hold a token; the card is above every window, on every desktop, in every screen share. | Low | Settings turn the card and approvals off; the tray hides Kuchu. | A one-key "hide everything" (the shortcut, twice); nothing shown is ever written to disk. |
| T32 | **What is on disk.** Kuchu's own settings hold recent folders; `hooks.json` holds the relay's path; nothing from any session. | Low | As it should be. | Keep; a test that no session text reaches `save`. |

### 11. The supply chain

| | Threat | Severity | Stands today | To do |
|---|---|---|---|---|
| T33 | **A dependency turns.** Cargo and npm packages, and the actions that build the release. | Medium | `Cargo.lock` and `package-lock.json` are committed and built `--locked`; actions pinned to a commit; releases built only from a tag, by the workflow, with their checksums attached; the token can write only the release; few dependencies. | `cargo audit` and `npm audit` in CI; provenance for what the workflow built (T34). |

## What already holds

Worth keeping on purpose, each with a test where there is none:

- The relay always exits 0, says nothing unless it has a checked decision, and each side's time limit is shorter than
  the one it waits on.
- A permission request needs the card on the screen within 800 ms and a click within 108 s; the relay leaving ends it
  at once; running out of time leaves the question to the terminal, never denies.
- The pet's window is never focusable; a click in a card's first 600 ms does not count.
- A question's answer must be one of its options; a plan is approved only with the plan sent back.
- `settings.json` is refused when unreadable, written only against a matching fingerprint, through a temp file and a
  rename, with a copy kept first; only entries that run the relay are touched.
- No network, no HTML from text, a strict CSP, lockfiles committed, raw recordings ignored, kept fixtures scrubbed.
  No `unsafe` but the few system calls in `wire` that read who made the pipe, each wrapped and tested.
- `kuchu setup` never writes `settings.json` (it reads it only to say how things stand); starting at sign-in is the
  current user's Run key or a LaunchAgent, never anything elevated; the installer installs for the current user;
  `uninstall` removes only folders named after Kuchu, and refuses while Kuchu's hooks are still in `settings.json`.
- Releases are built only by the workflow, only from a tag, with actions pinned to a commit, `--locked` builds and a
  token that can write only the release.

## The order of the fixes

Each is one small commit with its tests.

1. **The relay trusts only its own Kuchu** (T5, T6): one `connect` in `wire`, used by the relay and by `kuchu
   status`, that opens with `SECURITY_IDENTIFICATION` and goes on only if the pipe's owner is the relay's own account
   (the socket's owner and mode on Unix); a decision is checked against the request it answers.
2. **The pipe is the account's alone** (T1, T2, T3, T4): a DACL for the current user; the client's process checked
   (`peer_cred` on Unix); caps on sessions, waiting requests and connections; a dialog when the pipe cannot be opened.
3. **Kuchu cannot be hung from outside** (T15, T29): transcript paths checked and read off the lock; locks that
   survive a panic.
4. **The card tells the whole truth** (T9, T10, T11, T12): all of the input or no Allow; cuts and blank runs marked;
   controls and bidi marks made visible; the folder on the card; the rule on the "Always" button.
5. **settings.json copies** (T16, T17, T18): the original's mode; the last few kept; "ours" exact.
6. **Conversations** (T21, T22, T24): a new console without a shell; processes ended by handle and job; entries
   capped.
7. **Starting and shipping** (T7, T26, T27, T28, T33, T34, T35, T36): `record` behind a feature; the checks that keep
   Kuchu per-account; signed builds, provenance and audits in the release workflow; `claude` found once; the hand-off's
   arguments pinned.

## The register

| | Piece | Severity | Status |
|---|---|---|---|
| T1 | Pipe: name taken first | High | Fixed in 76b59e5 |
| T2 | Pipe: anyone can send events | Medium | Fixed in 76b59e5; see what fix 2 left out |
| T3 | Pipe: many connections | Low | Cap in 76b59e5; closes with its stress test, which also shows whether a relay quick to exit is still heard |
| T4 | Pipe: another account reads | Low | Fixed in 76b59e5: T1's DACL |
| T5 | Relay: impersonation on open | High | Fixed in 6de09f5 |
| T6 | Relay: trusts whoever answers | High | Fixed in 6de09f5 |
| T7 | Relay: record mode in release builds | Low | Fixed in 306e661: only with `--features record` |
| T8 | Relay: huge event | Low | Accepted: bounded by Claude Code and the line caps |
| T9 | Card: hides part of the input | High | Fixed in cf1286c |
| T10 | Card: characters that lie | Medium | Fixed in cf1286c |
| T11 | Card: session named by the model | Medium | Fixed in cf1286c |
| T12 | Card: "Always allow" without the rule | Medium | Fixed in cf1286c |
| T13 | Card: a click meant for something else | Low | Watching: the 600 ms guard and the unfocusable window hold |
| T14 | Card: answers never offered | Low | Watching: checked in `decision`; T6 adds the relay's check |
| T15 | Paths: `transcript_path` opened under the lock | Medium | Fixed in 7a1a2e0; see what fix 3 left out |
| T16 | settings.json: copy more readable than the original | Medium | Fixed in 8a91d2c |
| T17 | settings.json: copies pile up | Low | Fixed in 8a91d2c |
| T18 | settings.json: "ours" is a substring | Low | Fixed in 8a91d2c |
| T19 | settings.json: written while changed | Low | Watching: the fingerprint and the rename hold |
| T20 | Conversations: typed text reaches stdin | Low | Watching: one JSON line per message |
| T21 | Conversations: hand-off through `cmd /c start` | Low | Fixed in 5fbd3c9; see what fix 6 did |
| T22 | Conversations: ending a turn by pid | Low | Fixed in 5fbd3c9 on Windows; Unix stays by group; see what fix 6 left out |
| T23 | Conversations: finding `claude` | Low | Accepted: the same account's `PATH` is theirs |
| T24 | Conversations: growth | Low | Fixed in 5fbd3c9: entries capped; whole sent, bounded by the cap |
| T25 | Pages: script in a page | High if ever | Watching: no HTML from text, strict CSP; the grep test to add |
| T26 | On disk: the relay's path is the person's | Medium | Accepted for a per-user install; a per-machine install offered later |
| T27 | Never a service | High (design) | Fixed in be8f21a, 75153eb, aaa38b2, 50c1f9a; its check is in 306e661 |
| T28 | The install itself | Medium | Open: needs a signing identity for Windows and macOS, which is not code; provenance (T34) meanwhile |
| T29 | One panic ends Kuchu | Medium | Fixed in 7a1a2e0 and 306e661: locks, the garbage test, unwinding instead of aborting, the relay catching a panic and still exiting 0 |
| T30 | Kuchu gone without a word | Low | Open |
| T31 | Secrets on the card | Low | Open |
| T32 | What is on disk | Low | Watching: nothing from sessions |
| T33 | A dependency turns | Medium | Fixed in 306e661: `cargo audit` and `npm audit` in a workflow; the rest done in e2766aa |
| T34 | Release and checksum share an origin | Medium | Fixed in 306e661: provenance attested for every file and for SHA256SUMS, verified by the README's install when `gh` is signed in; proven by 0.1.2, all seven of its files verified |
| T35 | Kuchu runs `claude` on its own | Low | Fixed in 306e661: relative and empty PATH entries skipped; `kuchu status` names the program. Settings does not say it yet |
| T36 | The single-instance hand-off | Low | Accepted: same account, harmless arguments; `handed` pins them to quit, settings and show (306e661) |

## Reviews

- **2026-11-01**, at commit `bbe1328` plus the uncommitted tree of that evening (conversations, step marks). Read in
  full: `wire`, `relay`, `hub.rs`, `sessions.rs`, `chat.rs`, `hooks.rs`, `prefs.rs`, `lib.rs`, the pages,
  `tauri.conf.json`, the capabilities, the fixtures. Found T1 to T33. Nothing fixed yet.
- **2026-11-03**, at commit `db788a6`, 32 commits on: step marks and Claude's words from the transcript, the liveness
  check, "+N", the idle life, `kuchu setup`, `status`, `quit` and `uninstall`, starting at sign-in, the Windows
  installer, the Store package, the release workflow, the README's install for Claude Code, the privacy policy. Read
  in full: `install.rs`, `alive.rs`, the workflow, `hooks.nsh`, the Store files, the README's install; as diffs:
  `hub.rs`, `sessions.rs`, `chat.rs`, `lib.rs`, `prefs.rs`, `main.tsx`, `api.ts`, `tauri.conf.json`. Found T34 to
  T36. T15 widened (the transcript is read at every tool start and end). T27 fixed; T28 and T33 part done. The
  approval card and the settings writer did not change.
- **2026-11-03**, later: fixes 1 to 7 written on branch `security` from `db788a6`, each with its tests, uncommitted
  and awaiting review; fixes 1 to 4 also checked live against a running Kuchu (the relay through the pipe, the
  dialog when the pipe is taken, the cards for a rule, a padded command and an over-long one). What each fix left
  out, and why, is under its section. Open: T28 (a signing identity); T3's stress test; T34 until a release proves
  it; Settings naming the `claude` found (T35).
- **2026-11-03**, the review of fixes 1 to 7 before they were committed (6de09f5 to 306e661): read in full as
  diffs, against this record, and their tests run on each commit. Found, and fixed in the fix it belongs to: padding
  of other blank characters (no-break, wide and thin spaces, letters drawn blank) passed the counted runs, and a card
  not shown whole was kept from Allow by the page alone (fix 4); a `!` in a folder reached cmd with delayed expansion
  possibly on, and a stop ended its process under the conversations' lock (fix 6). Checked and holding: a relay run
  from an elevated terminal is still heard (Kuchu reads the token of an elevated process of the same account); the
  pinned actions match their tags; provenance needs a public repository, which this is. Noted, not threats: the end
  of a transcript is now read for every event, where before the name was read every 20 s and Claude's words on tool
  events; what fixes 2, 4 and 6 left out is under each.
- **2026-11-04**, release 0.1.2, the first since fix 7: `gh attestation verify` accepted all seven of its files
  (the four archives and installers for Windows, the two for macOS, `SHA256SUMS`). T34 holds.
- **2026-11-05**, the Microsoft Store's policies (7.20) read against Kuchu. Two changes: from the Store the relay is
  taken out of `~/.kuchu` once no hooks run it, and only then (T26: a check of `settings.json`, never a guess); and a
  page can open the page for reporting an answer from Claude, a fixed address the command takes no argument for (T25:
  a page can open that one page and nothing else).

## Checks to keep

Tests that stand guard, so none of this quietly comes back:

- `wire`: each limit shorter than the one it waits on (exists).
- `relay`: only `allow` and `deny` pass (exists); a decision whose `updatedInput` or `updatedPermissions` the request
  never offered prints nothing; the open sets `SECURITY_IDENTIFICATION`; a pipe owned by another SID is not written to.
- `hub`: a `transcript_path` that is a pipe, a device, a UNC path or outside Claude Code's folder is never opened; a
  client that is not the relay is dropped; the caps hold under a burst.
- `sessions`: every fixture and a corpus of malformed events pass through `apply` without a panic.
- `hooks`: the backup's mode equals the original's; a hook that merely mentions `kuchu-relay` is not "ours".
- pages: no `dangerouslySetInnerHTML`, `innerHTML` or `eval`; `approvalShown` precedes any `decide`; a detail with
  bidi marks renders them visibly.
