//! `Compat::MathJax` behavior. Expected widths are MathJax 3 SVG output
//! (tools/mathjax_compat), in em, for inline (text-style) math.

use std::cell::RefCell;
use std::rc::Rc;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::{parse_with_options, Compat, ParseOptions};
use ratex_types::{DisplayItem, MathStyle, TextMeasurer, TextMetrics, TextStyle};

fn options(compat: Compat) -> (ParseOptions, LayoutOptions) {
    (
        ParseOptions {
            compat,
            color_v2: compat.is_mathjax(),
        },
        LayoutOptions::default()
            .with_style(MathStyle::Text)
            .with_compat(compat),
    )
}

fn width(latex: &str, compat: Compat) -> f64 {
    let (parse_options, layout_options) = options(compat);
    let nodes =
        parse_with_options(latex, &parse_options).unwrap_or_else(|e| panic!("{latex}: {e}"));
    to_display_list(&layout(&nodes, &layout_options)).width
}

fn mj(latex: &str) -> f64 {
    width(latex, Compat::MathJax)
}

fn assert_close(actual: f64, expected: f64, latex: &str) {
    assert!(
        (actual - expected).abs() < 0.01,
        "{latex}: width {actual:.4}, expected {expected:.4}"
    );
}

#[test]
fn mathjax_only_syntax_parses() {
    for latex in [
        r"\int_\limits0^a x",
        r"\int_\limits\limits0^a x",
        r"\buildrel {top} \over \longrightarrow",
        r"\matrix{1 & 2 \cr 3 & 4}",
        r"\pmatrix{1 & 2 \cr 3 & 4}",
        r"\eqalign{a &= b \cr c &= d}",
        r"\begin{array}{left}a & b\end{array}",
        r"\begin{array}{r}a & b\end{array}",
        r"\require{cancel}\cancel{x}",
        r"\class{x}{y}+\style{color:red}{z}",
    ] {
        let (parse_options, _) = options(Compat::MathJax);
        assert!(
            parse_with_options(latex, &parse_options).is_ok(),
            "MathJax mode should parse {latex}"
        );
    }
    // KaTeX mode keeps rejecting them
    let (katex, _) = options(Compat::KaTeX);
    assert!(parse_with_options(r"\int_\limits0^a x", &katex).is_err());
}

#[test]
fn limits_after_script_marker_match_plain_limits() {
    assert_close(
        mj(r"\int_\limits0^a x"),
        mj(r"\int\limits_0^a x"),
        r"\int_\limits0^a x",
    );
}

#[test]
fn fractions_follow_mathjax() {
    assert_close(mj(r"\frac{1}{2}"), 0.794, r"\frac{1}{2}");
    assert_close(mj(r"\dfrac{1}{2}"), 0.940, r"\dfrac{1}{2}");
    // no thin space around a bare fraction
    assert_close(mj(r"-\frac{1}{2}"), 1.572, r"-\frac{1}{2}");
    assert_close(mj(r"\frac{1}{2}."), 1.072, r"\frac{1}{2}.");
}

#[test]
fn boxes_roots_arrays_and_accents_follow_mathjax() {
    assert_close(mj(r"\boxed{x}"), 1.106, r"\boxed{x}");
    assert_close(mj(r"\sqrt[3]{9}"), mj(r"\sqrt{9}"), r"\sqrt[3]{9}");
    assert_close(mj(r"\begin{array}{cc}1&2\end{array}"), 2.0, "array cc");
    assert_close(
        mj(r"\begin{array}{|c|c|}1&2\end{array}"),
        3.0,
        "array |c|c|",
    );
    assert_close(mj(r"\vec{V}_1"), mj(r"V_1"), r"\vec{V}_1");
    // KaTeX mode keeps LaTeX's outer \arraycolsep
    assert_close(
        width(r"\begin{array}{cc}1&2\end{array}", Compat::KaTeX),
        3.0,
        "katex array",
    );
}

#[test]
fn font_switch_keeps_relation_spacing() {
    for compat in [Compat::KaTeX, Compat::MathJax] {
        assert_close(
            width(r"\mathrm{4:1}", compat),
            width("4:1", compat),
            r"\mathrm{4:1}",
        );
        assert_close(width(r"\mathrm{a=b}", compat), 2.39, r"\mathrm{a=b}");
    }
}

#[test]
fn color_v2_colors_only_the_argument() {
    let (parse_options, layout_options) = options(Compat::MathJax);
    let nodes = parse_with_options(r"\color{red}{x}+y", &parse_options).unwrap();
    let list = to_display_list(&layout(&nodes, &layout_options));
    let red = list
        .items
        .iter()
        .filter(|item| matches!(item, DisplayItem::GlyphPath { color, .. } if color.r == 1.0 && color.g == 0.0))
        .count();
    assert_eq!(red, 1);
}

/// Records each run and measures 0.5em per character.
struct Recorder(RefCell<Vec<(String, TextStyle)>>);

impl TextMeasurer for Recorder {
    fn measure(&self, text: &str, style: TextStyle) -> TextMetrics {
        self.0.borrow_mut().push((text.to_string(), style));
        TextMetrics {
            width: 0.5 * text.chars().count() as f64,
            height: 0.7,
            depth: 0.2,
        }
    }
}

#[test]
fn host_text_runs() {
    let recorder = Rc::new(Recorder(RefCell::new(Vec::new())));
    let (parse_options, layout_options) = options(Compat::MathJax);
    let layout_options = layout_options.with_text_measurer(recorder.clone());
    let nodes = parse_with_options(
        r"x^{\text{max}} = \text{कुल \textit{ऊर्जा} $E$ J}",
        &parse_options,
    )
    .unwrap();
    let list = to_display_list(&layout(&nodes, &layout_options));
    let runs: Vec<(String, TextStyle)> = recorder.0.borrow().clone();
    let italic = TextStyle {
        italic: true,
        ..TextStyle::default()
    };
    assert_eq!(
        runs,
        vec![
            ("max".to_string(), TextStyle::default()),
            ("कुल ".to_string(), TextStyle::default()),
            ("ऊर्जा".to_string(), italic),
            (" ".to_string(), TextStyle::default()),
            (" J".to_string(), TextStyle::default()),
        ]
    );
    let texts: Vec<_> = list
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::Text { text, scale, .. } => Some((text.as_str(), *scale)),
            _ => None,
        })
        .collect();
    assert_eq!(texts.len(), 5);
    // the superscript run is drawn at script size
    assert!(texts[0].1 < 0.8, "{texts:?}");
    assert!((texts[1].1 - 1.0).abs() < 1e-9, "{texts:?}");
}
