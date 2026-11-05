#!/usr/bin/env bash
# Builds Zed.app (terax branch) from a release-fast build and zips it.
#   cargo build --profile release-fast -p zed -p cli
#   terax/bundle.sh            -> target/release-fast/bundle/Zed-<version>-terax-<arch>.zip
# Optional: TERAX_BUILD (release suffix, default 1).
set -euo pipefail
cd "$(dirname "$0")/.."

target="${CARGO_TARGET_DIR:-target}/release-fast"
out="$target/bundle"
app="$out/Zed.app"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/zed/Cargo.toml | head -1)
build="${TERAX_BUILD:-1}"
arch=$(uname -m)

rm -rf "$out"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$target/zed" "$target/cli" "$app/Contents/MacOS/"
cp crates/zed/resources/Document.icns "$app/Contents/Resources/"

# App icon from Zed's own PNGs.
iconset="$out/Zed.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  sips -z $size $size crates/zed/resources/app-icon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) crates/zed/resources/app-icon@2x.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/Zed.icns"
rm -rf "$iconset"

cp terax/Info.plist "$app/Contents/Info.plist"
/usr/libexec/PlistBuddy \
  -c "Set :CFBundleShortVersionString $version" \
  -c "Set :CFBundleVersion $version.terax.$build" \
  "$app/Contents/Info.plist"

# Ad-hoc signature: enough to run on this Mac. A download from GitHub needs
# `xattr -dr com.apple.quarantine Zed.app` once (not notarized).
codesign --force --deep --sign - "$app"

zip="$out/Zed-$version-terax.$build-$arch.zip"
ditto -c -k --keepParent "$app" "$zip"
echo "$zip"
