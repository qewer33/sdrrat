//! Top header row: connection status (left) + VFO (centered) + dB range
//! (right-aligned), all overlaid on the same single-line `Rect`.

mod db_range;
mod vfo;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, AppMode, RadioMode};

use super::theme;

pub(super) fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let vfo_focused = app.mode == AppMode::EditingFrequency;
    let mm_focused = app.mode == AppMode::EditingMinMax;

    // Left: connection status + device name + radio mode badge.
    let mut left_spans: Vec<Span<'static>> = Vec::new();
    if app.is_connected() {
        left_spans.push(Span::styled(" ● ", Style::default().fg(Color::Green).bold()));
        left_spans.push(Span::styled(
            app.device_kind.label().to_string(),
            Style::default().fg(Color::White).bold(),
        ));
    } else {
        left_spans.push(Span::styled(" ○ ", Style::default().fg(Color::Red).bold()));
        left_spans.push(Span::styled(
            "Disconnected",
            Style::default().fg(Color::Red).bold(),
        ));
    }

    if app.radio_mode != RadioMode::Off {
        left_spans.push(Span::styled("  ", Style::default()));
        left_spans.push(Span::styled(
            format!(" {} ", app.radio_mode.label()),
            Style::default()
                .fg(theme::HEADER_FG)
                .bg(theme::ACCENT)
                .bold(),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(left_spans)), area);

    // Center: VFO.
    let vfo = vfo::spans(app.center_freq, app.vfo_cursor, vfo_focused);
    frame.render_widget(Paragraph::new(Line::from(vfo)).centered(), area);

    // Right: dB range.
    let db = db_range::spans(app.y_min, app.y_max, app.minmax_focus, mm_focused);
    frame.render_widget(Paragraph::new(Line::from(db)).right_aligned(), area);
}
