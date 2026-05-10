mod header;
mod popup;
mod spectrum;
mod status_bar;
mod theme;
mod util;
mod waterfall;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

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
    spectrum::draw(frame, chunks[1], chunks[2], app);
    waterfall::draw(frame, chunks[3], chunks[4], app);
    status_bar::draw(frame, chunks[5], app);

    match app.mode {
        AppMode::EditingSource => popup::draw_source_popup(frame, app),
        AppMode::EditingRadio => popup::draw_radio_popup(frame, app),
        AppMode::Help => popup::draw_help_popup(frame, app),
        _ => {}
    }
}
