use crossterm::event::KeyCode;

use crate::dsp::{DeviceKind, DspCommand};

use super::{App, AppMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceField {
    Device,
    SampleRate,
    GainMode,
    ManualGain,
    /// Action row at the bottom: Connect when disconnected, Reconnect when connected.
    Connect,
}

impl SourceField {
    pub fn next(self) -> Self {
        match self {
            SourceField::Device => SourceField::SampleRate,
            SourceField::SampleRate => SourceField::GainMode,
            SourceField::GainMode => SourceField::ManualGain,
            SourceField::ManualGain => SourceField::Connect,
            SourceField::Connect => SourceField::Device,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            SourceField::Device => SourceField::Connect,
            SourceField::SampleRate => SourceField::Device,
            SourceField::GainMode => SourceField::SampleRate,
            SourceField::ManualGain => SourceField::GainMode,
            SourceField::Connect => SourceField::ManualGain,
        }
    }
}

impl App {
    pub fn current_sample_rate(&self) -> u32 {
        let opts = self.device_kind.sample_rate_options();
        opts[self.sample_rate_idx.min(opts.len() - 1)]
    }

    pub fn current_gain_db(&self) -> f64 {
        let opts = self.device_kind.gain_options_tenths();
        opts[self.gain_idx.min(opts.len() - 1)] as f64 / 10.0
    }

    /// Snap `sample_rate_idx` and `gain_idx` into the bounds of the current
    /// device's option lists. Called whenever the device changes.
    fn clamp_indices_to_device(&mut self) {
        let sr_len = self.device_kind.sample_rate_options().len();
        if self.sample_rate_idx >= sr_len {
            self.sample_rate_idx = self.device_kind.default_sample_rate_idx();
        }
        let g_len = self.device_kind.gain_options_tenths().len();
        if self.gain_idx >= g_len {
            self.gain_idx = self.device_kind.default_gain_idx();
        }
    }

    /// Cycle the focused source field's value left or right.
    /// Returns Some(DspCommand) if the change should be sent to DSP.
    pub fn source_cycle(&mut self, delta: i32) -> Option<DspCommand> {
        match self.source_focus {
            SourceField::Device => {
                // Cycle through devices, skipping unsupported ones.
                let mut next = self.device_kind;
                for _ in 0..DeviceKind::ALL.len() {
                    next = if delta >= 0 { next.next() } else { next.prev() };
                    if next.is_supported() {
                        break;
                    }
                }
                if next == self.device_kind {
                    return None;
                }
                self.device_kind = next;
                // Reset to the new device's defaults.
                self.sample_rate_idx = next.default_sample_rate_idx();
                self.gain_idx = next.default_gain_idx();
                self.clamp_indices_to_device();
                // Clamp current freq into the new device's range.
                let (lo, hi) = next.freq_range();
                self.center_freq = self.center_freq.clamp(lo, hi);
                // The running flowgraph is now stale — tear it down so the
                // user must explicitly Connect with the new device.
                if self.is_connected() {
                    self.request_disconnect();
                }
                self.source_focus = SourceField::Connect;
                None
            }
            SourceField::Connect => None, // not a value field
            SourceField::SampleRate => {
                let opts = self.device_kind.sample_rate_options();
                let len = opts.len() as i32;
                let new_idx = (self.sample_rate_idx as i32 + delta).rem_euclid(len) as usize;
                if new_idx == self.sample_rate_idx {
                    return None;
                }
                self.sample_rate_idx = new_idx;
                // Sample rate is baked into the flowgraph's decim factors, so
                // a rebuild is needed. We DON'T auto-disconnect — let the user
                // press Apply when ready (the Connect button label changes).
                None
            }
            SourceField::GainMode => {
                self.agc_enabled = !self.agc_enabled;
                if self.agc_enabled {
                    Some(DspCommand::SetAgcMode(true))
                } else {
                    let opts = self.device_kind.gain_options_tenths();
                    Some(DspCommand::SetManualGain(opts[self.gain_idx]))
                }
            }
            SourceField::ManualGain => {
                if self.agc_enabled {
                    return None;
                }
                let opts = self.device_kind.gain_options_tenths();
                let len = opts.len() as i32;
                let new_idx = (self.gain_idx as i32 + delta).rem_euclid(len) as usize;
                self.gain_idx = new_idx;
                Some(DspCommand::SetManualGain(opts[self.gain_idx]))
            }
        }
    }

    pub(super) fn handle_source_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
            }
            KeyCode::Enter => {
                if self.source_focus == SourceField::Connect {
                    self.request_connect();
                } else {
                    self.mode = AppMode::Normal;
                }
            }
            KeyCode::Up => {
                self.source_focus = self.source_focus.prev();
            }
            KeyCode::Down | KeyCode::Tab => {
                self.source_focus = self.source_focus.next();
            }
            KeyCode::BackTab => {
                self.source_focus = self.source_focus.prev();
            }
            KeyCode::Right => {
                if let Some(cmd) = self.source_cycle(1) {
                    self.send_cmd(cmd);
                }
            }
            KeyCode::Left => {
                if let Some(cmd) = self.source_cycle(-1) {
                    self.send_cmd(cmd);
                }
            }
            _ => {}
        }
    }
}
