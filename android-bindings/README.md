# android-bindings — generated Kotlin bindings for `nv-core`

Versioned artifact: the Kotlin side of the UniFFI interface declared in
`../nv-core/src/ffi.rs` (proc-macros, no UDL file). Regenerate whenever the
FFI surface changes; do NOT hand-edit `uniffi/`.

## Exact generation command (run from the repo root)

```sh
cargo build -p nv_core
cargo run -p nv_core --bin uniffi-bindgen -- \
  generate --library target/debug/libnv_core.so \
  --language kotlin --out-dir android-bindings
```

Notes:

- Library mode (`--library`) reads UniFFI metadata embedded in the built cdylib,
  so it must run from inside this Cargo workspace.
- The `uniffi-bindgen` binary is the `[[bin]]` target in `nv-core/Cargo.toml`
  (`fn main() { uniffi::uniffi_bindgen_main() }`); it needs `uniffi` with the
  `cli` feature, which `nv-core` already declares.
- `ktlint` auto-format is skipped when ktlint is absent
  (`Warning: Unable to auto-format ...`); the output is still complete.
- Bindgen version must match the `uniffi` dependency in `nv-core/Cargo.toml`
  (currently 0.32.x) — metadata across major versions is not compatible.

## Android library builds (no repo config needed)

Linkers come from the NDK via `cargo-ndk` (preferred; no `.cargo/config.toml`
checked in, since NDK paths are machine-local):

```sh
# $ANDROID_HOME=/home/francisco/Software/android/dev, NDK 27/28 installed
cargo ndk -t arm64-v8a -o <jniLibs-dir> build -p nv_core --lib
cargo ndk -t x86_64    -o <jniLibs-dir> build -p nv_core --lib
```

Equivalent plain-cargo build (used for T1 verification) with the linker passed
via env instead of a config file:

```sh
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=\
"$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang"
cargo build -p nv_core --target aarch64-linux-android
```

Requires `rustup target add aarch64-linux-android x86_64-linux-android`
(one-time) and Java 17 for the later Gradle scaffold (next task, not here).
