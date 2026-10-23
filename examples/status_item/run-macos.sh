#!/bin/sh
set -eu

if [ "$(uname -s)" != "Darwin" ]; then
    printf '%s\n' "The native status item example requires macOS." >&2
    exit 1
fi
case "${1:-}" in
    ""|--build-only) ;;
    *) printf '%s\n' "Usage: $0 [--build-only]" >&2; exit 2 ;;
esac

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
target_dir=${CARGO_TARGET_DIR:-"$root/target"}
case "$target_dir" in
    /*) ;;
    *) target_dir="$root/$target_dir" ;;
esac
cd "$root"
CARGO_TARGET_DIR="$target_dir" cargo build --locked -p status_item

bundle="$target_dir/status-item-example/Status Item Example.app"
mkdir -p "$bundle/Contents/MacOS"
cp "$target_dir/debug/status_item" "$bundle/Contents/MacOS/status_item"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>status_item</string>
  <key>CFBundleIdentifier</key><string>org.gpui-kit.status-item-example</string>
  <key>CFBundleName</key><string>Status Item Example</string>
  <key>CFBundleDisplayName</key><string>Status Item Example</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundleShortVersionString</key><string>0.1</string>
  <key>LSMinimumSystemVersion</key><string>15.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign --force --sign - "$bundle"
printf '%s\n' "$bundle"
if [ "${1:-}" != "--build-only" ]; then
    open "$bundle"
fi
