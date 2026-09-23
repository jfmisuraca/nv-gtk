package ar.com.nvgtk

import android.content.Context
import android.os.Build
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

/**
 * Additional ColorSchemes for the NV-GTK mobile app.
 *
 * Dracula (dark) / Alucard (Dracula light) and Flexoki dark/light, mapped onto
 * the same Material 3 roles [CatppuccinTheme.kt][CatppuccinLightColors] uses:
 * primary = mauve/purple, secondary = teal/cyan, tertiary = lavender/purple,
 * error = red, surfaces/outlines from the elevation ramp. Where a theme defines
 * no soft-red surface, errorContainer aliases that theme's pink/magenta (its
 * nearest soft-red hue) — never another theme's colors.
 *
 * Hex sources (authoritative, see `odd/tasks/theme-palettes.md`): Dracula and
 * Alucard from draculatheme.com/spec; Flexoki from the kepano/flexoki README.
 * Elevation steps with no explicit spec value are blended `base -> text` at the
 * same fixed ratios the desktop `src/palettes.rs` tables document.
 */
val DraculaDarkColors: ColorScheme = darkColorScheme(
    primary = Color(0xFFBD93F9), // purple
    onPrimary = Color(0xFF282A36), // base
    primaryContainer = Color(0xFF424450), // bgLighter (surface1)
    onPrimaryContainer = Color(0xFFF8F8F2), // fg
    secondary = Color(0xFF8BE9FD), // cyan
    onSecondary = Color(0xFF282A36), // base
    secondaryContainer = Color(0xFF343746), // bgLight (surface0)
    onSecondaryContainer = Color(0xFFF8F8F2), // fg
    tertiary = Color(0xFFBD93F9), // purple
    onTertiary = Color(0xFF282A36), // base
    tertiaryContainer = Color(0xFF424450), // bgLighter (surface1)
    onTertiaryContainer = Color(0xFFF8F8F2), // fg
    background = Color(0xFF282A36), // bg
    onBackground = Color(0xFFF8F8F2), // fg
    surface = Color(0xFF282A36), // bg
    onSurface = Color(0xFFF8F8F2), // fg
    surfaceVariant = Color(0xFF343746), // bgLight (surface0)
    onSurfaceVariant = Color(0xFF9A9B9D), // base -> fg blend at 0.55
    outline = Color(0xFF44475A), // selection (surface2)
    outlineVariant = Color(0xFF424450), // bgLighter (surface1)
    error = Color(0xFFFF5555), // red
    onError = Color(0xFF282A36), // base
    errorContainer = Color(0xFFFF79C6), // pink (soft-red alias)
    onErrorContainer = Color(0xFFFF5555), // red
    inverseSurface = Color(0xFFF8F8F2), // fg
    inverseOnSurface = Color(0xFF282A36), // base
    surfaceTint = Color(0xFFBD93F9), // purple
)

val DraculaLightColors: ColorScheme = lightColorScheme(
    primary = Color(0xFF644AC9), // purple
    onPrimary = Color(0xFFFFFBEB), // bg
    primaryContainer = Color(0xFFCECCC0), // bgDark (surface1)
    onPrimaryContainer = Color(0xFF1F1F1F), // fg
    secondary = Color(0xFF036A96), // cyan
    onSecondary = Color(0xFFFFFBEB), // bg
    secondaryContainer = Color(0xFFDEDCCF), // bgLight (surface0)
    onSecondaryContainer = Color(0xFF1F1F1F), // fg
    tertiary = Color(0xFF644AC9), // purple
    onTertiary = Color(0xFFFFFBEB), // bg
    tertiaryContainer = Color(0xFFCECCC0), // bgDark (surface1)
    onTertiaryContainer = Color(0xFF1F1F1F), // fg
    background = Color(0xFFFFFBEB), // bg
    onBackground = Color(0xFF1F1F1F), // fg
    surface = Color(0xFFFFFBEB), // bg
    onSurface = Color(0xFF1F1F1F), // fg
    surfaceVariant = Color(0xFFDEDCCF), // bgLight (surface0)
    onSurfaceVariant = Color(0xFF84827B), // base -> fg blend at 0.55
    outline = Color(0xFFBCBAB3), // bgDarker (surface2)
    outlineVariant = Color(0xFFCECCC0), // bgDark (surface1)
    error = Color(0xFFCB3A2A), // red
    onError = Color(0xFFFFFBEB), // bg
    errorContainer = Color(0xFFA3144D), // pink (soft-red alias)
    onErrorContainer = Color(0xFFCB3A2A), // red
    inverseSurface = Color(0xFF1F1F1F), // fg
    inverseOnSurface = Color(0xFFFFFBEB), // bg
    surfaceTint = Color(0xFF644AC9), // purple
)

val FlexokiDarkColors: ColorScheme = darkColorScheme(
    primary = Color(0xFF5E409D), // purple 600
    onPrimary = Color(0xFF100F0F), // black (base)
    primaryContainer = Color(0xFF403E3C), // ramp step (surface1)
    onPrimaryContainer = Color(0xFFF2F0E5), // text
    secondary = Color(0xFF24837B), // cyan 600
    onSecondary = Color(0xFF100F0F), // black (base)
    secondaryContainer = Color(0xFF343331), // ramp step (surface0)
    onSecondaryContainer = Color(0xFFF2F0E5), // text
    tertiary = Color(0xFF5E409D), // purple 600
    onTertiary = Color(0xFF100F0F), // black (base)
    tertiaryContainer = Color(0xFF403E3C), // ramp step (surface1)
    onTertiaryContainer = Color(0xFFF2F0E5), // text
    background = Color(0xFF100F0F), // black (base)
    onBackground = Color(0xFFF2F0E5), // text
    surface = Color(0xFF100F0F), // black (base)
    onSurface = Color(0xFFF2F0E5), // text
    surfaceVariant = Color(0xFF343331), // ramp step (surface0)
    onSurfaceVariant = Color(0xFF878580), // ramp step (overlay1)
    outline = Color(0xFF575653), // ramp step (surface2)
    outlineVariant = Color(0xFF403E3C), // ramp step (surface1)
    error = Color(0xFFAF3029), // red 600
    onError = Color(0xFF100F0F), // black (base)
    errorContainer = Color(0xFFA02F6F), // magenta 600 (soft-red alias)
    onErrorContainer = Color(0xFFAF3029), // red 600
    inverseSurface = Color(0xFFF2F0E5), // text
    inverseOnSurface = Color(0xFF100F0F), // black (base)
    surfaceTint = Color(0xFF5E409D), // purple 600
)

val FlexokiLightColors: ColorScheme = lightColorScheme(
    primary = Color(0xFF8B7EC8), // purple 400
    onPrimary = Color(0xFFFFFCF0), // paper (base)
    primaryContainer = Color(0xFFCECDC3), // ramp step (surface1)
    onPrimaryContainer = Color(0xFF100F0F), // text
    secondary = Color(0xFF3AA99F), // cyan 400
    onSecondary = Color(0xFFFFFCF0), // paper (base)
    secondaryContainer = Color(0xFFDAD8CE), // ramp step (surface0)
    onSecondaryContainer = Color(0xFF100F0F), // text
    tertiary = Color(0xFF8B7EC8), // purple 400
    onTertiary = Color(0xFFFFFCF0), // paper (base)
    tertiaryContainer = Color(0xFFCECDC3), // ramp step (surface1)
    onTertiaryContainer = Color(0xFF100F0F), // text
    background = Color(0xFFFFFCF0), // paper (base)
    onBackground = Color(0xFF100F0F), // text
    surface = Color(0xFFFFFCF0), // paper (base)
    onSurface = Color(0xFF100F0F), // text
    surfaceVariant = Color(0xFFDAD8CE), // ramp step (surface0)
    onSurfaceVariant = Color(0xFF878580), // ramp step (overlay1)
    outline = Color(0xFFB7B5AC), // ramp step (surface2)
    outlineVariant = Color(0xFFCECDC3), // ramp step (surface1)
    error = Color(0xFFD14D41), // red 400
    onError = Color(0xFFFFFCF0), // paper (base)
    errorContainer = Color(0xFFCE5D97), // magenta 400 (soft-red alias)
    onErrorContainer = Color(0xFFD14D41), // red 400
    inverseSurface = Color(0xFF100F0F), // text
    inverseOnSurface = Color(0xFFFFFCF0), // paper (base)
    surfaceTint = Color(0xFF8B7EC8), // purple 400
)

/**
 * The user-selectable palettes. Ids match the desktop
 * `ThemeId::as_str` values so both platforms persist the same string.
 */
enum class NvTheme(val id: String) {
    Catppuccin("catppuccin"),
    Dracula("dracula"),
    Flexoki("flexoki"),
    Wallpaper("wallpaper");

    companion object {
        fun fromPersisted(id: String): NvTheme =
            entries.firstOrNull { it.id == id } ?: Catppuccin
    }
}

/**
 * Resolves the [ColorScheme] for a theme and variant. `Wallpaper` returns the
 * Material You dynamic scheme on API >= 31 and falls back to Catppuccin below
  * it, preserving today's effective behavior there.
 */
@Composable
fun colorSchemeFor(theme: NvTheme, dark: Boolean, context: Context = LocalContext.current): ColorScheme =
    when (theme) {
        NvTheme.Catppuccin -> if (dark) CatppuccinDarkColors else CatppuccinLightColors
        NvTheme.Dracula -> if (dark) DraculaDarkColors else DraculaLightColors
        NvTheme.Flexoki -> if (dark) FlexokiDarkColors else FlexokiLightColors
        NvTheme.Wallpaper ->
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
            } else if (dark) {
                CatppuccinDarkColors
            } else {
                CatppuccinLightColors
            }
    }

/**
 * Non-Compose resolver for the boot window background (T10).
 *
 * Returns the palette `background` role for a theme and variant, read from
 * the same [ColorScheme] tables [colorSchemeFor] serves, so the `onCreate`
 * window background and the first Compose frame always agree. `Wallpaper`
 * resolves to the Catppuccin fallback: the Material You dynamic roles are
 * `@Composable` and cannot run before `setContent`, and below API 31
 * Catppuccin is the effective scheme anyway — matching the static day/night
 * XML (`values/themes.xml`, `values-night/themes.xml`).
 */
fun bootBackground(theme: NvTheme, dark: Boolean): Color =
    when (theme) {
        NvTheme.Catppuccin -> if (dark) CatppuccinDarkColors.background else CatppuccinLightColors.background
        NvTheme.Dracula -> if (dark) DraculaDarkColors.background else DraculaLightColors.background
        NvTheme.Flexoki -> if (dark) FlexokiDarkColors.background else FlexokiLightColors.background
        NvTheme.Wallpaper -> if (dark) CatppuccinDarkColors.background else CatppuccinLightColors.background
    }
