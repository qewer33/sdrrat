use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

use super::super::theme;
use super::draw_chrome;

const HELP_WIDTH: u16 = 60;
const HELP_HEIGHT: u16 = 22;

fn key(s: &str) -> Span<'static> {
    Span::styled(
        format!(" {s} "),
        Style::default()
            .fg(theme::HEADER_FG)
            .bg(theme::HEADER_BG)
            .bold(),
    )
}

fn desc(s: &str) -> Span<'static> {
    Span::styled(format!(" {s}"), Style::default().fg(theme::BRIGHT))
}

fn section(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default().fg(theme::ACCENT).bold(),
    ))
}

fn row(k: &str, d: &str) -> Line<'static> {
    Line::from(vec![key(k), desc(d)])
}

pub(in crate::ui) fn draw_popup(frame: &mut Frame, _app: &App) {
    let inner = draw_chrome(frame, "Help", HELP_WIDTH, HELP_HEIGHT);

    let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let left = vec![
        Line::default(),
        section(" Global"),
        row("q", "Quit"),
        row("Space", "Start / Stop stream"),
        row("←/→", "Freq ±0.1 MHz"),
        row("o", "Toggle overlay"),
        row("d", "Mute / unmute"),
        row("f", "Focus VFO"),
        row("m", "Focus Min/Max"),
        row("s", "Source popup"),
        row("r", "Radio popup"),
        row("h", "This help"),
    ];

    let right = vec![
        Line::default(),
        section(" VFO mode"),
        row("←/→", "Move cursor"),
        row("↑/↓", "Inc / dec digit"),
        row("z", "Zero right of cursor"),
        row("Enter", "Commit"),
        row("Esc", "Cancel"),
        Line::default(),
        section(" Min/Max"),
        row("Tab", "Switch Min/Max"),
        row("↑/↓", "±5 dB"),
        Line::default(),
        section(" Popups"),
        row("Tab/↑↓", "Move between fields"),
        row("←/→", "Cycle value"),
        row("Enter", "Confirm / Connect"),
        row("Esc", "Close"),
    ];

    frame.render_widget(Paragraph::new(left), cols[0]);
    frame.render_widget(Paragraph::new(right), cols[1]);
}
