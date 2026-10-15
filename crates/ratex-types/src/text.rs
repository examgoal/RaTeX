use serde::{Deserialize, Serialize};

/// Font style of a run of text that the host draws (`\text`, `\textbf`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TextStyle {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub monospace: bool,
}

impl TextStyle {
    /// Bit flags used across the C ABI: 1 = bold, 2 = italic, 4 = monospace.
    pub fn flags(self) -> u32 {
        (self.bold as u32) | (self.italic as u32) << 1 | (self.monospace as u32) << 2
    }

    pub fn from_flags(flags: u32) -> Self {
        Self {
            bold: flags & 1 != 0,
            italic: flags & 2 != 0,
            monospace: flags & 4 != 0,
        }
    }
}

/// Box of a text run at a 1em font size: advance width, and extent above
/// (height) and below (depth) the baseline, all in em.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextMetrics {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
}

/// Measures text runs with the host's own font and shaping.
///
/// When a measurer is supplied to layout, text-mode content (`\text{…}` and
/// the `\text…` font commands) is laid out as whole runs measured here and
/// emitted as [`crate::DisplayItem::Text`], for the host to draw with the same
/// font — like MathJax's `mtextInheritFont`. That keeps complex scripts
/// (Devanagari, Bengali, Arabic, …) correctly shaped and lets formula text
/// match the surrounding page text.
pub trait TextMeasurer {
    fn measure(&self, text: &str, style: TextStyle) -> TextMetrics;
}
