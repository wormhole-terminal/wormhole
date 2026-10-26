//! The time of day from the C library's time zone.

/// Hour, minute and second at `unix_seconds` in the system's time zone.
#[allow(unsafe_code)] // libc FFI: localtime_r fills a local `tm` from a local `time_t`.
pub(crate) fn local_hms(unix_seconds: u64) -> Option<(u8, u8, u8)> {
    let seconds = libc::time_t::try_from(unix_seconds).ok()?;
    // SAFETY: an all-zero `tm` is a valid value (integers and a null zone pointer).
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are to locals that live for the call; localtime_r is reentrant.
    let filled = unsafe { libc::localtime_r(&seconds, &mut tm) };
    if filled.is_null() {
        return None;
    }
    Some((u8::try_from(tm.tm_hour).ok()?, u8::try_from(tm.tm_min).ok()?, u8::try_from(tm.tm_sec).ok()?))
}
