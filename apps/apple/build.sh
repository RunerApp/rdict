#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CORE_DIR="$REPO_ROOT/core"
SWIFT_LIB_DIR="$REPO_ROOT/swift"
APP_DIR="$REPO_ROOT/apps/apple"

echo "=== 1. Build Rust staticlib (release) ==="
cargo build --manifest-path "$CORE_DIR/Cargo.toml" --release

echo "=== 2. Copy librdict.a to Swift binding library ==="
cp "$CORE_DIR/target/release/librdict.a" "$SWIFT_LIB_DIR/lib/"
# SwiftPM does not track librdict.a (linked via unsafeFlags), so a
# changed static lib would NOT trigger a relink — touch a CRdict
# source to force the whole chain (CRdict -> Rdict -> app) to rebuild.
touch "$SWIFT_LIB_DIR/Sources/CRdict/shim.c"

echo "=== 3. Build Swift binding library ==="
SWIFT_DRIVER_USE_SANDBOX=0 swift build --package-path "$SWIFT_LIB_DIR" --disable-sandbox -c release

echo "=== 4. Build Swift app ==="
SWIFT_DRIVER_USE_SANDBOX=0 swift build --package-path "$APP_DIR" --disable-sandbox

echo "=== 5. Build sample dictionary ==="
cargo run --manifest-path "$CORE_DIR/Cargo.toml" --example build_dict

echo "=== 6. Assemble .app bundle ==="
APP="$APP_DIR/RdictApp.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$APP_DIR/.build/debug/RdictApp" "$APP/Contents/MacOS/RdictApp"
# Regenerate icon if script exists (also re-signs — see gen_icon.py)
if [ -f "$REPO_ROOT/scripts/gen_icon.py" ]; then
    python3 "$REPO_ROOT/scripts/gen_icon.py" >/dev/null 2>&1
fi
codesign --force --deep --sign - "$APP"

echo ""
echo "Done!"
echo ""
echo "Sample dictionary: $CORE_DIR/examples/sample.rdict"
echo ""
echo "To install (fresh LS registration, avoids icon cache staleness):"
echo "  rm -rf /Applications/RdictApp.app"
echo "  cp -R $APP /Applications/ && killall Dock Finder && open /Applications/RdictApp.app"
echo ""
echo "Or run the repo copy directly:"
echo "  $APP/Contents/MacOS/RdictApp"
