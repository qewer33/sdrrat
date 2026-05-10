mod command;
mod device_kind;
mod flowgraph;
mod mock;
mod silence;
mod sink;
mod tee;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use futuresdr::seify::{Device, GenericDevice};

pub use command::DspCommand;
pub use device_kind::DeviceKind;

/// Radio mode codes shared between UI and DSP via `Atoms::radio_mode`.
pub const MODE_OFF: u8 = 0;
pub const MODE_WBFM: u8 = 1;
pub const MODE_NBFM: u8 = 2;
pub const MODE_AM: u8 = 3;

/// Open the requested SDR device. Call this BEFORE `ratatui::init()` so that
/// the noisy SoapySDR / librtlsdr "[INFO] Opening..." prints land on the
/// normal terminal rather than corrupting the TUI buffer.
pub fn open_device(kind: DeviceKind) -> std::result::Result<Device<GenericDevice>, String> {
    silence::silenced(|| Device::from_args(kind.open_args()))
        .map_err(|e| format!("Failed to open {}: {e}", kind.label()))
}

/// Spawn the DSP/data thread running a FutureSDR flowgraph.
/// `dev` must be a pre-opened SDR device (see [`open_device`]). The flowgraph's
/// decimation factors are computed from `sample_rate`, so changing the SDR
/// rate at runtime requires reconnecting (which re-builds with the new rate).
pub fn spawn_dsp_thread(
    dev: Device<GenericDevice>,
    sample_rate: u32,
    initial_freq: u64,
    tx: Sender<Vec<f32>>,
    cmd_rx: Receiver<DspCommand>,
    quit: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let _ = flowgraph::run(dev, sample_rate, initial_freq, tx, cmd_rx, quit);
    })
}

/// Spawn the mock DSP thread used by `--test` mode.
pub fn spawn_mock_dsp_thread(
    tx: Sender<Vec<f32>>,
    cmd_rx: Receiver<DspCommand>,
    quit: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    mock::spawn(tx, cmd_rx, quit)
}
