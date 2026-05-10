//! Popups: shared window chrome plus the individual popup screens.
//!
//! `chrome` here = full-width title bar + bordered body, used by every popup.

mod help;
mod radio;
mod source;

pub(super) use help::draw_popup as draw_help_popup;
pub(super) use radio::draw_popup as draw_radio_popup;
pub(super) use source::draw_popup as draw_source_popup;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::theme;
use super::util::centered_rect_abs;

/// Default width for popups using only standard field rows.
/// Marker(2) + label(13) + arrows(3+3) + value(14) + borders(2) = 37.
pub(super) const STD_POPUP_WIDTH: u16 = 37;

/// Render the popup chrome (clear, title bar, body block) and return the
/// `Rect` representing the inner body area where field rows should be drawn.
pub(super) fn draw_chrome(
    frame: &mut Frame,
    title: &str,
    width: u16,
    height: u16,
) -> Rect {
    let area = centered_rect_abs(width, height, frame.area());
    frame.render_widget(Clear, area);

    let parts = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);

    let title_style = Style::default().fg(theme::HEADER_FG).bg(theme::ACCENT).bold();
    frame.render_widget(
        Paragraph::new(format!(" {title} ")).centered().style(title_style),
        parts[0],
    );

    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(theme::ACCENT));
    let inner = block.inner(parts[1]);
    frame.render_widget(block, parts[1]);
    inner
}

/// Build a single field row with the standard layout:
/// `▸ Label         ◂ Value         ▸ `
pub(super) fn field_row(
    label: &str,
    value: String,
    focused: bool,
    disabled: bool,
) -> Line<'static> {
    let label_style = Style::default().fg(theme::BRIGHT);
    let val_style = Style::default().fg(theme::VAL).bold();
    let val_focused = Style::default()
        .fg(theme::ACCENT)
        .bold()
        .add_modifier(Modifier::REVERSED);
    let val_disabled = Style::default().fg(theme::DIM);
    let arrow = Style::default().fg(theme::ACCENT).bold();
    let dim = Style::default().fg(theme::DIM);

    let marker = if focused { "▸ " } else { "  " };
    let label_text = format!("{:<13}", label);
    let v_style = if disabled {
        val_disabled
    } else if focused {
        val_focused
    } else {
        val_style
    };
    let left = if focused && !disabled {
        Span::styled(" ◂ ", arrow)
    } else {
        Span::styled("   ", dim)
    };
    let right = if focused && !disabled {
        Span::styled(" ▸ ", arrow)
    } else {
        Span::styled("   ", dim)
    };
    Line::from(vec![
        Span::styled(marker.to_string(), if focused { arrow } else { dim }),
        Span::styled(label_text, label_style),
        left,
        Span::styled(format!("{:<14}", value), v_style),
        right,
    ])
}
