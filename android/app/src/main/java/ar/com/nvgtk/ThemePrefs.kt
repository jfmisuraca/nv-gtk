package ar.com.nvgtk

import android.content.Context
import android.content.SharedPreferences

/**
 * Minimal SharedPreferences-backed theme preference (T8).
 *
 * Persists the [NvTheme.id] string under [KEY_THEME]. The default is
 * [NvTheme.Wallpaper], which [colorSchemeFor] resolves to Catppuccin below
 * API 31 — preserving today's effective behavior there.
 *
 * No new dependency: plain SharedPreferences, not DataStore.
 */
object ThemePrefs {
    const val PREFS_NAME = "nv_gtk_prefs"
    const val KEY_THEME = "theme"
    val DEFAULT_THEME: NvTheme = NvTheme.Wallpaper

    private fun prefs(context: Context): SharedPreferences =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun load(context: Context): NvTheme {
        val id = prefs(context).getString(KEY_THEME, null) ?: return DEFAULT_THEME
        return NvTheme.fromPersisted(id)
    }

    fun save(context: Context, theme: NvTheme) {
        prefs(context).edit().putString(KEY_THEME, theme.id).apply()
    }
}
