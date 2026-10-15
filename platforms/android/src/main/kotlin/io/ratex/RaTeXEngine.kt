// RaTeXEngine.kt — Kotlin JNI wrapper around libratex_ffi.so

package io.ratex

import android.graphics.Color
import androidx.annotation.ColorInt
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

// MARK: - Error type

class RaTeXException(message: String) : Exception(message)

// MARK: - Layout options

/** TeX dialect where KaTeX and MathJax disagree. */
enum class RaTeXCompat(internal val code: Int) {
    /** KaTeX behavior (default). */
    KaTeX(0),

    /** MathJax 3: plain-TeX `\matrix`/`\buildrel`, `\int_\limits`, MathJax fraction/array/box spacing. */
    MathJax(1),
}

/**
 * Options for [RaTeXEngine.parse].
 *
 * @property compat TeX dialect.
 * @property colorV2 MathJax `colorv2`: `\color{c}{x}` colors only `x`.
 * @property textMeasurer Measures `\text{…}` runs with the app's typeface; layout then
 *   returns them as [DisplayItem.Text]. Null keeps them as KaTeX glyphs.
 */
data class RaTeXLayoutOptions(
    val compat: RaTeXCompat = RaTeXCompat.KaTeX,
    val colorV2: Boolean = false,
    val textMeasurer: RaTeXTextMeasurer? = null,
) {
    internal val flags: Int get() = if (colorV2) FLAG_COLOR_V2 else 0

    companion object {
        private const val FLAG_COLOR_V2 = 1

        /** MathJax 3 with the `colorv2` package — for content authored against MathJax. */
        @JvmStatic
        @JvmOverloads
        fun mathJax(textMeasurer: RaTeXTextMeasurer? = null) =
            RaTeXLayoutOptions(RaTeXCompat.MathJax, colorV2 = true, textMeasurer = textMeasurer)
    }
}

// MARK: - Engine

/**
 * Entry point for RaTeX rendering on Android.
 *
 * ```kotlin
 * val displayList = RaTeXEngine.parse("""\frac{-b \pm \sqrt{b^2-4ac}}{2a}""")
 * ```
 *
 * Note: [parse] is a suspend function; call it from a coroutine.
 * For one-shot calls from non-coroutine code, use [parseBlocking].
 */
object RaTeXEngine {

    private fun rgbaFloatArray(@ColorInt color: Int): FloatArray = floatArrayOf(
        ((color ushr 16) and 0xff) / 255f,
        ((color ushr 8) and 0xff) / 255f,
        (color and 0xff) / 255f,
        ((color ushr 24) and 0xff) / 255f,
    )

    /**
     * Whether `libratex_ffi.so` loaded (false on ABIs the AAR doesn't ship, e.g. 32-bit
     * x86). [parseBlocking] throws [RaTeXException] when it is false.
     */
    @JvmStatic
    val isAvailable: Boolean = try {
        System.loadLibrary("ratex_ffi")
        true
    } catch (_: UnsatisfiedLinkError) {
        false
    }

    // -------------------------------------------------------------------------
    // JNI declarations (implemented in crates/ratex-ffi/src/jni.rs)
    // -------------------------------------------------------------------------

    /**
     * Parse and lay out a LaTeX string with explicit display mode.
     * @param displayMode true = display/block style, false = inline/text style.
     * @return JSON DisplayList string on success, or null on error.
     */
    @JvmStatic
    private external fun nativeParseAndLayout(
        latex: String,
        displayMode: Boolean,
        color: FloatArray,
    ): String?

    /**
     * [nativeParseAndLayout] with a TeX dialect (`compat`: 0 KaTeX, 1 MathJax),
     * `RATEX_FLAG_*` bits and an optional text measurer, called back on this thread.
     */
    @JvmStatic
    private external fun nativeParseAndLayoutEx(
        latex: String,
        displayMode: Boolean,
        color: FloatArray,
        compat: Int,
        flags: Int,
        measurer: RaTeXTextMeasurer?,
    ): String?

    /**
     * Retrieve the last error message produced by a native layout call on this thread.
     */
    @JvmStatic
    private external fun nativeGetLastError(): String?

    // -------------------------------------------------------------------------
    // Public API
    // -------------------------------------------------------------------------

    /**
     * Parse [latex] and return a [DisplayList] decoded from the JSON result.
     * Runs on [Dispatchers.Default].
     *
     * @param displayMode `true` (default) for display/block style; `false` for inline/text style.
     * @param options TeX dialect and text measurement, see [RaTeXLayoutOptions].
     * @throws RaTeXException on parse or decode error.
     */
    suspend fun parse(
        latex: String,
        displayMode: Boolean = true,
        @ColorInt color: Int = Color.BLACK,
        options: RaTeXLayoutOptions = RaTeXLayoutOptions(),
    ): DisplayList = withContext(Dispatchers.Default) { parseBlocking(latex, displayMode, color, options) }

    /**
     * Blocking variant of [parse]. Safe to call on any background thread.
     * **Do not call on the main thread** — use [parse] instead.
     *
     * @param displayMode `true` (default) for display/block style; `false` for inline/text style.
     * @param options TeX dialect and text measurement, see [RaTeXLayoutOptions].
     * @throws RaTeXException on parse or decode error.
     */
    @JvmOverloads
    fun parseBlocking(
        latex: String,
        displayMode: Boolean = true,
        @ColorInt color: Int = Color.BLACK,
        options: RaTeXLayoutOptions = RaTeXLayoutOptions(),
    ): DisplayList {
        if (!isAvailable) throw RaTeXException("libratex_ffi is not available on this device")
        val json = if (options == RaTeXLayoutOptions()) {
            nativeParseAndLayout(latex = latex, displayMode = displayMode, color = rgbaFloatArray(color))
        } else {
            nativeParseAndLayoutEx(
                latex = latex,
                displayMode = displayMode,
                color = rgbaFloatArray(color),
                compat = options.compat.code,
                flags = options.flags,
                measurer = options.textMeasurer,
            )
        }
            ?: throw RaTeXException(nativeGetLastError() ?: "unknown error")
        return try {
            ratexJson.decodeFromString(DisplayList.serializer(), json)
        } catch (e: Exception) {
            throw RaTeXException("JSON decode failed: ${e.message}")
        }
    }
}
