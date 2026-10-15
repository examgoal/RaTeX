// RaTeXEngine.swift — Swift wrapper around the ratex_parse_and_layout C ABI.

import Foundation
import RaTeXFFI
#if os(macOS)
import AppKit
#else
import UIKit
#endif

#if os(macOS)

private func ffiColor(from color: NSColor, appearance: NSAppearance? = nil) -> RatexColor {
    func resolve(_ c: NSColor) -> RatexColor {
        guard let rgb = c.usingColorSpace(.sRGB) else {
            return RatexColor(r: 0, g: 0, b: 0, a: 1)
        }
        var r: CGFloat = 0
        var g: CGFloat = 0
        var b: CGFloat = 0
        var a: CGFloat = 0
        rgb.getRed(&r, green: &g, blue: &b, alpha: &a)
        return RatexColor(r: Float(r), g: Float(g), b: Float(b), a: Float(a))
    }

    if let appearance {
        var result = RatexColor(r: 0, g: 0, b: 0, a: 1)
        appearance.performAsCurrentDrawingAppearance {
            result = resolve(color)
        }
        return result
    }

    return resolve(color)
}

#else

private func ffiColor(from color: UIColor, traitCollection: UITraitCollection? = nil) -> RatexColor {
    let resolved = traitCollection.map { color.resolvedColor(with: $0) } ?? color
    var r: CGFloat = 0
    var g: CGFloat = 0
    var b: CGFloat = 0
    var a: CGFloat = 0
    if resolved.getRed(&r, green: &g, blue: &b, alpha: &a) {
        return RatexColor(r: Float(r), g: Float(g), b: Float(b), a: Float(a))
    }

    let fallbackSpace = CGColorSpace(name: CGColorSpace.sRGB) ?? CGColorSpaceCreateDeviceRGB()
    guard
        let converted = resolved.cgColor.converted(
            to: fallbackSpace,
            intent: .defaultIntent,
            options: nil
        ),
        let components = converted.components,
        components.count >= 4
    else {
        return RatexColor(r: 0, g: 0, b: 0, a: 1)
    }

    return RatexColor(
        r: Float(components[0]),
        g: Float(components[1]),
        b: Float(components[2]),
        a: Float(components[3])
    )
}

#endif

// MARK: - Error type

public enum RaTeXError: Error, LocalizedError {
    case parseError(String)
    case nullResult

    public var errorDescription: String? {
        switch self {
        case .parseError(let msg): return "RaTeX parse error: \(msg)"
        case .nullResult:          return "RaTeX returned null with no error message"
        }
    }
}

// MARK: - Layout options

/// Style of a `\text{…}` run (bit flags shared with the C ABI).
public struct RaTeXTextStyle: OptionSet, Hashable, Sendable {
    public let rawValue: UInt32
    public init(rawValue: UInt32) { self.rawValue = rawValue }

    public static let bold = RaTeXTextStyle(rawValue: 1)
    public static let italic = RaTeXTextStyle(rawValue: 2)
    public static let monospace = RaTeXTextStyle(rawValue: 4)
}

/// Box of a text run at a 1em font size, in em.
public struct RaTeXTextMetrics: Sendable {
    public var width: Double
    public var height: Double
    public var depth: Double

    public init(width: Double, height: Double, depth: Double) {
        self.width = width
        self.height = height
        self.depth = depth
    }
}

/// Options for `RaTeXEngine.parse`.
public struct RaTeXLayoutOptions {
    /// TeX dialect where KaTeX and MathJax disagree.
    public enum Compat: Int32, Sendable {
        case katex = 0
        /// MathJax 3: plain-TeX `\matrix`/`\buildrel`, `\int_\limits`, MathJax fraction/array/box spacing.
        case mathJax = 1
    }

    public var compat: Compat
    /// MathJax `colorv2`: `\color{c}{x}` colors only `x`.
    public var colorV2: Bool
    /// Measures `\text{…}` runs with the app's font (see `RaTeXRenderer.measure`); layout then
    /// returns them as `DisplayItem.text`. Nil keeps them as KaTeX glyphs.
    public var measureText: ((String, RaTeXTextStyle) -> RaTeXTextMetrics)?

    public init(
        compat: Compat = .katex,
        colorV2: Bool = false,
        measureText: ((String, RaTeXTextStyle) -> RaTeXTextMetrics)? = nil
    ) {
        self.compat = compat
        self.colorV2 = colorV2
        self.measureText = measureText
    }

    /// MathJax 3 with the `colorv2` package — for content authored against MathJax.
    public static func mathJax(
        measureText: ((String, RaTeXTextStyle) -> RaTeXTextMetrics)? = nil
    ) -> RaTeXLayoutOptions {
        RaTeXLayoutOptions(compat: .mathJax, colorV2: true, measureText: measureText)
    }
}

/// Holds the Swift measuring closure for the C callback's `ctx`.
private final class MeasureBox {
    let measure: (String, RaTeXTextStyle) -> RaTeXTextMetrics
    init(_ measure: @escaping (String, RaTeXTextStyle) -> RaTeXTextMetrics) { self.measure = measure }
}

private let measureTrampoline: RatexMeasureTextFn = { ctx, text, styleFlags, out in
    guard let ctx, let text, let out else { return 1 }
    let box = Unmanaged<MeasureBox>.fromOpaque(ctx).takeUnretainedValue()
    let metrics = box.measure(String(cString: text), RaTeXTextStyle(rawValue: styleFlags))
    out[0] = metrics.width
    out[1] = metrics.height
    out[2] = metrics.depth
    return 0
}

// MARK: - Engine

/// Thread-safe entry point for RaTeX rendering.
///
/// ```swift
/// let displayList = try RaTeXEngine.shared.parse(#"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#)
/// ```
public final class RaTeXEngine {
    public static let shared = RaTeXEngine()
    private init() {}

    #if os(macOS)
    /// Parse a LaTeX string and return the corresponding `DisplayList`.
    ///
    /// This call is synchronous and CPU-bound; run it on a background queue for
    /// complex formulas.
    ///
    /// - Parameters:
    ///   - latex: A LaTeX math-mode string, e.g. `\frac{1}{2}`.
    ///   - displayMode: `true` (default) for display/block style (`$$...$$`);
    ///     `false` for inline/text style (`$...$`).
    ///   - color: Default formula color. Explicit LaTeX colors still take precedence.
    ///   - options: TeX dialect and text measurement, see `RaTeXLayoutOptions`.
    /// - Returns: A `DisplayList` ready to be drawn.
    /// - Throws: `RaTeXError.parseError` on invalid LaTeX syntax.
    public func parse(
        _ latex: String,
        displayMode: Bool = true,
        color: NSColor = .black,
        options: RaTeXLayoutOptions = RaTeXLayoutOptions()
    ) throws -> DisplayList {
        try parse(
            latex,
            displayMode: displayMode,
            color: color,
            appearance: nil,
            options: options
        )
    }

    func parse(
        _ latex: String,
        displayMode: Bool,
        color: NSColor,
        appearance: NSAppearance?,
        options: RaTeXLayoutOptions = RaTeXLayoutOptions()
    ) throws -> DisplayList {
        let ffi = ffiColor(from: color, appearance: appearance)
        return try parseWithFFIColor(latex, displayMode: displayMode, ffiColor: ffi, options: options)
    }
    #else
    /// Parse a LaTeX string and return the corresponding `DisplayList`.
    ///
    /// This call is synchronous and CPU-bound; run it on a background queue for
    /// complex formulas.
    ///
    /// - Parameters:
    ///   - latex: A LaTeX math-mode string, e.g. `\frac{1}{2}`.
    ///   - displayMode: `true` (default) for display/block style (`$$...$$`);
    ///     `false` for inline/text style (`$...$`).
    ///   - color: Default formula color. Explicit LaTeX colors still take precedence.
    ///   - options: TeX dialect and text measurement, see `RaTeXLayoutOptions`.
    /// - Returns: A `DisplayList` ready to be drawn.
    /// - Throws: `RaTeXError.parseError` on invalid LaTeX syntax.
    public func parse(
        _ latex: String,
        displayMode: Bool = true,
        color: UIColor = .black,
        options: RaTeXLayoutOptions = RaTeXLayoutOptions()
    ) throws -> DisplayList {
        try parse(
            latex,
            displayMode: displayMode,
            color: color,
            traitCollection: nil,
            options: options
        )
    }

    func parse(
        _ latex: String,
        displayMode: Bool,
        color: UIColor,
        traitCollection: UITraitCollection?,
        options: RaTeXLayoutOptions = RaTeXLayoutOptions()
    ) throws -> DisplayList {
        let ffi = ffiColor(from: color, traitCollection: traitCollection)
        return try parseWithFFIColor(latex, displayMode: displayMode, ffiColor: ffi, options: options)
    }
    #endif

    private func parseWithFFIColor(
        _ latex: String,
        displayMode: Bool,
        ffiColor color: RatexColor,
        options: RaTeXLayoutOptions
    ) throws -> DisplayList {
        var ffiDefaultColor = color
        let measurer = options.measureText.map(MeasureBox.init)
        let result = withUnsafePointer(to: &ffiDefaultColor) { colorPtr in
            var opts = RatexOptions()
            opts.struct_size = MemoryLayout<RatexOptions>.size
            opts.display_mode = displayMode ? 1 : 0
            opts.color = colorPtr
            opts.compat = options.compat.rawValue
            opts.flags = options.colorV2 ? RATEX_FLAG_COLOR_V2 : 0
            if let measurer {
                opts.measure_text = measureTrampoline
                opts.measure_text_ctx = Unmanaged.passUnretained(measurer).toOpaque()
            }
            // the C callback borrows `measurer`; keep it alive for the call
            return withExtendedLifetime(measurer) { ratex_parse_and_layout(latex, &opts) }
        }
        guard result.error_code == 0, let ptr = result.data else {
            let msg: String
            if let errPtr = ratex_get_last_error() {
                msg = String(cString: errPtr)
            } else {
                msg = "unknown error"
            }
            throw RaTeXError.parseError(msg)
        }
        defer { ratex_free_display_list(ptr) }

        let json = String(cString: ptr)
        do {
            return try JSONDecoder().decode(DisplayList.self, from: Data(json.utf8))
        } catch {
            throw RaTeXError.parseError("JSON decode failed: \(error)")
        }
    }
}
