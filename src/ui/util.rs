use ratatui::layout::{Constraint, Flex, Layout, Rect};

/// Down-sample FFT magnitudes to fit `target_width` columns, with the bin
/// at `center_bin` aligned to the geometric middle of the output.
///
/// Chunks have size `data.len() / target_width` (truncated). The window slides
/// so that the chunk at output index `target_width / 2` starts exactly at
/// `center_bin`. Bins falling outside the window are dropped; chunks that fall
/// out of range are reported as -infinity (rendered as the dB floor).
///
/// Each chunk reports its **max** value so narrow spikes survive.
pub(super) fn downsample(
    data: &[f32],
    target_width: usize,
    center_bin: usize,
) -> Vec<(f64, f64)> {
    if target_width == 0 || data.is_empty() {
        return Vec::new();
    }

    let n = data.len() as isize;
    let chunk_size = data.len() / target_width;
    if chunk_size == 0 {
        // Fewer bins than columns — fall back to 1:1.
        return data
            .iter()
            .enumerate()
            .map(|(i, &v)| (i as f64, v as f64))
            .collect();
    }

    let half = (target_width / 2) as isize;
    let cs = chunk_size as isize;
    // Window starts so that chunk at output index `half` begins at center_bin.
    let start = center_bin as isize - half * cs;

    (0..target_width)
        .map(|i| {
            let cstart = start + (i as isize) * cs;
            let cend = cstart + cs;
            let lo = cstart.max(0).min(n) as usize;
            let hi = cend.max(0).min(n) as usize;
            let peak = if lo >= hi {
                f32::NEG_INFINITY
            } else {
                data[lo..hi]
                    .iter()
                    .cloned()
                    .fold(f32::NEG_INFINITY, f32::max)
            };
            (i as f64, peak as f64)
        })
        .collect()
}

/// Return a centered `Rect` of the given absolute size.
pub(super) fn centered_rect_abs(width: u16, height: u16, area: Rect) -> Rect {
    let v = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .split(v[0])[0]
}
