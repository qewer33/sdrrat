//! Supported SDR device kinds. Each variant knows its display name, the
//! seify/SoapySDR args needed to open it, and its tunable frequency range.
//!
//! Add a variant + handful of match arms here when adding a new device.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceKind {
    RtlSdr,
    HackRf,
}

impl DeviceKind {
    pub const ALL: &'static [DeviceKind] = &[DeviceKind::RtlSdr, DeviceKind::HackRf];

    pub fn label(self) -> &'static str {
        match self {
            DeviceKind::RtlSdr => "RTL-SDR",
            DeviceKind::HackRf => "HackRF",
        }
    }

    /// `true` when the device kind has a working open path. New devices start
    /// `false` and flip to `true` once the open/configure/streaming code is
    /// wired up. Cycling skips unsupported kinds.
    pub fn is_supported(self) -> bool {
        match self {
            DeviceKind::RtlSdr => true,
            DeviceKind::HackRf => true,
        }
    }

    /// Sample rates this device kind supports, in Hz. The order matches the
    /// indices saved in `App::sample_rate_idx`.
    pub fn sample_rate_options(self) -> &'static [u32] {
        match self {
            DeviceKind::RtlSdr => &[
                250_000, 1_024_000, 1_536_000, 1_792_000, 1_920_000, 2_048_000, 2_400_000, 2_560_000,
            ],
            DeviceKind::HackRf => &[
                2_000_000, 4_000_000, 8_000_000, 10_000_000, 12_500_000, 16_000_000, 20_000_000,
            ],
        }
    }

    /// Index of the default sample rate within `sample_rate_options()`.
    pub fn default_sample_rate_idx(self) -> usize {
        match self {
            DeviceKind::RtlSdr => 1, // 1.024 MSPS
            DeviceKind::HackRf => 0, // 2 MSPS — matches RTL-SDR's intermediate-rate math
        }
    }

    /// Manual-gain options in tenths of a dB. The seify/SoapySDR backend
    /// distributes these across the device's gain stages internally.
    pub fn gain_options_tenths(self) -> &'static [i32] {
        match self {
            // R820T2 / R828D supported gain steps (`SoapyRTLSDR`).
            DeviceKind::RtlSdr => &[
                0, 9, 14, 27, 37, 77, 87, 125, 144, 157, 166, 197, 207, 229, 254, 280, 297, 328,
                338, 364, 372, 386, 402, 421, 434, 439, 445, 480, 496,
            ],
            // HackRF: 0-102 dB total (LNA 0-40 in 8-step + VGA 0-62 in 2-step,
            // SoapySDR distributes via overall set_gain). 0..=102 in 2 dB steps.
            DeviceKind::HackRf => &[
                0, 20, 40, 60, 80, 100, 120, 140, 160, 180, 200, 220, 240, 260, 280, 300, 320, 340,
                360, 380, 400, 420, 440, 460, 480, 500, 520, 540, 560, 580, 600, 620, 640, 660,
                680, 700, 720, 740, 760, 780, 800, 820, 840, 860, 880, 900, 920, 940, 960, 980,
                1000, 1020,
            ],
        }
    }

    /// Index of the default gain within `gain_options_tenths()`.
    pub fn default_gain_idx(self) -> usize {
        // Middle of the list — sensible "moderate gain" default.
        self.gain_options_tenths().len() / 2
    }

    /// SoapySDR / seify args string used to open this device.
    pub fn open_args(self) -> &'static str {
        match self {
            DeviceKind::RtlSdr => "driver=soapy,soapy_driver=rtlsdr",
            DeviceKind::HackRf => "driver=soapy,soapy_driver=hackrf",
        }
    }

    /// Tunable frequency range in Hz `(min, max)`.
    pub fn freq_range(self) -> (u64, u64) {
        match self {
            DeviceKind::RtlSdr => (24_000_000, 1_766_000_000),
            DeviceKind::HackRf => (1_000_000, 6_000_000_000),
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|d| *d == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let n = Self::ALL.len();
        let i = Self::ALL.iter().position(|d| *d == self).unwrap_or(0);
        Self::ALL[(i + n - 1) % n]
    }
}

impl Default for DeviceKind {
    fn default() -> Self {
        DeviceKind::RtlSdr
    }
}
