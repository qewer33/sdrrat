use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;

use super::super::theme;
use super::draw_chrome;

const HELP_WIDTH: u16 = 60;
const HELP_HEIGHT: u16 = 29;

const BANNER: &str = r"           .___                     __
  ______ __| _/__________________ _/  |_
 /  ___// __ |\_  __ \_  __ \__  \\   __\
 \___ \/ /_/ | |  | \/|  | \// __ \|  |
/____  >____ | |__|   |__|  (____  /__|
     \/     \/                   \/";

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

    // Banner art lives in its own top region; the keybinding tables below
    // are split into two columns. Trailing whitespace gets stripped by code
    // formatters, so re-pad each line to the longest line's width at runtime
    // — otherwise `.centered()` centers each line independently and shifts
    // them relative to each other.
    let max_w = BANNER.lines().map(|l| l.chars().count()).max().unwrap_or(0);
    let mut banner_lines: Vec<Line<'static>> = Vec::new();
    banner_lines.push(Line::default());
    for l in BANNER.lines() {
        let pad = max_w.saturating_sub(l.chars().count());
        let padded = format!("{l}{}", " ".repeat(pad));
        banner_lines.push(Line::from(Span::styled(
            padded,
            Style::default().fg(theme::ACCENT).bold(),
        )));
    }
    banner_lines.push(Line::default());
    banner_lines.push(Line::from(Span::styled(
        "discover the RF spectrum from your terminal",
        Style::default().fg(theme::ACCENT),
    )));
    banner_lines.push(Line::default());
    let banner_height = banner_lines.len() as u16;

    let parts = Layout::vertical([
        Constraint::Length(banner_height),
        Constraint::Min(0),
    ])
    .split(inner);

    frame.render_widget(
        Paragraph::new(banner_lines).centered(),
        parts[0],
    );

    let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(parts[1]);

    let left = vec![
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
