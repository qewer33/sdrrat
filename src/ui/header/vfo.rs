use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::Span;

use crate::app::VFO_DIGITS;

use super::super::theme;

/// Build the VFO frequency display spans.
///
/// Unfocused: ` f ` button + space + dim digits.
/// Focused:   bright digits + space + active tuning step (`±1MHz` etc.).
pub(super) fn spans(freq: u64, cursor: usize, focused: bool) -> Vec<Span<'static>> {
    let digits = format!("{:0>width$}", freq, width = VFO_DIGITS);
    let dot_style = Style::default().fg(theme::DIM);

    let normal = if focused {
        Style::default().fg(theme::BRIGHT).bold()
    } else {
        Style::default().fg(theme::DIM).bold()
    };
    let cursor_style = Style::default()
        .fg(theme::ACCENT)
        .add_modifier(Modifier::REVERSED | Modifier::BOLD);

    let mut spans: Vec<Span<'static>> = Vec::new();

    // ` f ` keybinding badge — only visible when not editing.
    if !focused {
        let key_style = Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG);
        spans.push(Span::styled(" f ", key_style));
        spans.push(Span::styled(" ", Style::default()));
    }

    for (str_idx, ch) in digits.chars().enumerate() {
        let place = VFO_DIGITS - 1 - str_idx;
        let style = if focused && place == cursor { cursor_style } else { normal };
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
