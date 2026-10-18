#!/bin/sh
# Builds and runs the SwiftUI reference app on macOS.
# Usage: tools/swiftui-reference/build-macos.sh [controls|list|tabs|alert] [seconds]
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
out="${WINGS_REFERENCE_OUT:-$here/build}"
mkdir -p "$out/WingsReference.app/Contents/MacOS"
xcrun swiftc -parse-as-library -O \
    -target "$(uname -m)-apple-macos26.0" \
    -sdk "$(xcrun --sdk macosx --show-sdk-path)" \
    "$here/Reference.swift" -o "$out/WingsReference.app/Contents/MacOS/WingsReference"
cat > "$out/WingsReference.app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>dev.wings.reference</string>
<key>CFBundleName</key><string>WingsReference</string>
<key>CFBundleExecutable</key><string>WingsReference</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSPrincipalClass</key><string>NSApplication</string>
</dict></plist>
PLIST
page="${1:-controls}"
seconds="${2:-4}"
"$out/WingsReference.app/Contents/MacOS/WingsReference" "$page" &
pid=$!
sleep "$seconds"
kill "$pid" 2>/dev/null || true
