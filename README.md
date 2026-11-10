# Kuchu

A small desk pet for Claude Code. It sits above your windows and shows what your sessions are doing: it thinks while
Claude reads your message, works while tools run, waves when a session needs you and hops when one is done. Its card
says which project and what is happening now, and a permission request can be answered from it.

**Website and docs: [abhee235.github.io/kuchu](https://abhee235.github.io/kuchu/)**, with the pet live.

Windows and macOS first; Linux later. Work in progress: see [PLAN.md](PLAN.md).

## Install

Kuchu installs for your account only: no administrator rights, never a service. Pick one way:

- **Ask Claude Code.** Say this, in any session:

  > Install Kuchu from https://github.com/abhee235/kuchu

  It downloads the latest release and checks it, or builds Kuchu from its code if you ask for that.
- **One command.** It downloads the latest release, checks its SHA-256, installs it for your account and runs
  `kuchu setup`. Run it again to update Kuchu. Files fetched this way are not marked as downloaded from the internet,
  so Windows and macOS open Kuchu without a warning. To read a script before you run it, open its link.

  On a Mac, in Terminal ([install.sh](install.sh)):

  ```bash
  curl -fsSL https://raw.githubusercontent.com/abhee235/kuchu/main/install.sh | sh
  ```

  On Windows, in PowerShell ([install.ps1](install.ps1)):

  ```powershell
  irm https://raw.githubusercontent.com/abhee235/kuchu/main/install.ps1 | iex
  ```
- **The Windows installer.** From the [latest release](https://github.com/abhee235/kuchu/releases/latest), download
  `kuchu-windows-x64-setup.exe` (`kuchu-windows-arm64-setup.exe` on an Arm PC) and open it. **It is not code-signed
  yet**, so Windows says "Windows protected your PC": click **More info**, then **Run anyway**. To check the file
  first, compare its SHA-256 (`Get-FileHash` in PowerShell) with the release's `SHA256SUMS`.
- **[The Microsoft Store](https://apps.microsoft.com/detail/9NNK1Q800R84)**, signed by Microsoft, so Windows opens it
  without a warning (x64 for now). It updates through the Store: Windows closes Kuchu for a moment and starts the new
  one.
- **On a Mac**, download `kuchu-macos-arm64.tar.gz` (`-x64` on an Intel Mac), open it and drag `Kuchu.app` into
  Applications. The first time, macOS refuses to open it: in System Settings → Privacy & Security, click
  **Open Anyway**. (The one command above has no such step.) Not yet tried on a Mac.

The first time Kuchu starts, it opens its Settings: click **Connect** there, after reading the change it shows you,
and your Claude Code sessions appear beside it.

Installed with the one command, one command removes Kuchu too. It runs `kuchu uninstall` and then deletes Kuchu. If
Kuchu is still connected to Claude Code, its Settings open instead: click **Disconnect** there, then run the command
again.

```bash
curl -fsSL https://raw.githubusercontent.com/abhee235/kuchu/main/uninstall.sh | sh      # on a Mac, in Terminal
```

```powershell
irm https://raw.githubusercontent.com/abhee235/kuchu/main/uninstall.ps1 | iex           # on Windows, in PowerShell
```

Otherwise, to uninstall Kuchu, first click **Disconnect** in its Settings, so Claude Code stops running its hooks. Then
remove it from Windows' Settings → Apps. Installed with the Windows installer, tick "Delete the application data" to
remove everything (Kuchu asks you to disconnect first if you have not). From the Store, Windows removes Kuchu and its
data, and Kuchu took its relay away when you disconnected. Uninstalled without disconnecting, Kuchu's hooks stay in
Claude Code's `settings.json` (and, from the Store, its relay in the `.kuchu` folder in your home folder). They do
nothing while Kuchu is gone; to remove them, install Kuchu again and click Disconnect, or take out the hooks that run
`kuchu-relay` and delete that folder yourself. The copies Kuchu kept of `settings.json` (`settings.json.kuchu-backup-…`,
beside it) stay too: they are yours to keep or delete.

### Claude Code, this part is for you

Install Kuchu on the person's own computer, for their account only: no administrator rights, no `sudo`, never a
service. There are two ways:

1. **From the latest release**, the usual way: download the archive for this system and CPU, check its SHA-256, unpack
   it into the account's own programs folder, run `kuchu setup`.
2. **From source**, when the person asks for it, or when no release has a file for this system: clone this repository,
   build it, put it in the same folder, run `kuchu setup`.

Two things are the person's, not yours:

- **Their Claude Code settings.** Never edit `~/.claude/settings.json` (or `$CLAUDE_CONFIG_DIR/settings.json`), and
  never add Kuchu's hooks to it in any other way. `kuchu setup` opens Kuchu's Settings, where the person clicks
  **Connect** after Kuchu has shown them the change. That click is how Kuchu gets connected.
- **What gets installed.** If a checksum does not match, stop: delete the download and tell them. Never skip the
  check. Before installing any build tools for a build from source, ask.

On Windows, run the commands below in Bash (Claude Code's shell there is Git Bash). Kuchu's commands write only into a
pipe, as there; from PowerShell, add `| Out-String`.

### 1. From the latest release

The releases are at https://github.com/abhee235/kuchu/releases. Each has:

| File | For |
|---|---|
| `kuchu-windows-x64.zip` | Windows 10 and 11 on Intel or AMD (also runs on Arm, emulated) |
| `kuchu-windows-arm64.zip` | Windows 11 on Arm |
| `kuchu-windows-x64-setup.exe`, `kuchu-windows-arm64-setup.exe` | the same, as an installer for people; use the zip |
| `kuchu-macos-arm64.tar.gz` | macOS 11 or later on Apple silicon |
| `kuchu-macos-x64.tar.gz` | macOS 11 or later on Intel |
| `SHA256SUMS` | the SHA-256 of each file above |

A Windows zip holds `kuchu.exe` and `kuchu-relay.exe`; a macOS archive holds `Kuchu.app`, with the relay inside it.
The relay is the small program Claude Code runs for each hook event: Kuchu copies it to a fixed place of its own, and
the hooks point there.

Windows:

```bash
set -euo pipefail
case "${PROCESSOR_IDENTIFIER:-}" in ARM*) asset=kuchu-windows-arm64.zip ;; *) asset=kuchu-windows-x64.zip ;; esac
repo="https://github.com/abhee235/kuchu"
work="$(mktemp -d)"
curl -fsSL -o "$work/$asset" "$repo/releases/latest/download/$asset"
curl -fsSL -o "$work/SHA256SUMS" "$repo/releases/latest/download/SHA256SUMS"
(cd "$work" && grep "  $asset\$" SHA256SUMS | sha256sum -c -)   # must print "<asset>: OK"
# With GitHub's CLI signed in, also that the release workflow built this very file (it stops here if not):
if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then gh attestation verify "$work/$asset" --repo abhee235/kuchu; fi

dest="$(cygpath -u "$LOCALAPPDATA")/Programs/Kuchu"
[ -x "$dest/kuchu.exe" ] && "$dest/kuchu.exe" quit || true     # an older Kuchu, if one runs
mkdir -p "$dest"
unzip -o -q "$work/$asset" -d "$dest"
rm -rf "$work"
"$dest/kuchu.exe" setup
```

macOS:

```bash
set -euo pipefail
case "$(uname -m)" in arm64) asset=kuchu-macos-arm64.tar.gz ;; *) asset=kuchu-macos-x64.tar.gz ;; esac
repo="https://github.com/abhee235/kuchu"
work="$(mktemp -d)"
curl -fsSL -o "$work/$asset" "$repo/releases/latest/download/$asset"
curl -fsSL -o "$work/SHA256SUMS" "$repo/releases/latest/download/SHA256SUMS"
(cd "$work" && grep "  $asset\$" SHA256SUMS | shasum -a 256 -c -)   # must print "<asset>: OK"
# With GitHub's CLI signed in, also that the release workflow built this very file (it stops here if not):
if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then gh attestation verify "$work/$asset" --repo abhee235/kuchu; fi

app="$HOME/Applications/Kuchu.app"
[ -x "$app/Contents/MacOS/kuchu" ] && "$app/Contents/MacOS/kuchu" quit || true
mkdir -p "$HOME/Applications"
rm -rf "$app"
tar -xzf "$work/$asset" -C "$HOME/Applications"
rm -rf "$work"
"$app/Contents/MacOS/kuchu" setup
```

If a download fails with a 404, this release has no file for this system: build from source instead.

### 2. From source

This needs Git, Rust (stable, from rustup) and Node 20 or later; on Windows also the Visual Studio C++ build tools, on
macOS Xcode's command line tools. Check with `git --version`, `cargo --version` and `node --version`. If any is
missing, tell the person what is needed and ask before installing it: these are large tools to put on their computer.
The first build takes 5 to 10 minutes.

Windows:

```bash
set -euo pipefail
work="$(mktemp -d)"
git clone --depth 1 https://github.com/abhee235/kuchu "$work/kuchu"
cd "$work/kuchu"
cargo build --release --locked -p kuchu-relay
(cd app && npm ci && npx tauri build --no-bundle -- --locked)

dest="$(cygpath -u "$LOCALAPPDATA")/Programs/Kuchu"
[ -x "$dest/kuchu.exe" ] && "$dest/kuchu.exe" quit || true
mkdir -p "$dest"
cp target/release/kuchu.exe target/release/kuchu-relay.exe LICENSE LICENSE-ASSETS.md "$dest/"
cd / && rm -rf "$work"
"$dest/kuchu.exe" setup
```

macOS:

```bash
set -euo pipefail
work="$(mktemp -d)"
git clone --depth 1 https://github.com/abhee235/kuchu "$work/kuchu"
cd "$work/kuchu"
cargo build --release --locked -p kuchu-relay
mkdir -p app/src-tauri/binaries
cp target/release/kuchu-relay "app/src-tauri/binaries/kuchu-relay-$(rustc -vV | sed -n 's/^host: //p')"
config='{"bundle":{"externalBin":["binaries/kuchu-relay"],"macOS":{"signingIdentity":"-"}}}'
(cd app && npm ci && npx tauri build --bundles app --config "$config" -- --locked)

app="$HOME/Applications/Kuchu.app"
[ -x "$app/Contents/MacOS/kuchu" ] && "$app/Contents/MacOS/kuchu" quit || true
mkdir -p "$HOME/Applications"
rm -rf "$app"
cp -R target/release/bundle/macos/Kuchu.app "$HOME/Applications/"
cd / && rm -rf "$work"
"$app/Contents/MacOS/kuchu" setup
```

### After `kuchu setup`

`setup` puts the relay in place, adds Kuchu to the account's own list of apps that start at sign-in, and starts Kuchu
with its Settings open. It prints what it did. Tell the person, in plain words:

- Kuchu is on their screen and starts when they sign in (they can switch that off in Settings → Pet).
- To see their Claude Code sessions beside Kuchu, they click **Connect** in the Settings window. Kuchu shows the exact
  change to their `settings.json` first and keeps a copy of the file. Sessions that are already open show up after
  they are restarted.

Do not click Connect for them. `kuchu status` says how things stand at any time:

```
Kuchu 0.1.0
  Program:                 C:\Users\you\AppData\Local\Programs\Kuchu\kuchu.exe
  Running:                 yes
  Starts when you sign in: yes
  Relay:                   C:\Users\you\AppData\Local\dev.kuchu.app\bin\kuchu-relay.exe
  Claude Code:             connected (C:\Users\you\.claude\settings.json)
```

### Updating

Run the same steps again: they quit the running Kuchu, put the new one in its place, and run `setup`, which replaces
the relay. The person stays connected; nothing in `settings.json` changes.

### Uninstalling

1. Run `kuchu uninstall` (`"$dest/kuchu.exe" uninstall` on Windows, `"$app/Contents/MacOS/kuchu" uninstall` on
   macOS, with `dest` and `app` as above).
2. If it says Kuchu's hooks are still in `settings.json`, it has opened Kuchu's Settings: ask the person to click
   **Disconnect** there (Kuchu shows the change first), then run `kuchu uninstall` again. It removes nothing until
   then.
3. Once it succeeds (Kuchu quit, no longer starting at sign-in, its relay, settings and data gone), delete the program
   itself: `rm -rf "$dest"` on Windows, `rm -rf "$app"` on macOS.

The copies Kuchu kept of `settings.json` (`settings.json.kuchu-backup-…`, beside it) are the person's: leave them.

### Not signed yet

Releases are not code-signed yet. Files downloaded with `curl` as above are not marked as coming from the internet,
so Windows and macOS open them without a warning; the SHA-256 check is what makes sure they are the release's own
files.

## What it does

- **Every session, on its own card.** Beside the pet: the project and what Claude is doing now (or the answer's first
  line once done, for a few seconds), in one line or a few. With several at work, "+N" shows the others, each on its
  own card above it, while you hover it, or until you switch it off once you have clicked it on; a click on one
  switches the card to it. Each card names its project, and on Windows a button brings up the app its session runs in.
  A click on the pet lists every session.
- **Permission requests on the pet.** Allow, Deny, or hand it back to Claude Code. Nothing is allowed without a click.
- **Talk to Claude from Kuchu.** Start a conversation in a folder from the sessions list, answer it, stop it, or take
  it on in a terminal. Sessions started in a terminal or an editor are followed, not typed into: Claude Code takes
  their messages only there.
- **A life of its own.** With no work it stretches, yawns, looks around, taps a foot, flips its face, whistles or
  takes a short walk, and naps after a few quiet minutes. It walks when you drag it and likes being petted.
- **Your choice of pet.** A robot, a cat, a dog, an owl, a frog or a rubber duck, each with its own face, and each doing
  all the same things.
- **Its own Settings.** Your pet, size, see-through under the pointer, sounds, starting at sign-in, what shows beside the pet,
  what a click opens, and connecting to Claude Code: the change to `settings.json` is shown first, and a copy of the
  file is kept.
- **Out of the way.** It never takes the focus, and clicks go through everywhere but the pet and its cards. The tray
  icon and Ctrl+Alt+K always bring it back.
- **Yours alone.** Kuchu is installed for your account only and starts when you sign in, from your account's own list
  of startup apps. It is never a service and never runs with administrator rights.

## How it works

Claude Code runs `kuchu-relay`, a small native program, for each hook event. The relay hands the event to Kuchu and
exits at once; it never holds Claude Code up, and when Kuchu is not running it does nothing. What Claude Code sends
is in [docs/hooks.md](docs/hooks.md).

## Build

Requirements: Rust (stable) and Node 20+.

```
cargo build --release -p kuchu-relay
cd app && npm ci && npx tauri build --no-bundle
```

Both end up in `target/release/`. To install what you built, see "From source" above; `kuchu status` says how Kuchu
stands. Releases are built by `.github/workflows/release.yml` from a version tag.

## Licence

The code is under the [MIT licence](LICENSE). The name Kuchu, the character and the sounds are not: see
[LICENSE-ASSETS.md](LICENSE-ASSETS.md).
