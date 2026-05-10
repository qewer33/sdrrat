mod db_range;
mod persist;
mod radio;
mod source;
mod vfo;

pub use persist::{load as load_persisted, save as save_persisted};

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::{Receiver, Sender};
use crossterm::event::KeyCode;

use crate::dsp::{DeviceKind, DspCommand};

pub use db_range::MinMaxFocus;
pub use radio::{RadioField, RadioMode};
pub use source::SourceField;

pub const FFT_SIZE: usize = 1024;
pub const WATERFALL_MAX_ROWS: usize = 100;

/// Total number of digits displayed in the VFO (supports up to 9,999,999,999 Hz).
pub const VFO_DIGITS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    EditingFrequency,
    EditingMinMax,
    EditingSource,
    EditingRadio,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected { last_error: Option<String> },
    Connected,
}

pub struct App {
    pub running: bool,
    pub mode: AppMode,
    pub spectrum: Vec<f32>,
    pub waterfall: VecDeque<Vec<f32>>,
    pub rx: Option<Receiver<Vec<f32>>>,
    pub cmd_tx: Option<Sender<DspCommand>>,

    /// Whether the DSP thread is currently running.
    pub connection: ConnectionState,
    /// Set when the user presses Connect in the Source popup.
    /// Main loop consumes it via [`App::take_connect_request`].
    connect_requested: bool,
    /// Set when a config change makes the running flowgraph stale (e.g. the
    /// user picked a different device). Main loop tears the DSP thread down.
    disconnect_requested: bool,

    // VFO state
    pub center_freq: u64,
    /// Which digit is selected (0 = ones place, 9 = billions place).
    pub vfo_cursor: usize,
    /// Snapshot of frequency when the popup was opened, for cancel/restore.
    pub vfo_freq_snapshot: u64,

    // Overlay toggle
    pub show_overlay: bool,

    // Y-axis bounds
    pub y_min: f64,
    pub y_max: f64,
    pub minmax_focus: MinMaxFocus,
    pub y_min_snapshot: f64,
    pub y_max_snapshot: f64,

    // Source popup state
    pub source_focus: SourceField,
    /// Currently selected SDR device.
    pub device_kind: DeviceKind,
    /// Currently selected sample rate index into SAMPLE_RATE_OPTIONS.
    pub sample_rate_idx: usize,
    /// AGC enabled (true) vs manual gain mode (false).
    pub agc_enabled: bool,
    /// Currently selected manual gain index into GAIN_OPTIONS_TENTHS.
    pub gain_idx: usize,

    // Radio popup state
    pub radio_focus: RadioField,
    pub radio_mode: RadioMode,
    /// Last non-Off mode, restored when `d` un-mutes.
    pub last_radio_mode: RadioMode,
    /// Squelch threshold in dB.
    pub squelch_db: f32,
}

impl App {
    pub fn new() -> Self {
        Self {
            running: true,
            mode: AppMode::Normal,
            spectrum: vec![0.0; FFT_SIZE],
            waterfall: VecDeque::with_capacity(WATERFALL_MAX_ROWS),
            rx: None,
            cmd_tx: None,
            connection: ConnectionState::Disconnected { last_error: None },
            connect_requested: false,
            disconnect_requested: false,
            show_overlay: true,
            center_freq: 100_000_000,
            vfo_cursor: 6, // start on the MHz digit
            vfo_freq_snapshot: 0,
            y_min: -10.0,
            y_max: 100.0,
            minmax_focus: MinMaxFocus::Min,
            y_min_snapshot: 0.0,
            y_max_snapshot: 0.0,
            source_focus: SourceField::Device,
            device_kind: DeviceKind::default(),
            sample_rate_idx: DeviceKind::default().default_sample_rate_idx(),
            agc_enabled: true,
            gain_idx: DeviceKind::default().default_gain_idx(),
            radio_focus: RadioField::Mode,
            radio_mode: RadioMode::Off,
            last_radio_mode: RadioMode::WBFM,
            squelch_db: -50.0,
        }
    }

    /// Drain all pending FFT frames from the channel, keeping state up to date.
    pub fn poll_data(&mut self) {
        let Some(rx) = &self.rx else { return };
        while let Ok(fft_data) = rx.try_recv() {
            self.spectrum = fft_data.clone();
            self.waterfall.push_front(fft_data);
            if self.waterfall.len() > WATERFALL_MAX_ROWS {
                self.waterfall.pop_back();
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection, ConnectionState::Connected)
    }

    /// Hand the App a freshly-built channel pair (after a successful connect).
    pub fn set_connected(
        &mut self,
        rx: Receiver<Vec<f32>>,
        cmd_tx: Sender<DspCommand>,
    ) {
        self.rx = Some(rx);
        self.cmd_tx = Some(cmd_tx);
        self.connection = ConnectionState::Connected;
        // Push current state to the new DSP thread.
        self.send_freq();
        self.send_cmd(DspCommand::SetRadioMode(self.radio_mode.code()));
        self.send_cmd(DspCommand::SetSquelchDb(self.squelch_db));
    }

    pub fn set_disconnected(&mut self, error: Option<String>) {
        self.rx = None;
        self.cmd_tx = None;
        self.connection = ConnectionState::Disconnected { last_error: error };
        self.spectrum = vec![0.0; FFT_SIZE];
        self.waterfall.clear();
    }

    /// User asked to (re)connect via the Source popup. Polled by the main loop.
    pub fn request_connect(&mut self) {
        self.connect_requested = true;
    }

    /// Returns true exactly once after `request_connect` is called.
    pub fn take_connect_request(&mut self) -> bool {
        std::mem::replace(&mut self.connect_requested, false)
    }

    /// Mark the running DSP flowgraph as stale (e.g. after device or sample
    /// rate change). Polled by the main loop, which tears down the thread.
    pub fn request_disconnect(&mut self) {
        self.disconnect_requested = true;
    }

    pub fn take_disconnect_request(&mut self) -> bool {
        std::mem::replace(&mut self.disconnect_requested, false)
    }

    /// True when the radio is producing audio (any mode other than Off).
    pub fn audio_active(&self) -> bool {
        self.radio_mode != RadioMode::Off
    }

    /// Send the current frequency to the DSP thread (no-op if disconnected).
    /// Also drains any pre-retune spectrum frames sitting in the UI channel
    /// so the spectrum visually updates as soon as new data arrives.
    pub fn send_freq(&self) {
        if let Some(rx) = &self.rx {
            while rx.try_recv().is_ok() {}
        }
        self.send_cmd(DspCommand::TuneFrequency(self.center_freq as u32));
    }

    pub fn send_cmd(&self, cmd: DspCommand) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.try_send(cmd);
        }
    }

    /// Top-level keyboard dispatch. Routes the keypress to the appropriate
    /// per-mode handler.
    pub fn handle_key(&mut self, code: KeyCode, quit: &AtomicBool) {
        match self.mode {
            AppMode::Normal => self.handle_normal_key(code, quit),
            AppMode::EditingFrequency => self.handle_freq_key(code),
            AppMode::EditingMinMax => self.handle_minmax_key(code),
            AppMode::EditingSource => self.handle_source_key(code),
            AppMode::EditingRadio => self.handle_radio_key(code),
            AppMode::Help => self.handle_help_key(code),
        }
    }

    fn handle_help_key(&mut self, code: KeyCode) {
        // Any of these closes the popup; otherwise ignore.
        match code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('h') | KeyCode::Char('q') => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode, quit: &AtomicBool) {
        match code {
            KeyCode::Char('q') => {
                self.running = false;
                quit.store(true, Ordering::Relaxed);
            }
            KeyCode::Char('f') => {
                self.vfo_freq_snapshot = self.center_freq;
                self.mode = AppMode::EditingFrequency;
            }
            KeyCode::Char('m') => {
                self.y_min_snapshot = self.y_min;
                self.y_max_snapshot = self.y_max;
                self.minmax_focus = MinMaxFocus::Min;
                self.mode = AppMode::EditingMinMax;
            }
            KeyCode::Char('o') => {
                self.show_overlay = !self.show_overlay;
            }
            KeyCode::Char('d') => {
                self.toggle_mute();
            }
            KeyCode::Char('s') => {
                self.source_focus = SourceField::Device;
                self.mode = AppMode::EditingSource;
            }
            KeyCode::Char('r') => {
                self.radio_focus = RadioField::Mode;
                self.mode = AppMode::EditingRadio;
            }
            KeyCode::Char('h') => {
                self.mode = AppMode::Help;
            }
            KeyCode::Right => {
                let new_freq = self.center_freq.saturating_add(100_000);
                self.set_center_freq_clamped(new_freq);
            }
            KeyCode::Left => {
                let new_freq = self.center_freq.saturating_sub(100_000);
                self.set_center_freq_clamped(new_freq);
            }
            _ => {}
        }
    }
}
