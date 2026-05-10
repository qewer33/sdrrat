//! Mock DSP thread for `--test` mode (not yet ported to FutureSDR).

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;

use crossbeam_channel::{Receiver, Sender};

use super::command::DspCommand;

pub fn spawn(
    _tx: Sender<Vec<f32>>,
    _cmd_rx: Receiver<DspCommand>,
    _quit: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(|| {
        eprintln!("--test mode not yet ported to FutureSDR backend");
    })
}
