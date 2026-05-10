//! Flowgraph builder: assembles the FFT path plus the WBFM and Narrow
//! demod chains, and wires them into a `Runtime` that runs until quit.
//!
//! ## Adding a new demod chain (e.g. SSB / CW)
//!
//! Each chain follows the same shape:
//!     decim (Complex32, Fs → Fs/N) → [optional power meter] →
//!     demod (Complex32 → f32) → resamp (f32, Fs/N → 48 kHz) →
//!     [optional de-emphasis] → volume gate (gates on mode + squelch)
//!
//! The volume gate's output is summed into the audio mixer. To add a third
//! chain (e.g. SSB at ~6 kHz bandwidth):
//!   1. Add a `MODE_SSB` constant in `super::mod`.
//!   2. Add bandwidth/decim constants here and build the chain blocks
//!      following the WBFM / Narrow patterns below.
//!   3. Add a third `Tee` (or extend the fan-out) to feed it.
//!   4. Replace `Combine` with a 3-input mixer (chain two `Combine`s).
//!   5. Update the volume gate to include `MODE_SSB` in its enable check.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use futuresdr::blocks::audio::AudioSink;
use futuresdr::blocks::seify::Builder as SeifyBuilder;
use futuresdr::blocks::{Apply, Combine, Fft, FftDirection, FirBuilder};
use futuresdr::num_complex::Complex32;
use futuresdr::prelude::*;
use futuresdr::seify::{Device, GenericDevice};

use crate::app::FFT_SIZE;

use super::command::{Atoms, DspCommand, apply_command};
use super::{MODE_AM, MODE_NBFM, MODE_WBFM, sink, tee};

const DEFAULT_GAIN: f64 = 40.0;
const AUDIO_RATE: u32 = 48_000;

/// Target intermediate rate for the WBFM chain (Hz). The decimation factor
/// is picked so that `sample_rate / decim` ≈ this value.
const WBFM_TARGET_RATE: u32 = 256_000;
/// Target intermediate rate for the Narrow chain (NBFM / AM).
const NB_TARGET_RATE: u32 = 32_000;

pub(super) fn run(
    dev: Device<GenericDevice>,
    sample_rate: u32,
    initial_freq: u64,
    tx: Sender<Vec<f32>>,
    cmd_rx: Receiver<DspCommand>,
    quit: Arc<AtomicBool>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut fg = Flowgraph::new();

    // Pick decimation factors so each chain's intermediate rate is close to
    // its target. With smaller-than-target sample rates `decim` falls back
    // to 1 (chains run at the input rate; not ideal for WBFM but functional).
    let wbfm_decim = ((sample_rate as f64) / (WBFM_TARGET_RATE as f64))
        .round()
        .max(1.0) as usize;
    let wbfm_rate = sample_rate / wbfm_decim as u32;
    let nb_decim = ((sample_rate as f64) / (NB_TARGET_RATE as f64))
        .round()
        .max(1.0) as usize;
    let nb_rate = sample_rate / nb_decim as u32;

    // Source built from the pre-opened device — no I/O happens here.
    let src = SeifyBuilder::from_device(dev)
        .frequency(initial_freq as f64)
        .sample_rate(sample_rate as f64)
        .gain(DEFAULT_GAIN)
        .build_source()?;

    let atoms = Atoms::new();

    // Two-stage fan-out: src → tee1 (FFT vs everything-else)
    //                          → tee2 (WBFM chain vs Narrow chain)
    let tee1: tee::Tee = tee::Tee::new();
    let tee2: tee::Tee = tee::Tee::new();

    // ── Spectrum (FFT) path ─────────────────────────────────────────────
    let fft = Fft::with_options(FFT_SIZE, FftDirection::Forward, true, None);
    let mag = Apply::new(|c: &Complex32| -> f32 {
        10.0 * (c.norm_sqr() + 1e-20).log10()
    });
    let snk = sink::ChunkSink::new(FFT_SIZE, tx);

    // ── WBFM chain: source → wbfm_rate → 48 kHz ────────────────────────
    let wbfm_decim_block =
        FirBuilder::decimating::<Complex32, Complex32, Vec<f32>>(wbfm_decim);

    // WBFM discriminator — gain set so ±75 kHz deviation maps to ±1.
    let mut last_wbfm = Complex32::new(0.0, 0.0);
    let wbfm_gain = (wbfm_rate as f32) / (2.0 * std::f32::consts::PI * 75_000.0);
    let wbfm_demod = Apply::new(move |c: &Complex32| -> f32 {
        let phase = (c * last_wbfm.conj()).arg();
        last_wbfm = *c;
        phase * wbfm_gain
    });

    // Rational resampler from `wbfm_rate` down to AUDIO_RATE (48 kHz).
    // FirBuilder::resampling reduces by gcd internally so we just pass raw rates.
    let wbfm_resamp =
        FirBuilder::resampling::<f32, f32>(AUDIO_RATE as usize, wbfm_rate as usize);

    // De-emphasis: single-pole IIR low-pass τ = 75µs (US FM broadcast).
    let tau_s: f32 = 75e-6;
    let alpha: f32 = (-1.0 / (AUDIO_RATE as f32 * tau_s)).exp();
    let mut y_prev: f32 = 0.0;
    let deemph = Apply::new(move |x: &f32| -> f32 {
        let y = (1.0 - alpha) * *x + alpha * y_prev;
        y_prev = y;
        y
    });

    // WBFM volume gate — non-zero only when mode == WBFM and squelch is open.
    let vol_a_mode = Arc::clone(&atoms.radio_mode);
    let vol_a_pm = Arc::clone(&atoms.signal_power_db);
    let vol_a_sq = Arc::clone(&atoms.squelch_db);
    let volume_a = Apply::new(move |s: &f32| -> f32 {
        if vol_a_mode.load(Ordering::Relaxed) != MODE_WBFM {
            return 0.0;
        }
        let power = f32::from_bits(vol_a_pm.load(Ordering::Relaxed));
        let threshold = f32::from_bits(vol_a_sq.load(Ordering::Relaxed));
        if power < threshold { 0.0 } else { *s * 0.3 }
    });

    // ── Narrow chain (NBFM / AM): source → nb_rate → 48 kHz ────────────
    let nb_decim_block =
        FirBuilder::decimating::<Complex32, Complex32, Vec<f32>>(nb_decim);

    // Power meter on the narrow filtered IQ — feeds the squelch threshold.
    let pm_w = Arc::clone(&atoms.signal_power_db);
    let mut pm_sum = 0.0_f32;
    let mut pm_count: u32 = 0;
    let power_meter = Apply::new(move |c: &Complex32| -> Complex32 {
        pm_sum += c.norm_sqr();
        pm_count += 1;
        if pm_count >= 256 {
            let avg = pm_sum / pm_count as f32;
            let db = 10.0 * (avg + 1e-20).log10();
            pm_w.store(f32::to_bits(db), Ordering::Relaxed);
            pm_sum = 0.0;
            pm_count = 0;
        }
        *c
    });

    // Mode-dispatched discriminator for NBFM and AM.
    let nb_mode = Arc::clone(&atoms.radio_mode);
    let nbfm_gain = (nb_rate as f32) / (2.0 * std::f32::consts::PI * 5_000.0);
    let mut last_nb = Complex32::new(0.0, 0.0);
    let mut am_dc: f32 = 0.0;
    let dc_alpha: f32 = 0.999; // single-pole DC tracker for AM
    let nb_demod = Apply::new(move |c: &Complex32| -> f32 {
        match nb_mode.load(Ordering::Relaxed) {
            MODE_NBFM => {
                let phase = (c * last_nb.conj()).arg();
                last_nb = *c;
                phase * nbfm_gain
            }
            MODE_AM => {
                let mag = c.norm();
                am_dc = dc_alpha * am_dc + (1.0 - dc_alpha) * mag;
                mag - am_dc
            }
            _ => 0.0, // not active — silence on this chain
        }
    });

    let nb_resamp =
        FirBuilder::resampling::<f32, f32>(AUDIO_RATE as usize, nb_rate as usize);

    let vol_b_mode = Arc::clone(&atoms.radio_mode);
    let vol_b_pm = Arc::clone(&atoms.signal_power_db);
    let vol_b_sq = Arc::clone(&atoms.squelch_db);
    let volume_b = Apply::new(move |s: &f32| -> f32 {
        let m = vol_b_mode.load(Ordering::Relaxed);
        if m != MODE_NBFM && m != MODE_AM {
            return 0.0;
        }
        let power = f32::from_bits(vol_b_pm.load(Ordering::Relaxed));
        let threshold = f32::from_bits(vol_b_sq.load(Ordering::Relaxed));
        if power < threshold { 0.0 } else { *s * 0.3 }
    });

    // ── Mixer + audio sink ──────────────────────────────────────────────
    let mixer = Combine::new(|a: &f32, b: &f32| -> f32 { a + b });
    let audio_sink = AudioSink::new(AUDIO_RATE, 1)?;

    // ── Wire it all up ──────────────────────────────────────────────────
    let src = fg.add(src);
    let tee1 = fg.add(tee1);
    let tee2 = fg.add(tee2);
    let fft = fg.add(fft);
    let mag = fg.add(mag);
    let snk = fg.add(snk);
    let wbfm_decim_block = fg.add(wbfm_decim_block);
    let wbfm_demod = fg.add(wbfm_demod);
    let wbfm_resamp = fg.add(wbfm_resamp);
    let deemph = fg.add(deemph);
    let volume_a = fg.add(volume_a);
    let nb_decim_block = fg.add(nb_decim_block);
    let power_meter = fg.add(power_meter);
    let nb_demod = fg.add(nb_demod);
    let nb_resamp = fg.add(nb_resamp);
    let volume_b = fg.add(volume_b);
    let mixer = fg.add(mixer);
    let audio_sink = fg.add(audio_sink);

    fg.stream(&src, |b| b.outputs().get_mut(0).unwrap(), &tee1, |b| b.input())?;
    // tee1: out_a → FFT, out_b → tee2
    fg.stream(&tee1, |b| b.out_a(), &fft, |b| b.input())?;
    fg.stream(&fft, |b| b.output(), &mag, |b| b.input())?;
    fg.stream(&mag, |b| b.output(), &snk, |b| b.input())?;
    fg.stream(&tee1, |b| b.out_b(), &tee2, |b| b.input())?;

    // tee2: out_a → WBFM chain, out_b → Narrow chain
    fg.stream(&tee2, |b| b.out_a(), &wbfm_decim_block, |b| b.input())?;
    fg.stream(&wbfm_decim_block, |b| b.output(), &wbfm_demod, |b| b.input())?;
    fg.stream(&wbfm_demod, |b| b.output(), &wbfm_resamp, |b| b.input())?;
    fg.stream(&wbfm_resamp, |b| b.output(), &deemph, |b| b.input())?;
    fg.stream(&deemph, |b| b.output(), &volume_a, |b| b.input())?;

    fg.stream(&tee2, |b| b.out_b(), &nb_decim_block, |b| b.input())?;
    fg.stream(&nb_decim_block, |b| b.output(), &power_meter, |b| b.input())?;
    fg.stream(&power_meter, |b| b.output(), &nb_demod, |b| b.input())?;
    fg.stream(&nb_demod, |b| b.output(), &nb_resamp, |b| b.input())?;
    fg.stream(&nb_resamp, |b| b.output(), &volume_b, |b| b.input())?;

    // Mixer: vol_a + vol_b → audio_sink
    fg.stream(&volume_a, |b| b.output(), &mixer, |b| b.in0())?;
    fg.stream(&volume_b, |b| b.output(), &mixer, |b| b.in1())?;
    fg.stream(&mixer, |b| b.output(), &audio_sink, |b| b.input())?;

    let src_id: BlockId = (&src).into();

    // Start the flowgraph and pump commands.
    let rt = Runtime::new();
    let running = rt.start(fg)?;
    let handle = running.handle();

    // Pump commands on a dedicated blocking thread. Using `recv_timeout`
    // wakes the instant a command lands (no polling latency) and still
    // checks the quit flag periodically.
    let cmd_quit = Arc::clone(&quit);
    let atoms_pump = atoms.clone();
    std::thread::spawn(move || {
        use futuresdr::futures::executor::block_on;
        loop {
            if cmd_quit.load(Ordering::Relaxed) {
                break;
            }
            match cmd_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(cmd) => block_on(apply_command(&handle, src_id, &atoms_pump, cmd)),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    // Block this thread until quit is signaled, then drop runtime to stop the flowgraph.
    while !quit.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(50));
    }
    drop(rt);
    Ok(())
}
