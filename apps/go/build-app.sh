#!/bin/bash
# build-app.sh — build the rdict Go app and package it as Rdict.app (+ DMG)
# Usage: apps/go/build-app.sh [output-dir] [--dmg]
#   output-dir defaults to /Applications (installs, replacing Rdict.app)
#   Any other dir receives a copy named Rdict.app without touching /Applications.
#   --dmg also writes Rdict.dmg (drag-to-Applications layout) next to it.

set -euo pipefail
cd "$(dirname "$0")"
OUT_DIR="/Applications"
WANT_DMG=0
for arg in "$@"; do
	case "$arg" in
		--dmg) WANT_DMG=1 ;;
		*) OUT_DIR="$arg" ;;
	esac
done
APP_NAME="Rdict.app"
BUNDLE_ID="dev.rdict.app"
ICON_SRC="../apple/RdictApp.app/Contents/Resources/AppIcon.icns"

GOPROXY=https://goproxy.cn,direct CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o /tmp/rdictapp-build .

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
APP="$STAGE/$APP_NAME"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp /tmp/rdictapp-build "$APP/Contents/MacOS/rdictapp"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key>
	<string>rdictapp</string>
	<key>CFBundleIdentifier</key>
	<string>$BUNDLE_ID</string>
	<key>CFBundleName</key>
	<string>Rdict</string>
	<key>CFBundleDisplayName</key>
	<string>Rdict</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>0.1.0</string>
	<key>CFBundleVersion</key>
	<string>1</string>
	<key>CFBundleIconFile</key>
	<string>AppIcon</string>
	<key>LSMinimumSystemVersion</key>
	<string>12.0</string>
	<key>NSHighResolutionCapable</key>
	<true/>
</dict>
</plist>
EOF

cp "$ICON_SRC" "$APP/Contents/Resources/AppIcon.icns"

# Ad-hoc signature: runs locally; distribution needs Developer ID + notarization.
codesign --force --sign - --timestamp=none "$APP"

rm -rf "$OUT_DIR/$APP_NAME"
cp -R "$APP" "$OUT_DIR/$APP_NAME"
echo "Installed $OUT_DIR/$APP_NAME"
if [ "$WANT_DMG" = 1 ]; then
	# DMG: writable image, Finder icon layout (app + Applications link),
	# then compress. Layout needs a GUI session (osascript/Finder).
	DMG_RW="$STAGE/Rdict-rw.dmg"
	DMG_ROOT="$STAGE/dmgroot"
	mkdir -p "$DMG_ROOT"
	cp -R "$APP" "$DMG_ROOT/"
	ln -s /Applications "$DMG_ROOT/Applications"
	hdiutil create -volname "Rdict" -srcfolder "$DMG_ROOT" -ov -format UDRW "$DMG_RW" >/dev/null
	DEV="$(hdiutil attach -readwrite -noverify -noautoopen "$DMG_RW" | grep -o '/Volumes/Rdict' | head -1)"
	osascript <<'OSA' >/dev/null
tell application "Finder"
	tell disk "Rdict"
		open
		set current view of container window to icon view
		set toolbar visible of container window to false
		set statusbar visible of container window to false
		set the bounds of container window to {200, 120, 660, 480}
		set theViewOptions to the icon view options of container window
		set arrangement of theViewOptions to not arranged
		set icon size of theViewOptions to 96
		set position of item "Rdict" of container window to {140, 160}
		set position of item "Applications" of container window to {400, 160}
		close
		open
		update without registering applications
		delay 3
	end tell
end tell
OSA
	hdiutil detach "$DEV" >/dev/null
	hdiutil convert "$DMG_RW" -format UDZO -imagekey zlib-level=9 -ov -o "$OUT_DIR/Rdict.dmg" >/dev/null
	echo "Installed $OUT_DIR/Rdict.dmg"
fi
