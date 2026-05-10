use std::collections::VecDeque;

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::widgets::{Paragraph, Widget};

use crate::app::{App, FFT_SIZE};

use super::theme;

const UPPER_HALF_BLOCK: &str = "▀";

/// Down-sample a single row to `target_width` f32 values (peak per chunk),
/// with `center_bin` aligned to the geometric middle of the output. Mirrors
/// the math in `super::util::downsample` so the waterfall lines up vertically
/// with the spectrum chart above it.
fn downsample_row(data: &[f32], target_width: usize, center_bin: usize) -> Vec<f32> {
    if target_width == 0 || data.is_empty() {
        return Vec::new();
    }

    let n = data.len() as isize;
    let chunk_size = data.len() / target_width;
    if chunk_size == 0 {
        return data.to_vec();
    }

    let half = (target_width / 2) as isize;
    let cs = chunk_size as isize;
    let start = center_bin as isize - half * cs;

    (0..target_width)
        .map(|i| {
            let cstart = start + (i as isize) * cs;
            let cend = cstart + cs;
            let lo = cstart.max(0).min(n) as usize;
            let hi = cend.max(0).min(n) as usize;
            if lo >= hi {
                f32::NEG_INFINITY
            } else {
                data[lo..hi]
                    .iter()
                    .cloned()
                    .fold(f32::NEG_INFINITY, f32::max)
            }
        })
        .collect()
}

/// Map a magnitude (in dB) to an RGB heat-map color.
fn magnitude_to_rgb(mag: f32, min_db: f32, max_db: f32) -> Color {
    let range = max_db - min_db;
    let t = if range <= 0.0 {
        0.0
    } else {
        ((mag - min_db) / range).clamp(0.0, 1.0)
    };

    let (r, g, b) = if t < 0.25 {
        let s = t / 0.25;
        (0.0, s, 1.0)
    } else if t < 0.5 {
        let s = (t - 0.25) / 0.25;
        (0.0, 1.0, 1.0 - s)
    } else if t < 0.75 {
        let s = (t - 0.5) / 0.25;
        (s, 1.0, 0.0)
    } else {
        let s = (t - 0.75) / 0.25;
        (1.0, 1.0 - s, 0.0)
    };

    Color::Rgb(
        (r * 255.0) as u8,
        (g * 255.0) as u8,
        (b * 255.0) as u8,
    )
}

struct WaterfallWidget<'a> {
    history: &'a VecDeque<Vec<f32>>,
    min_db: f32,
    max_db: f32,
}

impl<'a> WaterfallWidget<'a> {
    fn new(history: &'a VecDeque<Vec<f32>>, min_db: f32, max_db: f32) -> Self {
        Self {
            history,
            min_db,
            max_db,
        }
    }
}

impl Widget for WaterfallWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = area.width as usize;
        let height = area.height as usize;
        if width == 0 || height == 0 {
            return;
        }

        let needed = (height * 2).min(self.history.len());
        let rows: Vec<Vec<f32>> = self
            .history
            .iter()
            .take(needed)
            .map(|r| downsample_row(r, width, FFT_SIZE / 2))
            .collect();

        let black = Color::Black;

        for row in 0..height {
            let upper = rows.get(row * 2);
            let lower = rows.get(row * 2 + 1);

            for col in 0..width {
                let x = area.x + col as u16;
                let y = area.y + row as u16;

                let fg_color = upper
                    .and_then(|r| r.get(col))
                    .map(|&m| magnitude_to_rgb(m, self.min_db, self.max_db))
                    .unwrap_or(black);

                let bg_color = lower
                    .and_then(|r| r.get(col))
                    .map(|&m| magnitude_to_rgb(m, self.min_db, self.max_db))
                    .unwrap_or(black);

                let cell = buf.cell_mut((x, y)).unwrap();
                cell.set_symbol(UPPER_HALF_BLOCK)
                    .set_fg(fg_color)
                    .set_bg(bg_color);
            }
        }
    }
}

pub(super) fn draw(frame: &mut Frame, header_area: Rect, area: Rect, app: &App) {
    let header_style = Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG).bold();
    frame.render_widget(
        Paragraph::new(" Waterfall ").centered().style(header_style),
        header_area,
    );

    let widget = WaterfallWidget::new(&app.waterfall, app.y_min as f32, app.y_max as f32);
    frame.render_widget(widget, area);
}
