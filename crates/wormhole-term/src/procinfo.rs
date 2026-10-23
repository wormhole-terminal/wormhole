//! Whether a host name is this machine's (the check a shell's reported folder goes through).

use crate::sys;

/// Whether `host`, as a shell names it in OSC 7 (`file://host/path`), is this machine: no host,
/// `localhost`, the loopback addresses, or this machine's host name. Short names are compared,
/// case-insensitively: macOS reports `Studio.local` where a shell may say `studio`.
pub(super) fn is_this_host(host: &str) -> bool {
    if host.is_empty() || host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1" {
        return true;
    }
    let short = |name: &str| name.split('.').next().unwrap_or("").to_ascii_lowercase();
    sys::host::name().is_some_and(|ours| short(ours) == short(host))
}

#[cfg(test)]
mod tests {
    use crate::sys;

    #[test]
    fn a_folder_counts_as_here_when_its_host_is_this_machine() {
        for local in ["", "localhost", "LOCALHOST", "127.0.0.1", "::1"] {
            assert!(super::is_this_host(local), "{local:?}");
        }
        assert!(!super::is_this_host("build-server.example.com"));
        assert!(!super::is_this_host("elsewhere"));
        if let Some(ours) = sys::host::name() {
            assert!(super::is_this_host(ours));
            assert!(super::is_this_host(&ours.to_ascii_uppercase()));
            let short = ours.split('.').next().unwrap_or(ours);
            assert!(super::is_this_host(&format!("{short}.local")), "short names match");
        }
    }
}
