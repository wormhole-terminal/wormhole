use crate::sys::signals::Ending;

/// The signals the host blocks: the two that end it, and the hangup it ignores.
const BLOCKED: [libc::c_int; 3] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP];

/// A signal set of the blocked ones.
#[allow(unsafe_code)] // libc FFI on a local signal set.
fn blocked_set() -> libc::sigset_t {
    // SAFETY: the set is local and initialized by sigemptyset before use.
    unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        for signal in BLOCKED {
            libc::sigaddset(&mut set, signal);
        }
        set
    }
}

/// Block the ending signals in this thread and every thread it starts afterwards, so only
/// [`wait_ending_signal`] receives them (the program's process resets its mask before it runs).
#[allow(unsafe_code)] // libc FFI on a local signal set.
pub(crate) fn block_ending_signals() {
    let set = blocked_set();
    // SAFETY: the set is initialized; a null old set is allowed.
    unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut()) };
}

/// Wait for one of the blocked signals; the ending it asks for, or `None` for the hangup (which the
/// host ignores) and for a wait that failed.
#[allow(unsafe_code)] // libc FFI on a local signal set.
pub(crate) fn wait_ending_signal() -> Option<Ending> {
    let set = blocked_set();
    let mut got = 0;
    // SAFETY: as in `block_ending_signals`; sigwait writes the signal into a local.
    if unsafe { libc::sigwait(&set, &mut got) } != 0 {
        return None;
    }
    match got {
        libc::SIGTERM => Some(Ending::Terminate),
        libc::SIGINT => Some(Ending::Interrupt),
        _ => None,
    }
}
