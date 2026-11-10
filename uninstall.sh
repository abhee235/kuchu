#!/bin/sh
# Uninstalls Kuchu from this Mac: `kuchu uninstall` (it no longer starts when you sign in, and its relay, settings and
# data go), then Kuchu.app itself, from ~/Applications or /Applications.
#
#   curl -fsSL https://raw.githubusercontent.com/abhee235/kuchu/main/uninstall.sh | sh
#
# Kuchu changes Claude Code's settings only on your click. If it is still connected, its Settings open: click
# Disconnect there, then run this again. The copies Kuchu kept of Claude Code's settings.json stay, for you.

set -eu

REPO="https://github.com/abhee235/kuchu"

say() { printf '%s\n' "$*"; }
fail() {
    printf 'Kuchu was not uninstalled: %s\n' "$*" >&2
    exit 1
}

# All in one function, called on the last line: a script cut short on its way here runs nothing.
main() {
    [ "$(uname -s)" = Darwin ] || fail "this script is for macOS. For Windows, see $REPO#install"

    app=""
    for candidate in "$HOME/Applications/Kuchu.app" /Applications/Kuchu.app; do
        if [ -x "$candidate/Contents/MacOS/kuchu" ]; then
            app=$candidate
            break
        fi
    done
    [ -n "$app" ] || fail "there is no Kuchu.app in ~/Applications or /Applications." \
        "Elsewhere, run its \`kuchu uninstall\`, then delete it."

    if ! "$app/Contents/MacOS/kuchu" uninstall; then
        say ""
        say "When that is done, run this command again."
        exit 1
    fi

    rm -rf "$app" || fail "$app could not be deleted. Delete it yourself."
    say "Deleted: $app"
}

main "$@"
