use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::Span;

use crate::app::MinMaxFocus;

use super::super::theme;

/// Build the dB range display spans.
pub(super) fn spans(y_min: f64, y_max: f64, focus: MinMaxFocus, focused: bool) -> Vec<Span<'static>> {
    let highlight = Style::default()
        .fg(theme::ACCENT)
        .add_modifier(Modifier::REVERSED | Modifier::BOLD);
    let normal_bright = Style::default().fg(theme::BRIGHT).bold();
    let dimmed = Style::default().fg(theme::DIM).bold();
    let sep_style = Style::default().fg(theme::DIM);

    let (min_style, max_style) = if focused {
        match focus {
            MinMaxFocus::Min => (highlight, normal_bright),
            MinMaxFocus::Max => (normal_bright, highlight),
        }
    } else {
        (dimmed, dimmed)
    };

    let label_style = if focused {
        Style::default().fg(theme::ACCENT).bold()
    } else {
        Style::default().fg(theme::DIM)
    };

    let key_style = Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG);

    let mut spans: Vec<Span<'static>> = Vec::new();
    if !focused {
        spans.push(Span::styled(" m ", key_style));
        spans.push(Span::styled(" ", Style::default()));
    }
    spans.push(Span::styled(format!("{:.0} dB", y_min), min_style));
    spans.push(Span::styled(" ▸ ", sep_style));
    spans.push(Span::styled(format!("{:.0} dB", y_max), max_style));
    if focused {
        spans.push(Span::styled(" Tab switch ↑↓ ", label_style));
    } else {
        spans.push(Span::styled(" ", Style::default()));
    }
    spans
}
