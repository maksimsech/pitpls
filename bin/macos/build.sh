#!/bin/bash
set -euo pipefail

app_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$app_dir/../.." && pwd)"
mode="${1:-debug}"
case "$mode" in
    bindings|debug) rust_profile=debug; cargo_profile=dev; swift_profile=debug ;;
    release) rust_profile=native; cargo_profile=native; swift_profile=release ;;
    *) echo "Usage: bash bin/macos/build.sh [bindings|debug|release]" >&2; exit 2 ;;
esac

cd "$repo_dir"
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --locked -p macos_bindings --lib --profile "$cargo_profile" --target-dir "$repo_dir/target"
cargo run --locked -p macos_bindings --features bindgen --bin pitpls-bindgen --target-dir "$repo_dir/target" -- \
    generate --library "$repo_dir/target/$rust_profile/libmacos_bindings.dylib" --metadata-no-deps \
    --language swift --config crates/macos_bindings/uniffi.toml --out-dir "$app_dir/.generated"
mkdir -p "$app_dir/Sources/PitCore" "$app_dir/Sources/PitCoreFFI"
cp "$app_dir/.generated/PitCore.swift" "$app_dir/Sources/PitCore/"
cp "$app_dir/.generated/PitCoreFFI.h" "$app_dir/Sources/PitCoreFFI/"
cp "$app_dir/.generated/PitCoreFFI.modulemap" "$app_dir/Sources/PitCoreFFI/module.modulemap"
if [ "$mode" = bindings ]; then exit 0; fi

export PITPLS_RUST_PROFILE="$rust_profile"
export CLANG_MODULE_CACHE_PATH="$app_dir/.build/clang-cache"
export SWIFTPM_MODULECACHE_OVERRIDE="$app_dir/.build/swift-cache"
swift build --package-path "$app_dir" --configuration "$swift_profile" --disable-sandbox
swift_bin="$(swift build --package-path "$app_dir" --configuration "$swift_profile" --show-bin-path --disable-sandbox)"
bundle="$app_dir/.build/Pitpls.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp "$swift_bin/Pitpls" "$bundle/Contents/MacOS/Pitpls"
cp "$app_dir/Resources/Info.plist" "$bundle/Contents/Info.plist"
# Both apps use the icon generated from assets/app-icon.svg.
cp "$repo_dir/bin/desktop/src-tauri/icons/icon.icns" "$bundle/Contents/Resources/AppIcon.icns"
codesign --force --sign - "$bundle"
# Mark the bundle itself as updated so macOS can refresh its cached icon.
touch "$bundle"
echo "Built $bundle"
