#[allow(unsafe_code)] // libc FFI; the buffer's length is passed along with it.
pub(crate) fn read_name() -> Option<String> {
    // Longer than any host name POSIX allows, and zeroed: a name that fills it has no NUL.
    let mut buf = [0u8; 256];
    // SAFETY: `buf` is writable for the length given; gethostname writes at most that much.
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if rc != 0 {
        return None;
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8(buf[..end].to_vec()).ok().filter(|name| !name.is_empty())
}
