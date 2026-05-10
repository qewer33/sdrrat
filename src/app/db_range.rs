use crossterm::event::KeyCode;

use super::{App, AppMode};

const DB_STEP: f64 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinMaxFocus {
    Min,
    Max,
}

impl App {
    pub(super) fn handle_minmax_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.y_min = self.y_min_snapshot;
                self.y_max = self.y_max_snapshot;
                self.mode = AppMode::Normal;
            }
            KeyCode::Enter => {
                self.mode = AppMode::Normal;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.minmax_focus = match self.minmax_focus {
                    MinMaxFocus::Min => MinMaxFocus::Max,
                    MinMaxFocus::Max => MinMaxFocus::Min,
                };
            }
            KeyCode::Up => self.minmax_step_up(),
            KeyCode::Down => self.minmax_step_down(),
            _ => {}
        }
    }

    pub fn minmax_step_up(&mut self) {
        match self.minmax_focus {
            MinMaxFocus::Min => self.y_min += DB_STEP,
            MinMaxFocus::Max => self.y_max += DB_STEP,
        }
        self.clamp_minmax();
    }

    pub fn minmax_step_down(&mut self) {
        match self.minmax_focus {
            MinMaxFocus::Min => self.y_min -= DB_STEP,
            MinMaxFocus::Max => self.y_max -= DB_STEP,
        }
        self.clamp_minmax();
    }

    /// Ensure min stays below max with at least one step of separation.
    pub(super) fn clamp_minmax(&mut self) {
        if self.y_min >= self.y_max - DB_STEP {
            match self.minmax_focus {
                MinMaxFocus::Min => self.y_min = self.y_max - DB_STEP,
                MinMaxFocus::Max => self.y_max = self.y_min + DB_STEP,
            }
        }
    }
}
