mod header;
mod popup;
mod spectrum;
mod status_bar;
mod theme;
mod util;
mod waterfall;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, AppMode};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),       // Header (VFO + dB range)
        Constraint::Length(1),       // Spectrum header
        Constraint::Percentage(40),  // Spectrum chart
        Constraint::Length(1),       // Waterfall header
        Constraint::Percentage(60),  // Waterfall
        Constraint::Length(2),       // Status bar
    ])
    .split(frame.area());

    header::draw(frame, chunks[0], app);

    if app.is_connected() {
        spectrum::draw(frame, chunks[1], chunks[2], app);
        waterfall::draw(frame, chunks[3], chunks[4], app);
    } else {
        draw_section_header(frame, chunks[1], "Spectrum");
        draw_disconnected(frame, chunks[2]);
        draw_section_header(frame, chunks[3], "Waterfall");
        draw_disconnected(frame, chunks[4]);
    }

    status_bar::draw(frame, chunks[5], app);

    match app.mode {
        AppMode::EditingSource => popup::draw_source_popup(frame, app),
        AppMode::EditingRadio => popup::draw_radio_popup(frame, app),
        AppMode::Help => popup::draw_help_popup(frame, app),
        _ => {}
    }
}

fn draw_section_header(frame: &mut Frame, area: Rect, title: &str) {
    let style = Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG).bold();
    frame.render_widget(
        Paragraph::new(format!(" {title} ")).centered().style(style),
        area,
    );
}

fn draw_disconnected(frame: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let bright = Style::default().fg(theme::ACCENT).bold();
    let lines = vec![
        Line::default(),
        Line::from(Span::styled("No device connected", bright)),
        Line::default(),
        Line::from(vec![
            Span::styled("Press ", dim),
            Span::styled(" s ", Style::default().fg(Color::White).bg(theme::ACCENT).bold()),
            Span::styled(" to configure and connect.", dim),
        ]),
    ];
    let para = Paragraph::new(lines).centered();
    let inner = util::centered_rect_abs(40, 4, area);
    frame.render_widget(para, inner);
}
