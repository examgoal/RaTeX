//! Lays out every formula of a corpus and writes its box metrics, for
//! `tools/mathjax_compat/compare.mjs` to diff against MathJax.
//!
//! Input: a JSON array of `[latex, occurrences]` pairs (or plain strings).
//! Output: a JSON array, one `{"ok":true,"w","h","d"}` or `{"ok":false,"err"}`
//! per input formula, in input order. Formulas are laid out inline (text style).
//! `--mathjax` parses and lays out with `Compat::MathJax` and `colorv2`.
//!
//! ```sh
//! cargo run --release -p ratex-layout --example corpus_dims -- [--mathjax] formulas.json dims.json
//! ```

use std::time::Instant;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::{Compat, ParseOptions};
use ratex_types::math_style::MathStyle;
use serde_json::{json, Value};

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let compat = if let Some(i) = args.iter().position(|a| a == "--mathjax") {
        args.remove(i);
        Compat::MathJax
    } else {
        Compat::KaTeX
    };
    if args.len() < 3 {
        eprintln!("usage: corpus_dims [--mathjax] <formulas.json> <out.json>");
        std::process::exit(2);
    }
    let parse_options = ParseOptions {
        compat,
        color_v2: compat.is_mathjax(),
    };
    let input: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(&args[1]).expect("read input"))
            .expect("input must be a JSON array");
    let options = LayoutOptions::default()
        .with_style(MathStyle::Text)
        .with_compat(compat);
    let started = Instant::now();
    let out: Vec<Value> = input
        .iter()
        .map(|entry| {
            let latex = match entry {
                Value::Array(pair) => pair.first().and_then(Value::as_str).unwrap_or_default(),
                Value::String(s) => s.as_str(),
                _ => "",
            };
            match ratex_parser::parse_with_options(latex, &parse_options) {
                Ok(nodes) => {
                    let list = to_display_list(&layout(&nodes, &options));
                    json!({ "ok": true, "w": list.width, "h": list.height, "d": list.depth })
                }
                Err(e) => json!({ "ok": false, "err": e.to_string() }),
            }
        })
        .collect();
    eprintln!(
        "{} formulas in {:.2}s",
        out.len(),
        started.elapsed().as_secs_f64()
    );
    std::fs::write(&args[2], serde_json::to_string(&out).expect("serialize"))
        .expect("write output");
}
