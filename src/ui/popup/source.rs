use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, ConnectionState, SourceField};

use super::super::theme;
use super::{draw_chrome, field_row, STD_POPUP_WIDTH};

fn fmt_sample_rate(rate: u32) -> String {
    if rate >= 1_000_000 {
        format!("{:.3} MSPS", rate as f64 / 1_000_000.0)
    } else {
        format!("{} kSPS", rate / 1_000)
    }
}

/// Build the bottom action row: `▸ [Connect] ` (or `[Reconnect]` when
/// already connected). Highlighted when focused.
fn connect_row(focused: bool, connected: bool) -> Line<'static> {
    let label = if connected { "[ Reconnect ]" } else { "[ Connect ]" };
    let style = if focused {
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        Style::default().fg(theme::ACCENT).bold()
    };
    Line::from(Span::styled(label.to_string(), style))
        .alignment(Alignment::Center)
}

pub(in crate::ui) fn draw_popup(frame: &mut Frame, app: &App) {
    // Banner row appears only when the last connect attempt failed.
    let error_msg = match &app.connection {
        ConnectionState::Disconnected { last_error: Some(e) } => Some(e.clone()),
        _ => None,
    };

    // Body lines: blank + 4 field rows + blank + Connect button = 7,
    // plus 2 (error + blank) when there's an error banner.
    // Popup height = body lines + 1 (title bar) + 1 (bottom border).
    let height = if error_msg.is_some() { 11 } else { 9 };
    let inner = draw_chrome(frame, "Source", STD_POPUP_WIDTH, height);

    let mut lines: Vec<Line<'static>> = Vec::new();

    if let Some(msg) = error_msg {
        let truncated: String = msg.chars().take((STD_POPUP_WIDTH as usize) - 4).collect();
        lines.push(
            Line::from(vec![
                Span::styled(" ⚠ ", Style::default().fg(Color::Red).bold()),
                Span::styled(truncated, Style::default().fg(Color::Red)),
            ])
            .alignment(Alignment::Center),
        );
        lines.push(Line::default());
    } else {
        lines.push(Line::default());
    }

    lines.push(field_row(
        "Device",
        app.device_kind.label().into(),
        app.source_focus == SourceField::Device,
        false,
    ));
    lines.push(field_row(
        "Sample Rate",
        fmt_sample_rate(app.current_sample_rate()),
        app.source_focus == SourceField::SampleRate,
        false,
    ));
    lines.push(field_row(
        "Gain Mode",
        if app.agc_enabled { "AGC".into() } else { "Manual".into() },
        app.source_focus == SourceField::GainMode,
        false,
    ));
    let gain_value = if app.agc_enabled {
        "—".into()
    } else {
        format!("{:.1} dB", app.current_gain_db())
    };
    lines.push(field_row(
        "Manual Gain",
        gain_value,
        app.source_focus == SourceField::ManualGain,
        app.agc_enabled,
    ));
    lines.push(Line::default());
    lines.push(connect_row(
        app.source_focus == SourceField::Connect,
        app.is_connected(),
    ));

    let body = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(body, inner);
}
