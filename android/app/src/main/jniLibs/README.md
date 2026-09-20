# jniLibs — prebuilt `libnv_core.so` per ABI (gitignored, machine-built)

These binaries are NOT committed. Rebuild them from the repo root whenever
`nv-core` changes:

```sh
cargo ndk -t arm64-v8a -t x86_64 -o android/app/src/main/jniLibs \
  build -p nv_core --lib
```

Requires `rustup target add aarch64-linux-android x86_64-linux-android`,
`cargo-ndk`, and `ANDROID_HOME` pointing at an NDK (27/28 tested).
The `:app` module keeps only `arm64-v8a` + `x86_64` (`abiFilters` in
`app/build.gradle.kts`): physical devices and the emulator, no 32-bit.
