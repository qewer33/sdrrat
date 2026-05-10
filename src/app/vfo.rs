use crossterm::event::KeyCode;

use super::{App, AppMode, VFO_DIGITS};

impl App {
    pub(super) fn handle_freq_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.center_freq = self.vfo_freq_snapshot;
                self.send_freq();
                self.mode = AppMode::Normal;
            }
            KeyCode::Enter => {
                self.mode = AppMode::Normal;
            }
            KeyCode::Left => {
                if self.vfo_cursor < VFO_DIGITS - 1 {
                    self.vfo_cursor += 1;
                }
            }
            KeyCode::Right => {
                if self.vfo_cursor > 0 {
                    self.vfo_cursor -= 1;
                }
            }
            KeyCode::Up => self.vfo_increment(),
            KeyCode::Down => self.vfo_decrement(),
            KeyCode::Char('z') => self.vfo_zero_right(),
            _ => {}
        }
    }

    /// The place-value of the current cursor position: 10^vfo_cursor.
    pub fn vfo_step(&self) -> u64 {
        10u64.pow(self.vfo_cursor as u32)
    }

    pub fn vfo_increment(&mut self) {
        let new_freq = self.center_freq.saturating_add(self.vfo_step());
        self.set_center_freq_clamped(new_freq);
    }

    pub fn vfo_decrement(&mut self) {
        let new_freq = self.center_freq.saturating_sub(self.vfo_step());
        self.set_center_freq_clamped(new_freq);
    }

    /// Zero out all digits to the right of the cursor.
    pub fn vfo_zero_right(&mut self) {
        let step = self.vfo_step();
        let new_freq = self.center_freq / step * step;
        self.set_center_freq_clamped(new_freq);
    }

    /// Clamp `freq` into the current device's tunable range and update.
    pub fn set_center_freq_clamped(&mut self, freq: u64) {
        let (lo, hi) = self.device_kind.freq_range();
        let widget_max = 10u64.pow(VFO_DIGITS as u32) - 1;
        self.center_freq = freq.clamp(lo, hi.min(widget_max));
        self.send_freq();
    }
}
