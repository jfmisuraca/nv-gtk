package ar.com.nvgtk

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.dp

/**
 * T6: the app-level Material 3 shape scale — the classic M3 baseline ramp
 * pinned to its documented values, plus the Expressive flagship token exposed
 * for app use ([Shapes.largeIncreased]).
 *
 * API reality (verified against the material3-android 1.4.0 AAR in the Gradle
 * cache): [Shapes] in 1.4.0 exposes only five public constructor parameters
 * (extraSmall down to extraLarge). The three Expressive corners
 * (largeIncreased, extraLargeIncreased, extraExtraLarge) — and the
 * extraLargeTop/extraLargeBottom tokens of earlier 1.x releases — are emitted
 * as public bytecode but marked internal in the Kotlin metadata: exactly the
 * same internal-mangling pattern as the emphasized typography tokens from T5,
 * so none of them is usable from app code.
 *
 * [AppShapes] therefore pins the five public baseline tokens to their
 * documented M3 values, and the [Shapes.largeIncreased] extension supplies the
 * missing Expressive flagship token (36dp per the M3 Expressive story) at the
 * same name and value, read as `AppShapes.largeIncreased` so the spec text and
 * the code stay in lockstep. Once material3 exposes the member publicly, drop
 * the extension and set the constructor parameter instead.
 *
 * The app wires [AppShapes] into the theme
 * (`MaterialTheme(shapes = AppShapes)`), so any future tuning of the ramp
 * (including the Expressive accent) lands in exactly one place and propagates
 * through the same object.
 */
val AppShapes: Shapes = Shapes(
    extraSmall = RoundedCornerShape(4.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(16.dp),
    extraLarge = RoundedCornerShape(28.dp)
)

/**
 * M3 Expressive flagship token (36dp). Bridges the token that material3 1.4.0
 * keeps internal; consumers read it as `AppShapes.largeIncreased`.
 */
val Shapes.largeIncreased: Shape
    get() = RoundedCornerShape(36.dp)