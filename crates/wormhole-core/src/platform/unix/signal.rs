//! The SIGTERM handler behind `quit_signal`: it only writes a byte to a pipe, which a thread waits on.

use std::io;
use std::sync::atomic::{AtomicI32, Ordering};

/// The self-pipe: the handler writes a byte to `WAKE`, the thread in [`wait`] reads it from `SEEN`.
/// A pipe, because a write is the one thing a handler may do that wakes another thread.
static SEEN: AtomicI32 = AtomicI32::new(-1);
static WAKE: AtomicI32 = AtomicI32::new(-1);

/// Where this thread's `errno` lives.
#[allow(unsafe_code)]
unsafe fn errno_location() -> *mut libc::c_int {
    // SAFETY: both only return the address of the calling thread's `errno`.
    #[cfg(target_os = "macos")]
    unsafe {
        libc::__error()
    }
    #[cfg(not(target_os = "macos"))]
    unsafe {
        libc::__errno_location()
    }
}

extern "C" fn on_sigterm(_: libc::c_int) {
    let fd = WAKE.load(Ordering::Relaxed);
    // SAFETY: `write` is async-signal-safe; `fd` is the pipe's write end, which stays open for the
    // life of the process. A full pipe (EAGAIN) or a failure just drops the byte: one waiting is
    // enough, and `errno` is saved and restored so the interrupted code never sees the change.
    #[allow(unsafe_code)]
    unsafe {
        let errno = errno_location();
        let saved = *errno;
        libc::write(fd, [1u8].as_ptr().cast(), 1);
        *errno = saved;
    }
}

/// Open the pipe, close-on-exec on both ends (a started program must not hold it) and non-blocking on
/// the write end (the handler must never wait).
fn open_pipe() -> io::Result<()> {
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: `fds` has room for the two descriptors `pipe` fills in; `fcntl` gets descriptors it just made.
    #[allow(unsafe_code)]
    let status = unsafe {
        if libc::pipe(fds.as_mut_ptr()) != 0 {
            -1
        } else {
            let mut status = 0;
            for fd in fds {
                status |= libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
            }
            status | libc::fcntl(fds[1], libc::F_SETFL, libc::O_NONBLOCK)
        }
    };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    SEEN.store(fds[0], Ordering::Relaxed);
    WAKE.store(fds[1], Ordering::Relaxed);
    Ok(())
}

/// Catch SIGTERM from now on. Programs the app starts don't inherit the handler: `exec` puts a
/// caught signal back to its default.
pub(crate) fn catch_sigterm() -> io::Result<()> {
    if WAKE.load(Ordering::Relaxed) < 0 {
        open_pipe()?;
    }
    // SAFETY: `act` is fully initialised (zeroed, then an empty mask, the handler and flags) before
    // `sigaction` reads it; the handler is an `extern "C"` function that only calls `write`.
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

extern "C" fn on_sigint(_: libc::c_int) {
    // SAFETY: `_exit` is async-signal-safe: it ends the process without running anything.
    #[allow(unsafe_code)]
    unsafe {
        libc::_exit(0)
    }
}

/// Make SIGINT end the process with exit code 0.
pub(crate) fn exit_on_interrupt() -> io::Result<()> {
    // SAFETY: as for `catch_sigterm`: `act` is initialised before `sigaction` reads it, and the
    // handler only calls `_exit`.
    #[allow(unsafe_code)]
    let status = unsafe {
        let mut act: libc::sigaction = std::mem::zeroed();
        act.sa_sigaction = on_sigint as extern "C" fn(libc::c_int) as libc::sighandler_t;
        libc::sigemptyset(&mut act.sa_mask);
        libc::sigaction(libc::SIGINT, &act, std::ptr::null_mut())
    };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Block until SIGTERM has arrived (it may have before the call), then return. Errors when
/// [`catch_sigterm`] was not called or the pipe broke.
pub(crate) fn wait() -> io::Result<()> {
    let fd = SEEN.load(Ordering::Relaxed);
    if fd < 0 {
        return Err(io::Error::new(io::ErrorKind::NotConnected, "SIGTERM is not caught"));
    }
    let mut byte = 0u8;
    loop {
        // SAFETY: reads one byte into a local the length of the read; `fd` is the pipe's read end.
        #[allow(unsafe_code)]
        let n = unsafe { libc::read(fd, (&mut byte as *mut u8).cast(), 1) };
        match n {
            1 => return Ok(()),
            0 => return Err(io::ErrorKind::UnexpectedEof.into()),
            _ => {
                let err = io::Error::last_os_error();
                if err.kind() != io::ErrorKind::Interrupted {
                    return Err(err);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{catch_sigterm, wait};

    #[test]
    fn a_sigterm_wakes_the_waiter_and_is_not_the_end() {
        catch_sigterm().expect("installs");
        let waiter = std::thread::spawn(wait);
        // SAFETY: sends SIGTERM to this process, whose handler was just installed.
        #[allow(unsafe_code)]
        let sent = unsafe { libc::raise(libc::SIGTERM) };
        assert_eq!(sent, 0);
        waiter.join().expect("thread ends").expect("woken by the signal");
    }
}
