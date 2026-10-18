#!/bin/sh
# Builds the SwiftUI reference app for the iOS simulator, installs it on the booted simulator
# and launches it. Usage: tools/swiftui-reference/build-ios.sh [controls|list|tabs|alert]
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
out="${WINGS_REFERENCE_OUT:-$here/build}"
app="$out/WingsReference-ios.app"
mkdir -p "$app"
xcrun --sdk iphonesimulator swiftc -parse-as-library -O \
    -target "$(uname -m)-apple-ios26.0-simulator" \
    -sdk "$(xcrun --sdk iphonesimulator --show-sdk-path)" \
    "$here/Reference.swift" -o "$app/WingsReference"
cat > "$app/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>dev.wings.reference</string>
<key>CFBundleName</key><string>WingsReference</string>
<key>CFBundleExecutable</key><string>WingsReference</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleShortVersionString</key><string>1.0</string>
<key>MinimumOSVersion</key><string>26.0</string>
<key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
<key>UILaunchScreen</key><dict/>
<key>UIApplicationSceneManifest</key><dict><key>UIApplicationSupportsMultipleScenes</key><true/></dict>
</dict></plist>
PLIST
xcrun simctl install booted "$app"
xcrun simctl terminate booted dev.wings.reference 2>/dev/null || true
xcrun simctl launch --console-pty booted dev.wings.reference "${1:-controls}"
