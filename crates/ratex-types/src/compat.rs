use serde::{Deserialize, Serialize};

/// Which TeX renderer to follow where KaTeX and MathJax disagree.
///
/// RaTeX follows KaTeX by default. [`Compat::MathJax`] switches the parser and
/// layout to MathJax 3 behavior for content authored against MathJax:
///
/// - parser: plain-TeX `\matrix`, `\pmatrix`, `\eqalign`, `\buildrel … \over …`;
///   `\limits` written after the script marker (`\int_\limits0^1`); unknown
///   `array` column letters ignored; `\require`, `\class`, `\style`, `\cssId`
///   accepted (their decoration is dropped);
/// - layout: fraction bars extend 0.1em past the wider of numerator and
///   denominator; `array` has no column padding at its outer edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Compat {
    #[default]
    KaTeX,
    MathJax,
}

impl Compat {
    pub fn is_mathjax(self) -> bool {
        self == Compat::MathJax
    }
}
