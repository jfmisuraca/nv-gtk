// Top-level build file. Versions are declared here via the plugin aliases
// applied in :app; keep them in one place so Dependabot-style bumps are trivial.
plugins {
    // 8.6.1 is the minimum AGP whose metadata check accepts material3 1.4.0 /
    // Compose 1.11 (AAR minAndroidGradlePluginVersion = 8.6.0); 8.5.2 fails on it.
    id("com.android.application") version "8.6.1" apply false
    id("org.jetbrains.kotlin.android") version "2.0.21" apply false
    id("org.jetbrains.kotlin.plugin.compose") version "2.0.21" apply false
}
