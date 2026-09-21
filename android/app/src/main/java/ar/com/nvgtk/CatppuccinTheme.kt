package ar.com.nvgtk

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

/**
 * Catppuccin ColorSchemes for the NV-GTK mobile app.
 *
 * Latte (light) and Mocha (dark) palettes mapped onto Material3
 * semantic roles. The active scheme is chosen at the call site with
 * [androidx.compose.foundation.isSystemInDarkTheme], so the app follows
 * the OS theme automatically.
 */
val CatppuccinLightColors: ColorScheme = lightColorScheme(
    primary = Color(0xFF8839EF), // mauve
    onPrimary = Color(0xFFEFF1F5), // base
    primaryContainer = Color(0xFFBCC0CC), // surface1
    onPrimaryContainer = Color(0xFF4C4F69), // text
    secondary = Color(0xFF179299), // teal
    onSecondary = Color(0xFFEFF1F5), // base
    secondaryContainer = Color(0xFFCCD0DA), // surface0
    onSecondaryContainer = Color(0xFF4C4F69), // text
    tertiary = Color(0xFF7287FD), // lavender
    onTertiary = Color(0xFFEFF1F5), // base
    tertiaryContainer = Color(0xFFBCC0CC), // surface1
    onTertiaryContainer = Color(0xFF4C4F69), // text
    background = Color(0xFFEFF1F5), // base
    onBackground = Color(0xFF4C4F69), // text
    surface = Color(0xFFEFF1F5), // base
    onSurface = Color(0xFF4C4F69), // text
    surfaceVariant = Color(0xFFCCD0DA), // surface0
    onSurfaceVariant = Color(0xFF8C8FA1), // overlay1
    outline = Color(0xFFACB0BE), // surface2
    outlineVariant = Color(0xFFBCC0CC), // surface1
    error = Color(0xFFD20F39), // red
    onError = Color(0xFFEFF1F5), // base
    errorContainer = Color(0xFFBCC0CC), // surface1 (soft error surface)
    onErrorContainer = Color(0xFFD20F39), // red
    inverseSurface = Color(0xFF4C4F69), // text
    inverseOnSurface = Color(0xFFEFF1F5), // base
    surfaceTint = Color(0xFF8839EF), // mauve
)

val CatppuccinDarkColors: ColorScheme = darkColorScheme(
    primary = Color(0xFFCBA6F7), // mauve
    onPrimary = Color(0xFF1E1E2E), // base
    primaryContainer = Color(0xFF45475A), // surface1
    onPrimaryContainer = Color(0xFFCDD6F4), // text
    secondary = Color(0xFF94E2D5), // teal
    onSecondary = Color(0xFF1E1E2E), // base
    secondaryContainer = Color(0xFF313244), // surface0
    onSecondaryContainer = Color(0xFFCDD6F4), // text
    tertiary = Color(0xFFB4BEFE), // lavender
    onTertiary = Color(0xFF1E1E2E), // base
    tertiaryContainer = Color(0xFF45475A), // surface1
    onTertiaryContainer = Color(0xFFCDD6F4), // text
    background = Color(0xFF1E1E2E), // base
    onBackground = Color(0xFFCDD6F4), // text
    surface = Color(0xFF1E1E2E), // base
    onSurface = Color(0xFFCDD6F4), // text
    surfaceVariant = Color(0xFF313244), // surface0
    onSurfaceVariant = Color(0xFF7F849C), // overlay1
    outline = Color(0xFF585B70), // surface2
    outlineVariant = Color(0xFF45475A), // surface1
    error = Color(0xFFF38BA8), // red
    onError = Color(0xFF1E1E2E), // base
    errorContainer = Color(0xFF45475A), // surface1 (soft error surface)
    onErrorContainer = Color(0xFFF38BA8), // red
    inverseSurface = Color(0xFFCDD6F4), // text
    inverseOnSurface = Color(0xFF1E1E2E), // base
    surfaceTint = Color(0xFFCBA6F7), // mauve
)