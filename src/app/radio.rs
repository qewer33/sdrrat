use crossterm::event::KeyCode;

use crate::dsp::DspCommand;

use super::{App, AppMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioMode {
    Off,
    WBFM,
    NBFM,
    AM,
}

impl RadioMode {
    pub fn label(self) -> &'static str {
        match self {
            RadioMode::Off => "Off",
            RadioMode::WBFM => "WBFM",
            RadioMode::NBFM => "NBFM",
            RadioMode::AM => "AM",
        }
    }

    /// True for modes that are wired up to a working DSP path.
    pub fn is_supported(self) -> bool {
        matches!(
            self,
            RadioMode::Off | RadioMode::WBFM | RadioMode::NBFM | RadioMode::AM
        )
    }

    /// Numeric code matching `dsp::MODE_*` constants.
    pub fn code(self) -> u8 {
        match self {
            RadioMode::Off => 0,
            RadioMode::WBFM => 1,
            RadioMode::NBFM => 2,
            RadioMode::AM => 3,
        }
    }

    /// Approximate occupied bandwidth in Hz, used for the spectrum overlay.
    /// Returns `None` for `Off` so the overlay can hide the bandwidth shading.
    pub fn bandwidth_hz(self) -> Option<u32> {
        match self {
            RadioMode::Off => None,
            RadioMode::WBFM => Some(200_000),
            RadioMode::NBFM => Some(12_500),
            RadioMode::AM => Some(10_000),
        }
    }

    pub fn next(self) -> Self {
        match self {
            RadioMode::Off => RadioMode::WBFM,
            RadioMode::WBFM => RadioMode::NBFM,
            RadioMode::NBFM => RadioMode::AM,
            RadioMode::AM => RadioMode::Off,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            RadioMode::Off => RadioMode::AM,
            RadioMode::WBFM => RadioMode::Off,
            RadioMode::NBFM => RadioMode::WBFM,
            RadioMode::AM => RadioMode::NBFM,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioField {
    Mode,
    Squelch,
}

impl RadioField {
    pub fn next(self) -> Self {
        match self {
            RadioField::Mode => RadioField::Squelch,
            RadioField::Squelch => RadioField::Mode,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }
}

const SQUELCH_MIN: f32 = -100.0;
const SQUELCH_MAX: f32 = 0.0;
const SQUELCH_STEP: f32 = 5.0;

impl App {
    /// Toggle between Off and the last non-Off mode (`d` hotkey).
    pub fn toggle_mute(&mut self) {
        if self.radio_mode == RadioMode::Off {
            self.set_radio_mode(self.last_radio_mode);
        } else {
            self.last_radio_mode = self.radio_mode;
            self.set_radio_mode(RadioMode::Off);
        }
    }

    fn set_radio_mode(&mut self, mode: RadioMode) {
        self.radio_mode = mode;
        self.send_cmd(DspCommand::SetRadioMode(mode.code()));
    }

    fn radio_cycle(&mut self, delta: i32) {
        match self.radio_focus {
            RadioField::Mode => {
                let mut next = self.radio_mode;
                // Skip unsupported modes when cycling.
                for _ in 0..4 {
                    next = if delta >= 0 { next.next() } else { next.prev() };
                    if next.is_supported() {
                        break;
                    }
                }
                if next != RadioMode::Off {
                    self.last_radio_mode = next;
                }
                self.set_radio_mode(next);
            }
            RadioField::Squelch => {
                let step = SQUELCH_STEP * delta.signum() as f32;
                self.squelch_db = (self.squelch_db + step).clamp(SQUELCH_MIN, SQUELCH_MAX);
                self.send_cmd(DspCommand::SetSquelchDb(self.squelch_db));
            }
        }
    }

    /// Squelch as a 0.0..1.0 value for slider display.
    pub fn squelch_fraction(&self) -> f32 {
        ((self.squelch_db - SQUELCH_MIN) / (SQUELCH_MAX - SQUELCH_MIN)).clamp(0.0, 1.0)
    }

    pub(super) fn handle_radio_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc | KeyCode::Enter => {
                self.mode = AppMode::Normal;
            }
            KeyCode::Up => {
                self.radio_focus = self.radio_focus.prev();
            }
            KeyCode::Down | KeyCode::Tab | KeyCode::BackTab => {
                self.radio_focus = self.radio_focus.next();
            }
            KeyCode::Right => self.radio_cycle(1),
            KeyCode::Left => self.radio_cycle(-1),
            _ => {}
        }
    }
}
