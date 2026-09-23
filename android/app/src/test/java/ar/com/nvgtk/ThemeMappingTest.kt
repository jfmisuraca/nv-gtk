package ar.com.nvgtk

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * T11: mapping + default-preference resolution tests.
 *
 * Pure JVM (no Robolectric, no device): [NvTheme.fromPersisted],
 * [ThemePrefs.DEFAULT_THEME] and [bootBackground] need no Android framework.
 * The `SharedPreferences` round-trip inside [ThemePrefs.load]/`save` cannot
 * run here — no instrumentation harness in scope — so the load path is pinned
 * indirectly: missing key -> [ThemePrefs.DEFAULT_THEME], stored id ->
 * [NvTheme.fromPersisted], both asserted below.
 */
class ThemeMappingTest {

    private fun argb(color: Color): String =
        "0x" + color.toArgb().toUInt().toString(16).uppercase().padStart(8, '0')

    @Test
    fun themeIds_matchDesktopPersistenceStrings() {
        assertEquals("catppuccin", NvTheme.Catppuccin.id)
        assertEquals("dracula", NvTheme.Dracula.id)
        assertEquals("flexoki", NvTheme.Flexoki.id)
        assertEquals("wallpaper", NvTheme.Wallpaper.id)
    }

    @Test
    fun fromPersisted_roundTripsKnownIds() {
        for (theme in NvTheme.entries) {
            assertEquals(theme, NvTheme.fromPersisted(theme.id))
        }
    }

    @Test
    fun fromPersisted_unknownOrBlank_fallsBackToCatppuccin() {
        assertEquals(NvTheme.Catppuccin, NvTheme.fromPersisted("midnight"))
        assertEquals(NvTheme.Catppuccin, NvTheme.fromPersisted(""))
        assertEquals(NvTheme.Catppuccin, NvTheme.fromPersisted("DRACULA"))
    }

    @Test
    fun defaultPreference_isWallpaper() {
        // ThemePrefs.load returns this when no key is stored; Wallpaper
        // resolves to Catppuccin below API 31, preserving today's behavior.
        assertEquals(NvTheme.Wallpaper, ThemePrefs.DEFAULT_THEME)
    }

    @Test
    fun bootBackground_matchesServedSchemeTables() {
        // bootBackground must read the same `background` role colorSchemeFor
        // serves, so the onCreate window background and the first Compose
        // frame always agree.
        assertEquals(DraculaDarkColors.background, bootBackground(NvTheme.Dracula, dark = true))
        assertEquals(DraculaLightColors.background, bootBackground(NvTheme.Dracula, dark = false))
        assertEquals(FlexokiDarkColors.background, bootBackground(NvTheme.Flexoki, dark = true))
        assertEquals(FlexokiLightColors.background, bootBackground(NvTheme.Flexoki, dark = false))
        assertEquals(CatppuccinDarkColors.background, bootBackground(NvTheme.Catppuccin, dark = true))
        assertEquals(CatppuccinLightColors.background, bootBackground(NvTheme.Catppuccin, dark = false))
    }

    @Test
    fun bootBackground_pinsAuthoritativeHexes() {
        assertEquals("0xFF282A36", argb(bootBackground(NvTheme.Dracula, dark = true)))
        assertEquals("0xFFFFFBEB", argb(bootBackground(NvTheme.Dracula, dark = false)))
        assertEquals("0xFF100F0F", argb(bootBackground(NvTheme.Flexoki, dark = true)))
        assertEquals("0xFFFFFCF0", argb(bootBackground(NvTheme.Flexoki, dark = false)))
        assertEquals("0xFF1E1E2E", argb(bootBackground(NvTheme.Catppuccin, dark = true)))
        assertEquals("0xFFEFF1F5", argb(bootBackground(NvTheme.Catppuccin, dark = false)))
    }

    @Test
    fun bootBackground_wallpaper_resolvesToCatppuccinFallback() {
        // The Material You dynamic roles are @Composable and cannot run
        // before setContent, so Wallpaper boots into the Catppuccin fallback.
        assertEquals(bootBackground(NvTheme.Catppuccin, dark = true), bootBackground(NvTheme.Wallpaper, dark = true))
        assertEquals(bootBackground(NvTheme.Catppuccin, dark = false), bootBackground(NvTheme.Wallpaper, dark = false))
    }
}
