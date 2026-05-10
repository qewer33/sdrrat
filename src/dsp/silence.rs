//! Briefly redirect process-wide stderr to `/dev/null` while a noisy native
//! call runs. Used to suppress libSoapySDR / librtlsdr `fprintf(stderr, ...)`
//! output that otherwise corrupts ratatui's screen.
//!
//! IMPORTANT: file descriptors are process-wide on Linux. Touching fd 1
//! (stdout) would also break ratatui's rendering on the main thread. This
//! helper only touches fd 2 (stderr), which ratatui doesn't use, so racing
//! with the UI thread is safe.

use std::fs::File;
use std::os::unix::io::AsRawFd;

/// RAII guard that restores stderr on drop.
pub(super) struct SilencedStderr {
    backup: i32,
    /// Held to keep `/dev/null` open for the lifetime of the redirect.
    _devnull: File,
}

impl SilencedStderr {
    pub(super) fn new() -> Self {
        unsafe {
            let devnull = File::open("/dev/null").expect("/dev/null missing");
            let null_fd = devnull.as_raw_fd();
            let backup = libc::dup(2);
            libc::dup2(null_fd, 2);
            Self { backup, _devnull: devnull }
        }
    }
}

impl Drop for SilencedStderr {
    fn drop(&mut self) {
        unsafe {
            libc::dup2(self.backup, 2);
            libc::close(self.backup);
        }
    }
}

/// Run `f` with stderr redirected to /dev/null. Restores on return or panic.
pub(super) fn silenced<R>(f: impl FnOnce() -> R) -> R {
    let _g = SilencedStderr::new();
    f()
}
