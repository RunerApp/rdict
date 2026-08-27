#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CORE_DIR="$REPO_ROOT/core"
SWIFT_LIB_DIR="$REPO_ROOT/swift"
APP_DIR="$REPO_ROOT/apps/apple"

echo "=== 1. Build Rust staticlib for iOS simulator (release) ==="
cargo build --manifest-path "$CORE_DIR/Cargo.toml" --release --target aarch64-apple-ios-sim

echo "=== 2. Copy librdict.a to Swift binding library ==="
cp "$CORE_DIR/target/aarch64-apple-ios-sim/release/librdict.a" "$SWIFT_LIB_DIR/lib/librdict.a"

echo "=== 3. Generate Xcode project ==="
cd "$APP_DIR"
xcodegen generate 2>&1

echo "=== 4. Build for iOS simulator ==="
xcodebuild -project RdictApp.xcodeproj \
  -scheme RdictApp \
  -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath "$APP_DIR/.build/ios" \
  -sdk iphonesimulator \
  CODE_SIGNING_ALLOWED=NO \
  build 2>&1 | tail -30

echo ""
echo "Done! iOS build complete."
echo ""
echo "To run on simulator, open this in YOUR terminal (not logoscode):"
echo "  open -a Simulator"
echo "  xcodebuild -scheme RdictApp -destination 'platform=iOS Simulator,name=iPhone 15' -package-path $APP_DIR -derivedDataPath $APP_DIR/.build/ios build test"

