![banner](./assets/sdrrat_banner.png)

**sdrrat** is a general purpose SDR (Software Defined Radio) receiver TUI (Terminal User Interface) application that interfaces with your SDR hardware and allows you to view the RF spectrum and demodulate signals from your terminal. It's built with Rust, [Ratatui](https://ratatui.rs) and [FutureSDR](https://www.futuresdr.org/).

> [!WARNING]
> sdrrat is currently **not stable**, please open an issue for any bugs or crashes you encounter

![screenshot](./assets/screenshot.png)
## Features

sdrrat has most of the basic features you can expect from an SDR receiver though it's lacking most advanced features. You can request any missing features by opening an issue.

- **RTL-SDR** and **HackRF** support
- **FFT spectrum** graph
- **Waterfall spectrogram** graph
- Source **sample rate** and **gain** control
- **WBFM, NBFM**  and **AM** demodulation
- Basic **squelch**
- **Intuitive and easy to use** terminal user interface (TUI)

Watch the showcase video below!

![video](./assets/showcase_video.mp4)

## Key Bindings

### Global

| Key | Action |
|-----|--------|
| `q` | Quit (config saves on exit) |
| `Space` | Start / Stop the data stream |
| `←` / `→` | Nudge frequency by ±0.1 MHz |
| `o` | Toggle the spectrum overlay (bandwidth shading + center line) |
| `d` | Mute / unmute audio (toggles between Off and the last active mode) |
| `f` | Focus the **VFO** for digit-by-digit tuning |
| `m` | Focus the **Min/Max** dB-range stepper |
| `s` | Open the **Source** popup (device, sample rate, gain) |
| `r` | Open the **Radio** popup (demod mode, squelch) |
| `h` | Open the in-app **Help** popup |

### VFO mode (after `f`)

| Key | Action |
|-----|--------|
| `←` / `→` | Move cursor left/right by one digit |
| `↑` / `↓` | Increment / decrement the focused digit (±10ⁿ) |
| `z` | Zero out all digits to the right of the cursor |
| `Enter` | Commit and exit |
| `Esc` | Cancel — restore the frequency from when the popup opened |

### Min/Max mode (after `m`)

| Key | Action |
|-----|--------|
| `Tab` | Switch focus between Min and Max |
| `↑` / `↓` | Adjust focused value by ±5 dB |
| `Enter` | Commit and exit |
| `Esc` | Cancel — restore prior bounds |

### Popups

| Key | Action |
|-----|--------|
| `Tab` / `↑` / `↓` | Move between fields |
| `←` / `→` | Cycle value |
| `Enter` | Confirm |
| `Esc` | Close |

## Architecture

sdrrat runs as two threads bridged by lock-free channels:

```
                ┌──────────────────────────────────┐
                │           UI thread              │
                │  ratatui draw + crossterm input  │
                └──────┬───────────────────┬───────┘
                       │                   │
                       │                   ▼
                       │            DspCommand channel
                       │       (TuneFrequency, SetMode, …)
                       │                   │
                       ▲                   ▼
                spectrum frames     ┌─────────────────────┐
                Vec<f32>            │   DSP thread        │
                       │            │   FutureSDR runtime │
                       └────────────┴─────────────────────┘
                                          │
                                          ▼
                                   SDR hardware
                                  (via SoapySDR)
```

### Threads & channels

- **UI thread** (main thread) — owns the App state, runs the ratatui draw loop, handles crossterm key events. Reads FFT magnitude frames from a bounded `crossbeam_channel` and pushes user commands onto another. Never blocks on I/O.
- **DSP thread** — hosts a single FutureSDR `Runtime` that runs the flowgraph (see below). A small command-pump task drains the UI command channel and forwards changes to the source block's message ports (frequency, sample rate, gain) or shared atomics (squelch, audio mode).
- **`Supervisor`** in `main.rs` owns the DSP thread's lifecycle. Connect / reconnect tears the thread down with a per-thread quit flag, joins it (so the device is fully released), then spawns a fresh one with new channel ends.

### FutureSDR flowgraph

```
seify::Source ──▶ Tee₁ ──▶ FFT ──▶ Magnitude ──▶ ChunkSink ──▶ UI channel
                    │
                    ▼
                  Tee₂ ──▶ WBFM chain (decim 4 → 256 kHz)
                    │       discriminator → resamp 3/16 → de-emphasis → vol_a
                    │
                    ▼      ────────┐
                  Narrow chain (decim 32 → 32 kHz)            │
                    power-meter → demod (NBFM/AM dispatch)    ├─▶ Combine ─▶ AudioSink (cpal, 48 kHz)
                    → resamp 3/2 → vol_b                      │
                                                              ┘
```

- **Spectrum path** runs an `Fft` (1024-bin, FFT-shifted) followed by a magnitude conversion (10·log₁₀ of `|c|²`) and a custom `ChunkSink` that batches f32 samples into FFT-sized frames pushed via a `crossbeam_channel` to the UI.
- **WBFM chain** decimates IQ to 256 kHz, runs a quadrature discriminator (FM ±75 kHz deviation), resamples to 48 kHz audio, applies 75 µs de-emphasis, then a volume gate.
- **Narrow chain** decimates harder (to 32 kHz) for NBFM and AM. A single `Apply` block dispatches to the right discriminator at runtime based on a shared `AtomicU8` mode flag (so mode changes don't require flowgraph restart). The narrow-bandwidth IQ stream also feeds a power meter that drives the squelch.
- **Mixer** (`Combine`) sums the two volume gates into the audio sink. Only one chain is non-zero at a time (the inactive volume gate outputs 0).
- **`Tee` and `ChunkSink`** are custom blocks since FutureSDR's stream buffers are single-reader. Both live in `dsp/`.

### Module layout

```
src/
├── main.rs            entry point + DSP supervisor
├── app/
│   ├── mod.rs         App struct, AppMode, channel I/O, key dispatch
│   ├── vfo.rs         frequency tuning helpers + VFO key handler
│   ├── db_range.rs    Y-axis stepper
│   ├── source.rs      Source popup state + sample-rate / gain options
│   ├── radio.rs       Radio popup state + mode helpers
│   └── persist.rs     TOML config load/save (via confy)
├── dsp/
│   ├── mod.rs         public API (open_device, spawn, MODE_* constants)
│   ├── device_kind.rs DeviceKind enum (RTL-SDR, HackRF) + per-device args/range
│   ├── command.rs     DspCommand + shared Atoms + apply_command pump
│   ├── flowgraph.rs   the full FutureSDR flowgraph builder
│   ├── tee.rs         1-in 2-out fanout block
│   ├── sink.rs        ChunkSink: batches f32 samples into FFT-sized frames
│   └── mock.rs        --test mode stub
└── ui/
    ├── mod.rs         top-level draw + disconnected placeholder
    ├── theme.rs       colour constants
    ├── util.rs        downsample (DC-centered) + centered_rect_abs
    ├── spectrum.rs    chart + overlay + frequency ruler
    ├── waterfall.rs   half-block waterfall widget
    ├── status_bar.rs  bottom controls strip
    ├── header/        VFO + dB range row
    └── popup/         shared popup chrome + Source / Radio popups
```

### Persistence

User config is stored as TOML under the OS config dir (`~/.config/sdrtui/sdrtui.toml` on Linux). Saved fields include the last frequency, dB range, demod mode, squelch, sample rate, gain, and selected device. The file is read once at startup and rewritten once at graceful exit.

## License

sdrrat is licensed under the MIT license.
