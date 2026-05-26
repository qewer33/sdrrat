use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, tune_step_label};

use super::theme;

pub(super) fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let key_style = Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG);
    let desc_style = Style::default().fg(theme::HEADER_FG).bg(Color::Black);
    let mute_label = if app.audio_active() { " Mute  " } else { " Unmute " };
    let stream_label = if app.is_connected() { " Stop  " } else { " Start  " };
    let bar = Line::from(vec![
        Span::styled(" ←→ ", key_style),
        Span::styled(format!(" ±{}  ", tune_step_label(app.tune_step_idx)), desc_style),
        Span::styled(" Space ", key_style),
        Span::styled(stream_label, desc_style),
        Span::styled(" o ", key_style),
        Span::styled(" Overlay  ", desc_style),
        Span::styled(" d ", key_style),
        Span::styled(mute_label, desc_style),
        Span::styled(" s ", key_style),
        Span::styled(" Source  ", desc_style),
        Span::styled(" r ", key_style),
        Span::styled(" Radio  ", desc_style),
        Span::styled(" h ", key_style),
        Span::styled(" Help  ", desc_style),
        Span::styled(" q ", key_style),
        Span::styled(" Quit  ", desc_style),
    ]);
    frame.render_widget(
        Paragraph::new(vec![Line::default(), bar, Line::default()])
            .centered()
            .style(desc_style),
        area,
    );
}
