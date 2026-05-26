//! Persisted user config: a small TOML file under the OS config dir
//! (e.g. `~/.config/sdrtui/sdrtui.toml` on Linux). Loaded once at startup,
//! written once at graceful exit.

use serde::{Deserialize, Serialize};

use crate::dsp::DeviceKind;

use super::{App, RadioMode};

const APP_NAME: &str = "sdrtui";
const CONFIG_NAME: &str = "sdrtui";

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedConfig {
    #[serde(default)]
    pub device_kind: DeviceKind,
    pub center_freq: u64,
    pub y_min: f64,
    pub y_max: f64,
    pub show_overlay: bool,
    pub radio_mode: String,
    pub last_radio_mode: String,
    pub squelch_db: f32,
    pub sample_rate_idx: usize,
    pub agc_enabled: bool,
    pub gain_idx: usize,
    #[serde(default = "default_tune_step_idx")]
    pub tune_step_idx: usize,
}

fn default_tune_step_idx() -> usize {
    super::radio::DEFAULT_TUNE_STEP_IDX
}

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            device_kind: DeviceKind::default(),
            center_freq: 100_000_000,
            y_min: -10.0,
            y_max: 100.0,
            show_overlay: true,
            radio_mode: "Off".into(),
            last_radio_mode: "WBFM".into(),
            squelch_db: -50.0,
            sample_rate_idx: 1,
            agc_enabled: true,
            gain_idx: DeviceKind::default().default_gain_idx(),
            tune_step_idx: default_tune_step_idx(),
        }
    }
}

/// Load config from disk. Returns `Default` on missing/corrupt file.
pub fn load() -> PersistedConfig {
    confy::load(APP_NAME, CONFIG_NAME).unwrap_or_default()
}

/// Save the App's current state to disk. Errors are silently ignored —
/// failing to persist shouldn't take the app down.
pub fn save(app: &App) {
    let cfg = PersistedConfig {
        device_kind: app.device_kind,
        center_freq: app.center_freq,
        y_min: app.y_min,
        y_max: app.y_max,
        show_overlay: app.show_overlay,
        radio_mode: mode_to_str(app.radio_mode).into(),
        last_radio_mode: mode_to_str(app.last_radio_mode).into(),
        squelch_db: app.squelch_db,
        sample_rate_idx: app.sample_rate_idx,
        agc_enabled: app.agc_enabled,
        gain_idx: app.gain_idx,
        tune_step_idx: app.tune_step_idx,
    };
    let _ = confy::store(APP_NAME, CONFIG_NAME, &cfg);
}

fn mode_to_str(m: RadioMode) -> &'static str {
    match m {
        RadioMode::Off => "Off",
        RadioMode::WBFM => "WBFM",
        RadioMode::NBFM => "NBFM",
        RadioMode::AM => "AM",
    }
}

fn mode_from_str(s: &str) -> RadioMode {
    match s {
        "WBFM" => RadioMode::WBFM,
        "NBFM" => RadioMode::NBFM,
        "AM" => RadioMode::AM,
        _ => RadioMode::Off,
    }
}

impl App {
    /// Apply a persisted config to the App, validating where appropriate.
    pub fn apply_persisted(&mut self, cfg: PersistedConfig) {
        self.device_kind = if cfg.device_kind.is_supported() {
            cfg.device_kind
        } else {
            DeviceKind::default()
        };
        // Clamp to the device's supported range.
        let (lo, hi) = self.device_kind.freq_range();
        self.center_freq = cfg.center_freq.clamp(lo, hi);
        self.y_min = cfg.y_min;
        self.y_max = cfg.y_max;
        // Guard against invalid bounds in config.
        if self.y_min >= self.y_max {
            self.y_min = -10.0;
            self.y_max = 100.0;
        }
        self.show_overlay = cfg.show_overlay;
        self.radio_mode = mode_from_str(&cfg.radio_mode);
        let last = mode_from_str(&cfg.last_radio_mode);
        self.last_radio_mode = if last == RadioMode::Off { RadioMode::WBFM } else { last };
        self.squelch_db = cfg.squelch_db.clamp(-100.0, 0.0);

        if cfg.sample_rate_idx < self.device_kind.sample_rate_options().len() {
            self.sample_rate_idx = cfg.sample_rate_idx;
        } else {
            self.sample_rate_idx = self.device_kind.default_sample_rate_idx();
        }
        self.agc_enabled = cfg.agc_enabled;
        if cfg.gain_idx < self.device_kind.gain_options_tenths().len() {
            self.gain_idx = cfg.gain_idx;
        } else {
            self.gain_idx = self.device_kind.default_gain_idx();
        }
        if cfg.tune_step_idx < super::radio::tune_step_count() {
            self.tune_step_idx = cfg.tune_step_idx;
        } else {
            self.tune_step_idx = super::radio::DEFAULT_TUNE_STEP_IDX;
        }
    }
}
