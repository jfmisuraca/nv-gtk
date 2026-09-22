package ar.com.nvgtk

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight

/**
 * T5: the app-level Material 3 type scale — all 15 baseline styles
 * (displayLarge down to labelSmall) plus their 15 emphasized variants.
 *
 * API reality (verified against the material3-android 1.4.0 AAR in the Gradle
 * cache): the emphasized typography surface is Kotlin-internal in 1.4.0 — the
 * emphasized tokens object, the 30-argument [Typography] constructor and the
 * emphasized read accessors are all emitted as public bytecode but marked
 * internal in their Kotlin metadata, and the public `Typography.fromToken`
 * reader takes an internal key type. None of it is usable from app code, so
 * T5's fallback applies:
 *
 * - The 15 baseline styles are the Material 3 defaults ([Typography] with no
 *   arguments: Roboto / system default typeface, standard M3 sizes, line
 *   heights and weights) — no custom fonts, nothing downloaded.
 * - The 15 emphasized variants live here as the same style with an elevated
 *   weight per the M3 Expressive spec: Bold (700) for weight-400 styles,
 *   ExtraBold (800) for weight-500 styles. Use
 *   [TextStyle.appEmphasized] (or the direct values below) to apply them.
 *
 * The app wires [AppTypography] into the theme
 * (`MaterialTheme(typography = AppTypography)`), so any future customization
 * of the scale (variable font, tuned sizes) lands in exactly one place and
 * propagates through the same object.
 */
val AppTypography: Typography = Typography()

/** Emphasized variant of [this] style — size/line-height/tracking untouched. */
fun TextStyle.appEmphasized(): TextStyle = copy(
    fontWeight = if ((fontWeight ?: FontWeight.Normal).weight >= FontWeight.Medium.weight) {
        FontWeight.ExtraBold
    } else {
        FontWeight.Bold
    }
)

// Emphasized variants of the full scale (weight-400 baselines -> Bold 700,
// weight-500 baselines -> ExtraBold 800), mirroring the M3 Expressive spec.
val DisplayLargeEmphasized: TextStyle = AppTypography.displayLarge.appEmphasized()
val DisplayMediumEmphasized: TextStyle = AppTypography.displayMedium.appEmphasized()
val DisplaySmallEmphasized: TextStyle = AppTypography.displaySmall.appEmphasized()
val HeadlineLargeEmphasized: TextStyle = AppTypography.headlineLarge.appEmphasized()
val HeadlineMediumEmphasized: TextStyle = AppTypography.headlineMedium.appEmphasized()
val HeadlineSmallEmphasized: TextStyle = AppTypography.headlineSmall.appEmphasized()
val TitleLargeEmphasized: TextStyle = AppTypography.titleLarge.appEmphasized()
val TitleMediumEmphasized: TextStyle = AppTypography.titleMedium.appEmphasized()
val TitleSmallEmphasized: TextStyle = AppTypography.titleSmall.appEmphasized()
val BodyLargeEmphasized: TextStyle = AppTypography.bodyLarge.appEmphasized()
val BodyMediumEmphasized: TextStyle = AppTypography.bodyMedium.appEmphasized()
val BodySmallEmphasized: TextStyle = AppTypography.bodySmall.appEmphasized()
val LabelLargeEmphasized: TextStyle = AppTypography.labelLarge.appEmphasized()
val LabelMediumEmphasized: TextStyle = AppTypography.labelMedium.appEmphasized()
val LabelSmallEmphasized: TextStyle = AppTypography.labelSmall.appEmphasized()