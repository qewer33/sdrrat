mod app;
mod dsp;
mod ui;

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

use app::{App, SourceField};
use dsp::{DeviceKind, DspCommand};

/// Owns the DSP thread and the channels it uses. A new instance of these is
/// created on every (re)connect attempt.
struct DspThread {
    handle: JoinHandle<()>,
    quit: Arc<AtomicBool>,
}

/// Manages the DSP thread lifecycle. Survives across reconnects.
struct Supervisor {
    test_mode: bool,
    thread: Option<DspThread>,
}

impl Supervisor {
    fn new(test_mode: bool) -> Self {
        Self { test_mode, thread: None }
    }

    /// Try to (re)open the requested SDR device and spawn the DSP thread.
    /// On success returns fresh channel ends for the App to use.
    fn connect(
        &mut self,
        kind: DeviceKind,
        sample_rate: u32,
        initial_freq: u64,
    ) -> Result<(Receiver<Vec<f32>>, Sender<DspCommand>), String> {
        // Tear down any existing thread so we release the device first.
        self.disconnect();

        let dev = if self.test_mode {
            None
        } else {
            Some(dsp::open_device(kind)?)
        };

        let (tx, rx) = bounded::<Vec<f32>>(4);
        let (cmd_tx, cmd_rx) = bounded::<DspCommand>(16);
        let quit = Arc::new(AtomicBool::new(false));

        let handle = if let Some(dev) = dev {
            dsp::spawn_dsp_thread(dev, sample_rate, initial_freq, tx, cmd_rx, Arc::clone(&quit))
        } else {
            dsp::spawn_mock_dsp_thread(tx, cmd_rx, Arc::clone(&quit))
        };

        self.thread = Some(DspThread { handle, quit });
        Ok((rx, cmd_tx))
    }

    /// Stop the DSP thread (if running) and wait for it to finish so the
    /// hardware is fully released before the next connect attempt.
    fn disconnect(&mut self) {
        if let Some(t) = self.thread.take() {
            t.quit.store(true, Ordering::Relaxed);
            let _ = t.handle.join();
        }
    }
}

fn main() -> io::Result<()> {
    // Silence FutureSDR's tracing logs (debug/info) so they don't corrupt the
    // ratatui display.  Users can override via FUTURESDR_LOG=info etc.
    if std::env::var_os("FUTURESDR_LOG").is_none() {
        // SAFETY: single-threaded at this point (before any thread spawn).
        unsafe { std::env::set_var("FUTURESDR_LOG", "off") };
    }

    let test_mode = std::env::args().any(|a| a == "--test");
    let mut supervisor = Supervisor::new(test_mode);
    let mut app = App::new();
    app.apply_persisted(app::load_persisted());

    // Try the initial connection BEFORE entering the TUI so SoapySDR's
    // startup prints land on the normal terminal.
    match supervisor.connect(app.device_kind, app.current_sample_rate(), app.center_freq) {
        Ok((rx, cmd_tx)) => app.set_connected(rx, cmd_tx),
        Err(e) => {
            // Failed — open Source popup automatically so the user sees it.
            app.set_disconnected(Some(e));
            app.source_focus = SourceField::Connect;
            app.mode = app::AppMode::EditingSource;
        }
    }

    let mut terminal = ratatui::init();
    let result = run_ui(&mut terminal, &mut app, &mut supervisor);
    ratatui::restore();

    // Persist config once we've returned to a normal terminal.
    app::save_persisted(&app);

    supervisor.disconnect();
    result
}

fn run_ui(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    supervisor: &mut Supervisor,
) -> io::Result<()> {
    let event_quit = Arc::new(AtomicBool::new(false));

    while app.running {
        app.poll_data();
        terminal.draw(|frame| ui::draw(frame, app))?;

        // Service disconnect requests (config went stale, e.g. device change).
        if app.take_disconnect_request() {
            supervisor.disconnect();
            app.set_disconnected(None);
        }

        // Service connect requests from the Source popup.
        if app.take_connect_request() {
            match supervisor.connect(app.device_kind, app.current_sample_rate(), app.center_freq) {
                Ok((rx, cmd_tx)) => app.set_connected(rx, cmd_tx),
                Err(e) => app.set_disconnected(Some(e)),
            }
            // Popup stays open — the user closes it explicitly with Esc.
        }

        if event::poll(Duration::from_millis(16))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key.code, &event_quit);
        }
    }

    Ok(())
}
