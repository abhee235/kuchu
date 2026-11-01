# Kuchu — plan

Kuchu is a small desk pet that shows what your Claude Code sessions are doing. It sits above your windows, thinks,
works, waves when a session needs you and hops when one is done; its card says which project and what is happening
now, and a permission request can be answered from it.

## Decisions (2026-11-01)

- **Platforms:** Windows and macOS first; Linux later.
- **The app:** Tauri 2. The screens are React and TypeScript (the pet is drawn in SVG); the core is Rust (the window,
  the tray, the pipe, the settings writer). It uses the web view the system already has, so the installer stays small
  and the app light while it runs all day.
- **The relay:** `kuchu-relay`, one small native program Claude Code runs for each hook event. Nothing to install
  besides Kuchu: no Node, no Python. Measured on Windows (median of 30 runs, an event on stdin): 16 ms for the
  native relay, 49 ms for the same in Node, 103 ms in Python; Claude Code waits for it before and after every tool.
- **The pet:** a desk robot with a wink on its screen; its moods, sizes, card and tones.
- **Licence:** the code under MIT; the name, the character and the sounds reserved (LICENSE-ASSETS.md).

## Rules from day one

**Never slow Claude Code down**
- The relay waits at most 300 ms to reach Kuchu, then exits without a word; it always exits 0 (exit 2 would block
  Claude Code). Only a permission request waits for a person.
- Large fields (`tool_response`, file contents) never leave the relay.
- The relay lives at a fixed path per user and is replaced atomically when Kuchu updates.

**Claude Code's settings are the user's**
- One writer for `settings.json` (honouring `CLAUDE_CONFIG_DIR`): a file it cannot read or parse is refused, never
  taken as empty; a dated backup is taken first; the diff is shown and written only after a click, and only if the
  file is unchanged since; the write goes to a temporary file and is renamed over the old one.
- Only Kuchu's own entries, marked as such, are ever added or removed. Outdated entries are noticed and offered for
  repair. A project that switches all hooks off is noticed and said.
- Uninstalling removes Kuchu's hooks first, and a hook whose relay is missing does nothing.

**Every session, kept apart**
- Everything is keyed by `session_id`, from any terminal or editor. A session that goes quiet expires; every timer
  belongs to a session or a request.

**Approvals done safely**
- Only a click allows; running out of time never denies, it leaves the question to the terminal.
- The time limits nest: Kuchu 108 s, the relay 110 s, the hook 120 s. A card must be on screen within 800 ms or the
  request goes back to the terminal; a click in the card's first 600 ms does not count. Up to five requests queue.
- A request answered in the terminal leaves Kuchu at once.
- "Always" uses the scope Claude Code itself offers (`permission_suggestions`). Questions send their answers back
  with the original input; approving a plan sends the plan back.

**A window that never gets in the way**
- It never takes the focus; only the pet and its card take the mouse, everywhere else clicks go through. There is
  always a way back to it (the tray, and a shortcut that is on by default).
- Each monitor's scale and work area are respected; after sleep it is where it was.
- Idle costs nothing: no drawing, no polling while hidden, sound opened only while a tone plays.

**Tested against the real thing**
- Hook payloads recorded from each Claude Code version (`fixtures/`), replayed in tests.
- The pipe is reachable only by the same user and survives bursts of connections.

## Build order

0. **Record what Claude Code sends** ✅ (2026-11-01, Claude Code 2.1.310): see [docs/hooks.md](docs/hooks.md).
1. **The pet on the screen** ✅: the Tauri app, the robot, its window and tray; sizes, see-through, tones; its eyes
   follow the pointer.
2. **Sessions** ✅: the relay and the pipe (a named pipe per user on Windows, a socket in a private folder elsewhere),
   one record per session, the card beside the pet (one line, a few lines, or nothing), the sessions list (a click on
   the pet, Ctrl+Alt+K, the tray), Settings, and connecting to Claude Code with the change shown and a copy kept.
   Checked live on Claude Code 2.1.310: a session's steps on the card as it worked, then done.
3. **Approvals** ✅: Allow, Deny, Claude Code's own "don't ask again" offer, and "In Claude Code" to hand it back;
   questions with one choice; plans sent back when approved. Checked live: a Write asked for, allowed from the card,
   and written. Questions and plans are checked against recordings before the first release.
4. **Polish and shipping:** settings, the shortcut, several sessions, sleep and monitors, the installers (Windows,
   then macOS on GitHub's Mac machines), signing before the first public build.

## Later

Other coding agents; opening the right terminal; WSL and SSH sessions; Linux.
