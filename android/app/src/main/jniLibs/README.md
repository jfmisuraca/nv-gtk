# jniLibs — prebuilt `libnv_core.so` per ABI (gitignored, machine-built)

These binaries are NOT committed. Rebuild them from the repo root whenever
`nv-core` changes:

```sh
cargo ndk -t arm64-v8a -t x86_64 -o android/app/src/main/jniLibs \
  build -p nv_core --lib
```

This is mandatory — not optional — whenever the UniFFI surface
(`nv-core/src/ffi.rs`) gains methods: the generated Kotlin bindings look up
every symbol at load time (`Native.register`), so a stale `.so` crashes the
app on launch with `UnsatisfiedLinkError`. Bindings (`uniffi-bindgen`) and
`.so` must always be regenerated together.

Requires `rustup target add aarch64-linux-android x86_64-linux-android`,
`cargo-ndk`, and `ANDROID_HOME` pointing at an NDK (27/28 tested).
The `:app` module keeps only `arm64-v8a` + `x86_64` (`abiFilters` in
`app/build.gradle.kts`): physical devices and the emulator, no 32-bit.
