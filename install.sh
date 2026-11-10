#!/bin/sh
# Installs Kuchu on this Mac, for this account only: the latest release, checked against its SHA-256, put in
# ~/Applications, then `kuchu setup`, which opens Kuchu's Settings, where you connect it to Claude Code.
#
#   curl -fsSL https://raw.githubusercontent.com/abhee235/kuchu/main/install.sh | sh
#
# No administrator rights, and nothing outside your own folders. Run it again to update Kuchu.
#
# Files fetched with curl are not marked as downloaded from the internet, so macOS opens Kuchu without asking. The
# SHA-256 check is what makes sure they are the release's own files.

set -eu

REPO="https://github.com/abhee235/kuchu"

say() { printf '%s\n' "$*"; }
fail() {
    printf 'Kuchu was not installed: %s\n' "$*" >&2
    exit 1
}

# All in one function, called on the last line: a script cut short on its way here runs nothing.
main() {
    [ "$(uname -s)" = Darwin ] || fail "this script is for macOS. For Windows, see $REPO#install"

    # The Mac's own CPU, even from a shell running under Rosetta.
    if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" = 1 ]; then
        asset=kuchu-macos-arm64.tar.gz
    else
        asset=kuchu-macos-x64.tar.gz
    fi

    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
    trap 'exit 1' INT TERM

    say "Downloading $asset, from the latest release..."
    curl -fsSL -o "$work/$asset" "$REPO/releases/latest/download/$asset" ||
        fail "$asset could not be downloaded."
    curl -fsSL -o "$work/SHA256SUMS" "$REPO/releases/latest/download/SHA256SUMS" ||
        fail "the release's SHA256SUMS could not be downloaded."

    expected=$(awk -v file="$asset" '$2 == file { print $1 }' "$work/SHA256SUMS")
    [ -n "$expected" ] || fail "the release's SHA256SUMS has no line for $asset."
    actual=$(shasum -a 256 "$work/$asset" | awk '{ print $1 }')
    [ "$expected" = "$actual" ] || fail "$asset does not match its SHA-256 in the release, so it was deleted."
    say "Checked: its SHA-256 is the release's."

    # With GitHub's CLI signed in, also that the release workflow built this very file.
    if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
        gh attestation verify "$work/$asset" --repo abhee235/kuchu >/dev/null ||
            fail "GitHub could not confirm that the release workflow built $asset."
        say "Checked: GitHub confirms the release workflow built it."
    fi

    tar -xzf "$work/$asset" -C "$work" || fail "$asset could not be unpacked."
    [ -x "$work/Kuchu.app/Contents/MacOS/kuchu" ] || fail "$asset has no Kuchu.app in it."

    # A Kuchu already running, wherever it was started from, quits before it is replaced.
    "$work/Kuchu.app/Contents/MacOS/kuchu" quit ||
        fail "the Kuchu that is running did not quit. Quit it from its icon in the menu bar, then run this again."

    app="$HOME/Applications/Kuchu.app"
    mkdir -p "$HOME/Applications"
    rm -rf "$app"
    mv "$work/Kuchu.app" "$app"
    say "Installed: $app"

    "$app/Contents/MacOS/kuchu" setup
}

main "$@"
