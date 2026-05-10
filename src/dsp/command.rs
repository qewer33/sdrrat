use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use futuresdr::prelude::*;

use super::silence;

/// Commands sent from the UI thread to the DSP thread.
#[allow(dead_code)]
pub enum DspCommand {
    TuneFrequency(u32),
    SetSampleRate(u32),
    SetAgcMode(bool),
    SetManualGain(i32),
    /// 0 = Off, 1 = WBFM, 2 = NBFM, 3 = AM
    SetRadioMode(u8),
    SetSquelchDb(f32),
}

/// Lock-free state shared between the flowgraph blocks and the command pump.
///
/// All fields use atomic types so blocks can read them every sample without
/// blocking the audio thread.  `signal_power_db` and `squelch_db` are stored
/// as `f32::to_bits()` inside an `AtomicU32`.
#[derive(Clone)]
pub(super) struct Atoms {
    /// Most recent average signal power on the post-decim narrow IQ stream.
    pub signal_power_db: Arc<AtomicU32>,
    /// User-configured squelch threshold (dB).
    pub squelch_db: Arc<AtomicU32>,
    /// Current radio mode: see `MODE_*` constants in `super`.
    pub radio_mode: Arc<AtomicU8>,
}

impl Atoms {
    pub(super) fn new() -> Self {
        Self {
            signal_power_db: Arc::new(AtomicU32::new(f32::to_bits(-100.0))),
            squelch_db: Arc::new(AtomicU32::new(f32::to_bits(-50.0))),
            radio_mode: Arc::new(AtomicU8::new(super::MODE_OFF)),
        }
    }
}

pub(super) async fn apply_command(
    handle: &FlowgraphHandle,
    src: BlockId,
    atoms: &Atoms,
    cmd: DspCommand,
) {
    let (port, pmt) = match cmd {
        DspCommand::TuneFrequency(hz) => ("freq", Pmt::F64(hz as f64)),
        DspCommand::SetSampleRate(hz) => ("sample_rate", Pmt::F64(hz as f64)),
        DspCommand::SetAgcMode(_) => return, // AGC not exposed via seify message ports
        DspCommand::SetManualGain(tenths) => ("gain", Pmt::F64(tenths as f64 / 10.0)),
        DspCommand::SetRadioMode(mode) => {
            atoms.radio_mode.store(mode, Ordering::Relaxed);
            return;
        }
        DspCommand::SetSquelchDb(db) => {
            atoms.squelch_db.store(f32::to_bits(db), Ordering::Relaxed);
            return;
        }
    };
    // Hold a stderr-silence guard for the duration of the post — the source
    // block's freq / sample_rate / gain handlers call into libSoapySDR which
    // emits noisy `[INFO]` lines that would otherwise corrupt ratatui.
    let fut = handle.post(src, port, pmt);
    let _guard = silence::SilencedStderr::new();
    let _ = fut.await;
}
