// Top-level build file. Versions are declared here via the plugin aliases
// applied in :app; keep them in one place so Dependabot-style bumps are trivial.
plugins {
    // 9.1.1 supports API level 37 (android-37.0); AGP 9 bundles built-in Kotlin
    // support with a runtime dependency on KGP 2.2.10, so the kotlin.android
    // plugin must NOT be applied (AGP registers the kotlin extension itself)
    // and the compose plugin below must match that Kotlin version.
    id("com.android.application") version "9.1.1" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.2.10" apply false
}
