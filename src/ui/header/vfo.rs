use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::Span;

use crate::app::VFO_DIGITS;

use super::super::theme;

/// Build the VFO frequency display spans.
///
/// - Significant digits: bright white.
/// - Leading zeros: dim gray.
/// - Cursor (when focused): yellow REVERSED highlight.
/// - ` f ` keybinding badge appears only when not editing.
pub(super) fn spans(freq: u64, cursor: usize, focused: bool) -> Vec<Span<'static>> {
    let digits = format!("{:0>width$}", freq, width = VFO_DIGITS);
    let dot_style = Style::default().fg(theme::DIM);
    let bright = Style::default().fg(theme::BRIGHT).bold();
    let dim = Style::default().fg(theme::DIM).bold();
    let cursor_style = Style::default()
        .fg(theme::ACCENT)
        .add_modifier(Modifier::REVERSED | Modifier::BOLD);

    // Leftmost non-zero position. For freq=0 we still want at least the units
    // digit to render bright, so cap to digits.len()-1.
    let chars: Vec<char> = digits.chars().collect();
    let first_significant = chars
        .iter()
        .position(|&c| c != '0')
        .unwrap_or(chars.len() - 1);

    let mut spans: Vec<Span<'static>> = Vec::new();

    if !focused {
        let key_style = Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG);
        spans.push(Span::styled(" f ", key_style));
        spans.push(Span::styled(" ", Style::default()));
    }

    for (str_idx, ch) in chars.iter().enumerate() {
        let place = VFO_DIGITS - 1 - str_idx;
        let is_leading_zero = str_idx < first_significant;
        let style = if focused && place == cursor {
            cursor_style
        } else if is_leading_zero {
            dim
        } else {
            bright
        };
        spans.push(Span::styled(ch.to_string(), style));

        if place > 0 && place % 3 == 0 {
            spans.push(Span::styled(".", dot_style));
        }
    }

    spans.push(Span::styled(" Hz", dot_style));

    if focused {
        let place_labels = [
            "Hz", "10Hz", "100Hz", "1kHz", "10kHz", "100kHz",
            "1MHz", "10MHz", "100MHz", "1GHz",
        ];
        let label = place_labels.get(cursor).unwrap_or(&"");
        spans.push(Span::styled(
            format!(" ±{label}"),
            Style::default().fg(theme::ACCENT),
        ));
    }

    spans
}
