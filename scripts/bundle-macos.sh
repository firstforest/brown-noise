#!/usr/bin/env bash
# release ビルドを macOS のメニューバー専用 .app に包む。
set -euo pipefail

cd "$(dirname "$0")/.."

name="Brown Noise"
bin="brown-noise"
identifier="io.github.firstforest.brown-noise"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"

cargo build --release

app="target/release/bundle/${name}.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp "target/release/$bin" "$app/Contents/MacOS/$bin"

# LSUIElement: Dock に出さずメニューバーだけに常駐する
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>${name}</string>
  <key>CFBundleDisplayName</key><string>${name}</string>
  <key>CFBundleIdentifier</key><string>${identifier}</string>
  <key>CFBundleExecutable</key><string>${bin}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

# 署名なしの .app は Apple Silicon で起動できないため、ローカル用に ad-hoc 署名する
codesign --force --sign - "$app"

echo "$app"
