#!/usr/bin/env bash
# Packs Kuchu for the Microsoft Store, from release builds: for each CPU a folder with Kuchu, its relay, the images and
# the manifest, made into an MSIX; then one bundle of them, which is what goes to Partner Center. Unsigned: the Store
# signs what it publishes. The package's name and publisher come from Partner Center (Product identity).
#
# From the repository's root, on Windows with the Windows SDK, after `cargo build --release --target <t>` of both:
#   STORE_NAME=… STORE_PUBLISHER='CN=…' STORE_PUBLISHER_NAME=… \
#     bash app/src-tauri/windows/store/pack.sh <out> x86_64-pc-windows-msvc:x64 aarch64-pc-windows-msvc:arm64
set -euo pipefail
export MSYS_NO_PATHCONV=1

out="$1"
shift
here="$(dirname "$0")"
version="$(node -p "require('./app/src-tauri/tauri.conf.json').version").0"
sdk="$(ls -d "/c/Program Files (x86)/Windows Kits/10/bin/10."*/x64 | sort -V | tail -1)"

rm -rf "$out"
mkdir -p "$out/msix"
for build in "$@"; do
  target="${build%%:*}"
  arch="${build##*:}"
  layout="$out/layout-$arch"
  mkdir -p "$layout/Assets"
  cp "target/$target/release/kuchu.exe" "target/$target/release/kuchu-relay.exe" LICENSE LICENSE-ASSETS.md "$layout/"
  cp "$here"/Assets/*.png "$layout/Assets/"
  sed -e "s|@NAME@|$STORE_NAME|" -e "s|@PUBLISHER@|$STORE_PUBLISHER|" -e "s|@PUBLISHER_NAME@|$STORE_PUBLISHER_NAME|" \
    -e "s|@VERSION@|$version|" -e "s|@ARCH@|$arch|" "$here/AppxManifest.xml" > "$layout/AppxManifest.xml"
  "$sdk/makeappx.exe" pack /o /d "$(cygpath -w "$layout")" /p "$(cygpath -w "$out/msix/kuchu-$arch.msix")"
done
"$sdk/makeappx.exe" bundle /o /bv "$version" /d "$(cygpath -w "$out/msix")" /p "$(cygpath -w "$out/kuchu-store.msixbundle")"
echo "$out/kuchu-store.msixbundle"
