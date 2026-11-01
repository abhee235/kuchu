# Kuchu

A small desk pet for Claude Code. It sits above your windows and shows what your sessions are doing: it thinks while
Claude reads your message, works while tools run, waves when a session needs you and hops when one is done. Its card
says which project and what is happening now, and a permission request can be answered from it.

Windows and macOS first; Linux later. Work in progress: see [PLAN.md](PLAN.md).

## What it does

- **Every session, on its own card.** Beside the pet: the project and what Claude is doing now (or the answer's first
  line once done, for a few seconds), in one line or a few. A click on the pet lists every session.
- **Permission requests on the pet.** Allow, Deny, or hand it back to Claude Code. Nothing is allowed without a click.
- **Its own Settings.** Size, see-through under the pointer, sounds, what shows beside the pet, what a click opens,
  and connecting to Claude Code: the change to `settings.json` is shown first, and a copy of the file is kept.
- **Out of the way.** It never takes the focus, and clicks go through everywhere but the pet and its cards. The tray
  icon and Ctrl+Alt+K always bring it back.

## How it works

Claude Code runs `kuchu-relay`, a small native program, for each hook event. The relay hands the event to Kuchu and
exits at once; it never holds Claude Code up, and when Kuchu is not running it does nothing. What Claude Code sends
is in [docs/hooks.md](docs/hooks.md).

## Build

Requirements: Rust (stable) and Node 20+.

```
cargo build --release -p kuchu-relay
```

## Licence

The code is under the [MIT licence](LICENSE). The name Kuchu, the character and the sounds are not: see
[LICENSE-ASSETS.md](LICENSE-ASSETS.md).
