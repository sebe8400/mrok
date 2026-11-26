#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

cargo build --release --locked

APP="dist/MROK.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp target/release/mrok "$APP/Contents/MacOS/mrok"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>MROK</string>
    <key>CFBundleDisplayName</key>
    <string>MROK // ZAKLAD</string>
    <key>CFBundleIdentifier</key>
    <string>com.example.mrok</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleExecutable</key>
    <string>mrok</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

# Ad-hoc podpis: wystarcza do uruchomienia lokalnie. Do dystrybucji
# zrob porzadne codesign + notaryzacje.
codesign --force --deep --sign - "$APP" >/dev/null 2>&1 || true

echo "Gotowe: $APP"
