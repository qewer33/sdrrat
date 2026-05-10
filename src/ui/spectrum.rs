use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph};

use crate::app::{App, FFT_SIZE};

use super::theme;
use super::util::downsample;

pub(super) fn draw(frame: &mut Frame, header_area: Rect, chart_area: Rect, app: &App) {
    // --- Header ---
    let header_style = Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG).bold();
    frame.render_widget(
        Paragraph::new(" Spectrum ").centered().style(header_style),
        header_area,
    );

    // Reserve the bottom row of `chart_area` for our custom MHz ruler.
    let chart_body = Rect {
        height: chart_area.height.saturating_sub(1),
        ..chart_area
    };
    let ruler_row = Rect {
        x: chart_area.x,
        y: chart_area.y + chart_area.height.saturating_sub(1),
        width: chart_area.width,
        height: 1,
    };

    // --- Line graph ---
    // The chart will reserve `y_label_width + 1` columns for the Y-axis labels
    // and axis line.  Hand it exactly as many points as it has plot columns
    // so that point i lines up with column i, and downsample with DC pinned to
    // the geometric middle so the spectrum data slides under a fixed overlay.
    let y_label_width = y_axis_label_width(app);
    let plot_cols = chart_body.width.saturating_sub(y_label_width + 1) as usize;
    let points = downsample(&app.spectrum, plot_cols, FFT_SIZE / 2);
    let x_max = if points.is_empty() {
        1.0
    } else {
        (points.len() - 1) as f64
    };

    let dataset = Dataset::default()
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(theme::TRACE))
        .data(&points);

    let x_axis = Axis::default()
        .bounds([0.0, x_max])
        .style(Style::default().fg(theme::AXIS));

    let y_mid = (app.y_min + app.y_max) / 2.0;
    let y_axis = Axis::default()
        .bounds([app.y_min, app.y_max])
        .labels(vec![
            format!("{:.0}", app.y_min),
            format!("{y_mid:.0}"),
            format!("{:.0}", app.y_max),
        ])
        .style(Style::default().fg(theme::AXIS));

    let chart = Chart::new(vec![dataset])
        .x_axis(x_axis)
        .y_axis(y_axis);

    frame.render_widget(chart, chart_body);

    // --- Frequency ruler at the bottom ---
    draw_freq_ruler(
        frame,
        ruler_row,
        y_label_width + 1,
        plot_cols,
        app.center_freq,
        app.current_sample_rate(),
    );

    // --- Overlays (toggleable): bandwidth shading + center frequency line ---
    if !app.show_overlay {
        return;
    }

    // Plot region offset = Y-axis label width + 1 (axis line column).
    // Width = same `plot_cols` we passed to `downsample`.
    // Height = the chart body (we reserved the bottom row for the ruler).
    let chart_inner = Rect {
        x: chart_body.x + y_label_width + 1,
        y: chart_body.y,
        width: plot_cols as u16,
        height: chart_body.height,
    };

    if chart_inner.width == 0 || chart_inner.height == 0 {
        return;
    }

    // DC is pinned to the geometric middle by the centered downsample, so the
    // overlay center line sits at width/2 regardless of FFT size or plot width.
    let dc_col = chart_inner.width / 2;

    let buf = frame.buffer_mut();

    // Bandwidth shading centered on DC — width follows the active demod mode.
    if let Some(bw_hz) = app.radio_mode.bandwidth_hz() {
        let bw_frac = bw_hz as f64 / app.current_sample_rate() as f64;
        let bw_half = ((chart_inner.width as f64) * bw_frac / 2.0) as u16;
        let bw_start = dc_col.saturating_sub(bw_half);
        let bw_end = (dc_col + bw_half).min(chart_inner.width);

        for row in 0..chart_inner.height {
            for col in bw_start..bw_end {
                let x = chart_inner.x + col;
                let y = chart_inner.y + row;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(theme::BW_SHADE);
                }
            }
        }
    }

    let center_col = chart_inner.x + dc_col;
    for row in 0..chart_inner.height {
        let y = chart_inner.y + row;
        if let Some(cell) = buf.cell_mut((center_col, y)) {
            cell.set_symbol("│").set_fg(theme::CENTER_LINE);
        }
    }
}

/// Render frequency tick labels along `row`. Step size adapts to the visible
/// span so the user sees ~4-7 labels regardless of the sample rate.
fn draw_freq_ruler(
    frame: &mut Frame,
    row: Rect,
    plot_x_offset: u16,
    plot_cols: usize,
    center_freq: u64,
    sample_rate: u32,
) {
    if plot_cols == 0 || row.width == 0 {
        return;
    }

    let span = sample_rate as f64;
    let center = center_freq as f64;
    let lo = center - span / 2.0;
    let hi = center + span / 2.0;

    // Pick a "nice" step from {1, 2, 5} × 10^k that yields ~4-7 labels.
    let target_count = 5.0;
    let raw_step = span / target_count;
    let step_hz = nice_step(raw_step);

    // First tick at or above `lo` that's a multiple of step_hz.
    let first = (lo / step_hz).ceil() as i64;
    let last = (hi / step_hz).floor() as i64;

    let mut line_chars: Vec<char> = vec![' '; plot_cols];
    for k in first..=last {
        let hz = (k as f64) * step_hz;
        let frac = ((hz - lo) / span).clamp(0.0, 1.0);
        let col_center = (frac * (plot_cols as f64 - 1.0)).round() as usize;
        let label = fmt_freq_label(hz, step_hz);
        let half = label.chars().count() / 2;
        let start = col_center.saturating_sub(half);
        for (i, ch) in label.chars().enumerate() {
            let pos = start + i;
            if pos < plot_cols {
                line_chars[pos] = ch;
            }
        }
    }

    let label_text: String = line_chars.into_iter().collect();
    let label_area = Rect {
        x: row.x + plot_x_offset,
        y: row.y,
        width: plot_cols as u16,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(label_text).style(Style::default().fg(theme::AXIS)),
        label_area,
    );
}

/// Snap `raw` (Hz) to the nearest "nice" step (1·10^k, 2·10^k, 5·10^k).
fn nice_step(raw: f64) -> f64 {
    if raw <= 0.0 {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10f64.powf(exp);
    let mantissa = raw / base;
    let snapped = if mantissa < 1.5 {
        1.0
    } else if mantissa < 3.5 {
        2.0
    } else if mantissa < 7.5 {
        5.0
    } else {
        10.0
    };
    snapped * base
}

/// Format a frequency label, picking precision based on the step size so
/// labels are unique but not overly long.
fn fmt_freq_label(hz: f64, step_hz: f64) -> String {
    let mhz = hz / 1_000_000.0;
    if step_hz >= 1_000_000.0 {
        format!("{:.0}M", mhz)
    } else if step_hz >= 100_000.0 {
        format!("{:.1}M", mhz)
    } else if step_hz >= 10_000.0 {
        format!("{:.2}M", mhz)
    } else {
        format!("{:.3}M", mhz)
    }
}

fn y_axis_label_width(app: &App) -> u16 {
    let y_mid = (app.y_min + app.y_max) / 2.0;
    [app.y_min, y_mid, app.y_max]
        .iter()
        .map(|v| format!("{:.0}", v).len())
        .max()
        .unwrap_or(0) as u16
}
