# Kuchu

A small desk pet for Claude Code. It sits above your windows and shows what your sessions are doing: it thinks while
Claude reads your message, works while tools run, waves when a session needs you and hops when one is done. Its card
says which project and what is happening now, and a permission request can be answered from it.

Windows and macOS first; Linux later. Work in progress: see [PLAN.md](PLAN.md).

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
