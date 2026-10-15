# MathJax compatibility mode (ExamGOAL fork)

RaTeX follows KaTeX. This fork adds an opt-in dialect for content authored
against **MathJax 3**, plus host-measured text runs. KaTeX mode is unchanged.

## Turning it on

| Surface | Option |
|---|---|
| Rust | `ParseOptions { compat: Compat::MathJax, color_v2 }` + `LayoutOptions::with_compat(Compat::MathJax)` |
| C ABI | `RatexOptions.compat = 1`, `flags |= RATEX_FLAG_COLOR_V2` |
| Android JNI | `RaTeXEngine.nativeParseAndLayoutEx(…, compat = 1, flags = 1, measurer)` |
| WASM | `renderLatexWithOptions(tex, { compat: "mathjax", colorV2: true })` |

## What changes in MathJax mode

Parser:

- plain-TeX `\matrix{…\cr…}`, `\pmatrix{…}`, `\eqalign{…}`, `\buildrel X \over Y`;
- `\limits` written after the script marker is applied to the base
  (`\int_\limits0^1` is `\int\limits_0^1`);
- unknown `array` column letters are skipped (`{left}` is one `l` column), and
  rows may run past the declared columns (reusing the last alignment);
- `\require{…}` is a no-op; `\class`, `\style`, `\cssId` keep their content.
- `colorv2` (separate flag): `\color{c}{x}` colors only `x`.

Layout (each measured against MathJax 3 SVG output):

- fraction bars extend 0.1em past the wider of numerator and denominator;
- a bare fraction is spaced as an ordinary atom, not Inner (no thin spaces);
- `array` has no outer `\arraycolsep` unless that side has a frame rule;
- `\fbox` / `\boxed`: 0.2em padding, 0.067em rule;
- `\sqrt[n]{…}`: the index tucks 0.6 of the way into the surd;
- the subscript of an accented letter (`\vec{V}_1`) tucks under its italic correction.

In both modes a font switch keeps atom classes (`\mathrm{a=b}` spaces the `=`).

Deliberately not changed: inline `\\` still breaks the line (MathJax 3 ignores
it, MathJax 4 breaks); stretchy arrows keep KaTeX's minimum widths.

## Host text runs

When layout gets a `TextMeasurer` (C: `RatexOptions.measure_text`),
text-mode content (`\text{…}`, `\textbf{…}`, …) is laid out as whole runs that
the host measures and draws with its own font, emitted as `Text` display items
(see `DISPLAYLIST_JSON_PROTOCOL.md`). This is MathJax's `mtextInheritFont`, and
it is what makes Devanagari and other complex scripts shape correctly. The host
must draw with the face and size it measured with.

## Measuring parity

`tools/mathjax_compat` diffs RaTeX against MathJax 3 on a real corpus (parse
failures and box widths/heights); see its README. On 97,476 unique formulas
from an exam-question bank, MathJax mode cut formulas MathJax renders but
RaTeX rejects from 375 to 18, and formulas whose width differs by more than 5%
from 18.9% to 1.0%.
