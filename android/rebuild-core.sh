#!/bin/sh
# Rebuild everything the APK needs from nv-core: Kotlin bindings + .so files.
# Run from the repo root after ANY change to nv-core (especially ffi.rs).
# New UniFFI methods are looked up at load time (Native.register), so a
# stale .so crashes the app on launch with UnsatisfiedLinkError: bindings
# and .so must always be regenerated TOGETHER.
set -eu
cd "$(dirname "$0")/.."
: "${ANDROID_HOME:?Set ANDROID_HOME to your Android SDK root}"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/28.2.13676358}"
cargo build -p nv_core
cargo run -q -p nv_core --bin uniffi-bindgen -- \
  generate --library target/debug/libnv_core.so \
  --language kotlin --out-dir android-bindings
cp android-bindings/uniffi/nv_core/nv_core.kt \
  android/app/src/main/java/uniffi/nv_core/nv_core.kt
cargo ndk -t arm64-v8a -t x86_64 -o android/app/src/main/jniLibs \
  build -p nv_core --lib
echo "OK - now: cd android && ./gradlew :app:assembleDebug"
