use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, RadioField};

use super::super::theme;
use super::{draw_chrome, field_row};

const RADIO_POPUP_WIDTH: u16 = 44;
const SLIDER_WIDTH: u16 = 14;

fn slider_row(label: &str, fraction: f32, db: f32, focused: bool) -> Line<'static> {
    let arrow = Style::default().fg(theme::ACCENT).bold();
    let dim = Style::default().fg(theme::DIM);
    let bar_style = if focused {
        Style::default().fg(theme::ACCENT).bold()
    } else {
        Style::default().fg(theme::VAL)
    };
    let label_style = Style::default().fg(theme::BRIGHT);

    let marker = if focused { "▸ " } else { "  " };
    let label_text = format!("{:<13}", label);

    let knob_idx = (fraction * (SLIDER_WIDTH as f32 - 1.0)).round() as usize;
    let mut track = String::with_capacity(SLIDER_WIDTH as usize);
    for i in 0..SLIDER_WIDTH as usize {
        track.push(if i == knob_idx { '●' } else { '─' });
    }

    let left = if focused {
        Span::styled(" ◂ ", arrow)
    } else {
        Span::styled("   ", dim)
    };
    let right = if focused {
        Span::styled(" ▸ ", arrow)
    } else {
        Span::styled("   ", dim)
    };

    Line::from(vec![
        Span::styled(marker.to_string(), if focused { arrow } else { dim }),
        Span::styled(label_text, label_style),
        left,
        Span::styled(track, bar_style),
        Span::styled(format!(" {db:>+5.0} dB"), label_style),
        right,
    ])
}

pub(in crate::ui) fn draw_popup(frame: &mut Frame, app: &App) {
    let inner = draw_chrome(frame, "Radio", RADIO_POPUP_WIDTH, 6);

    let mode_value = if app.radio_mode.is_supported() {
        app.radio_mode.label().to_string()
    } else {
        format!("{} (TODO)", app.radio_mode.label())
    };
    let mode_row = field_row(
        "Mode",
        mode_value,
        app.radio_focus == RadioField::Mode,
        false,
    );

    let squelch_row = slider_row(
        "Squelch",
        app.squelch_fraction(),
        app.squelch_db,
        app.radio_focus == RadioField::Squelch,
    );

    let body = Paragraph::new(vec![Line::default(), mode_row, squelch_row])
        .alignment(Alignment::Left);
    frame.render_widget(body, inner);
}
