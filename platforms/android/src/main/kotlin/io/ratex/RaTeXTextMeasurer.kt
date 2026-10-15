// RaTeXTextMeasurer.kt — host measurement of `\text{…}` runs.

package io.ratex

import android.graphics.Paint
import android.graphics.Rect
import android.graphics.Typeface
import android.text.TextPaint

/** Bit flags of a text run's style, shared with the native layout. */
object RaTeXTextStyle {
    const val BOLD = 1
    const val ITALIC = 2
    const val MONOSPACE = 4
}

/**
 * Measures text runs with the app's own typeface. When passed to layout (see
 * [RaTeXLayoutOptions.textMeasurer]), `\text{…}` content is laid out as whole
 * runs measured here and returned as [DisplayItem.Text] for the renderer to draw
 * with the same typeface — so scripts like Devanagari are shaped by the platform.
 *
 * Called back from native code on the thread running the layout.
 */
interface RaTeXTextMeasurer {
    /**
     * Measure [text] at a 1em font size with [styleFlags] ([RaTeXTextStyle]); write
     * its advance width, height above and depth below the baseline, in em, to
     * `out[0]`, `out[1]`, `out[2]`.
     */
    fun measure(text: String, styleFlags: Int, out: FloatArray)
}

/**
 * [RaTeXTextMeasurer] backed by a [TextPaint]. Pass the same [typeface] function
 * to [RaTeXRenderer] so runs draw exactly as measured. Not thread-safe: create
 * one per layout call.
 */
class PaintTextMeasurer(
    private val typeface: (styleFlags: Int) -> Typeface,
) : RaTeXTextMeasurer {

    private val paint = TextPaint(Paint.ANTI_ALIAS_FLAG).apply { textSize = REFERENCE_PX }
    private val bounds = Rect()

    override fun measure(text: String, styleFlags: Int, out: FloatArray) {
        paint.typeface = typeface(styleFlags)
        out[0] = paint.measureText(text) / REFERENCE_PX
        paint.getTextBounds(text, 0, text.length, bounds)
        out[1] = maxOf(0, -bounds.top) / REFERENCE_PX
        out[2] = maxOf(0, bounds.bottom) / REFERENCE_PX
    }

    companion object {
        // measured large so rounding doesn't skew em ratios
        private const val REFERENCE_PX = 100f

        /** Typeface for [styleFlags] derived from [base] (monospace uses [Typeface.MONOSPACE]). */
        @JvmStatic
        fun styled(base: Typeface, styleFlags: Int): Typeface {
            val style = styleFlags and (RaTeXTextStyle.BOLD or RaTeXTextStyle.ITALIC)
            val family = if (styleFlags and RaTeXTextStyle.MONOSPACE != 0) Typeface.MONOSPACE else base
            return Typeface.create(family, style)
        }
    }
}
