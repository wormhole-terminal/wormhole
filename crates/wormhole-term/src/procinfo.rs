//! What the system says of a process or of this machine: a process's current folder (the polling
//! the shell's reported folder is checked against), and whether a host name is this machine's.

use std::path::PathBuf;

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // libproc FFI; buffer sizes are checked below.
pub(super) fn process_cwd(pid: u32) -> Option<PathBuf> {
    let mut info: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
    let size = size_of::<libc::proc_vnodepathinfo>() as i32;
    // SAFETY: `info` is a correctly sized, writable buffer for this flavor.
    let written = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            (&mut info as *mut libc::proc_vnodepathinfo).cast(),
            size,
        )
    };
    if written != size {
        return None;
    }
    // SAFETY: the kernel NUL-terminates `vip_path`.
    let path = unsafe { std::ffi::CStr::from_ptr(info.pvi_cdir.vip_path.as_ptr().cast()) };
    Some(PathBuf::from(path.to_string_lossy().into_owned()))
}

#[cfg(target_os = "linux")]
pub(super) fn process_cwd(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) fn process_cwd(_pid: u32) -> Option<PathBuf> {
    None
}

/// Whether `host`, as a shell names it in OSC 7 (`file://host/path`), is this machine: no host,
/// `localhost`, the loopback addresses, or this machine's host name. Short names are compared,
/// case-insensitively: macOS reports `Studio.local` where a shell may say `studio`.
pub(super) fn is_this_host(host: &str) -> bool {
    if host.is_empty() || host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1" {
        return true;
    }
    let short = |name: &str| name.split('.').next().unwrap_or("").to_ascii_lowercase();
    hostname().is_some_and(|ours| short(ours) == short(host))
}

/// This machine's host name, read once.
fn hostname() -> Option<&'static str> {
    static HOSTNAME: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    HOSTNAME.get_or_init(read_hostname).as_deref()
}

#[allow(unsafe_code)] // libc FFI; the buffer's length is passed along with it.
fn read_hostname() -> Option<String> {
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

#[cfg(test)]
mod tests {
    #[test]
    fn a_folder_counts_as_here_when_its_host_is_this_machine() {
        for local in ["", "localhost", "LOCALHOST", "127.0.0.1", "::1"] {
            assert!(super::is_this_host(local), "{local:?}");
        }
        assert!(!super::is_this_host("build-server.example.com"));
        assert!(!super::is_this_host("elsewhere"));
        if let Some(ours) = super::hostname() {
            assert!(super::is_this_host(ours));
            assert!(super::is_this_host(&ours.to_ascii_uppercase()));
            let short = ours.split('.').next().unwrap_or(ours);
            assert!(super::is_this_host(&format!("{short}.local")), "short names match");
        }
    }
}
