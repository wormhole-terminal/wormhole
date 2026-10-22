//! A polite `kill`: SIGTERM asks the app to quit as ⌘Q does (every window saved, session hosts
//! left running) instead of ending it on the spot, which lost the terminals' text since the last
//! minute's save. The handler only sets a flag; the app's tick reads it with [`take`].

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

static ASKED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigterm(_: libc::c_int) {
    // Async-signal-safe: a store to a lock-free atomic and nothing else.
    ASKED.store(true, Ordering::SeqCst);
}

/// Catch SIGTERM from now on. Programs the app starts don't inherit the handler: `exec` puts a
/// caught signal back to its default.
pub fn catch_sigterm() -> io::Result<()> {
    // SAFETY: `act` is fully initialised (zeroed, then an empty mask, the handler and flags) before
    // `sigaction` reads it; the handler is an `extern "C"` function that only touches an atomic.
    // `SA_RESTART` keeps other threads' system calls from failing with EINTR.
    #[allow(unsafe_code)]
    let status = unsafe {
        let mut act: libc::sigaction = std::mem::zeroed();
        act.sa_sigaction = on_sigterm as extern "C" fn(libc::c_int) as libc::sighandler_t;
        libc::sigemptyset(&mut act.sa_mask);
        act.sa_flags = libc::SA_RESTART;
        libc::sigaction(libc::SIGTERM, &act, std::ptr::null_mut())
    };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Whether SIGTERM arrived since the last call.
pub fn take() -> bool {
    ASKED.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::{catch_sigterm, take};

    #[test]
    fn a_sigterm_is_a_request_to_quit_not_the_end() {
        catch_sigterm().expect("installs");
        assert!(!take());
        // SAFETY: sends SIGTERM to this process, whose handler was just installed.
        #[allow(unsafe_code)]
        let sent = unsafe { libc::raise(libc::SIGTERM) };
        assert_eq!(sent, 0);
        assert!(take(), "the tick sees the request");
        assert!(!take(), "once");
    }
}
