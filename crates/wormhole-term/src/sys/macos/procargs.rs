//! A process's executable and arguments, from the kernel's `KERN_PROCARGS2` buffer.

/// Executable path and argv, from `KERN_PROCARGS2`
/// (`argc`, exec path, padding NULs, then the arguments).
pub(crate) fn command_line(pid: u32) -> Option<(String, Vec<String>)> {
    parse_procargs(&read(pid)?)
}

/// The names (not the values) of the variables in a process's environment, as it was started with.
pub(crate) fn environment_names(pid: u32) -> Option<Vec<String>> {
    parse_environment(&read(pid)?)
}

#[allow(unsafe_code)] // sysctl FFI; the buffer is sized by the kernel's own answer.
fn read(pid: u32) -> Option<Vec<u8>> {
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as libc::c_int];
    let mut size: libc::size_t = 0;
    // SAFETY: a NULL buffer only asks for the required size.
    let ok = unsafe {
        libc::sysctl(mib.as_mut_ptr(), 3, std::ptr::null_mut(), &mut size, std::ptr::null_mut(), 0)
    };
    if ok != 0 || size == 0 {
        return None;
    }
    let mut buf = vec![0u8; size];
    // SAFETY: `buf` is `size` bytes long and `size` tells the kernel so.
    let ok = unsafe {
        libc::sysctl(mib.as_mut_ptr(), 3, buf.as_mut_ptr().cast(), &mut size, std::ptr::null_mut(), 0)
    };
    if ok != 0 {
        return None;
    }
    buf.truncate(size);
    Some(buf)
}

/// The variable names of a `KERN_PROCARGS2` buffer: after the exec path and its padding come `argc`
/// strings (empty ones included, so a trick with an empty argument moves nothing) and then `NAME=value`.
fn parse_environment(buf: &[u8]) -> Option<Vec<String>> {
    let argc = usize::try_from(i32::from_ne_bytes(buf.get(..4)?.try_into().ok()?)).ok()?;
    let rest = &buf[4..];
    let exe_end = rest.iter().position(|&b| b == 0)?;
    let mut at = exe_end;
    while rest.get(at) == Some(&0) {
        at += 1;
    }
    let mut parts = rest[at..].split(|&b| b == 0);
    for _ in 0..argc {
        parts.next()?;
    }
    Some(
        parts
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                let name = entry.split(|&b| b == b'=').next().unwrap_or_default();
                String::from_utf8_lossy(name).into_owned()
            })
            .collect(),
    )
}

/// Split a `KERN_PROCARGS2` buffer into the exec path and argv.
fn parse_procargs(buf: &[u8]) -> Option<(String, Vec<String>)> {
    let argc = i32::from_ne_bytes(buf.get(..4)?.try_into().ok()?);
    let rest = &buf[4..];
    let exe_end = rest.iter().position(|&b| b == 0)?;
    let exe = String::from_utf8_lossy(&rest[..exe_end]).into_owned();
    let mut args = rest[exe_end..].split(|&b| b == 0).filter(|s| !s.is_empty());
    let argv =
        (0..argc.clamp(0, 64)).map_while(|_| args.next()).map(|a| String::from_utf8_lossy(a).into_owned());
    Some((exe, argv.collect()))
}

#[cfg(test)]
mod tests {
    use super::parse_procargs;

    #[test]
    fn the_environment_names_follow_the_arguments_even_when_one_is_empty() {
        let mut buf = 3i32.to_ne_bytes().to_vec();
        buf.extend_from_slice(
            b"/usr/bin/node\0\0\0node\0\0/x/cli.js\0HOME=/u\0NODE_OPTIONS=--require=/tmp/x\0",
        );
        let names = super::parse_environment(&buf).expect("parses");
        assert_eq!(names, ["HOME", "NODE_OPTIONS"]);
        assert!(super::parse_environment(&[1, 0]).is_none());
    }

    #[test]
    fn procargs_are_parsed() {
        let mut buf = 2i32.to_ne_bytes().to_vec();
        buf.extend_from_slice(b"/usr/bin/node\0\0\0\0node\0/x/gemini\0HOME=/Users/me\0");
        let (exe, args) = parse_procargs(&buf).expect("parses");
        assert_eq!(exe, "/usr/bin/node");
        assert_eq!(args, ["node", "/x/gemini"]);
        assert!(parse_procargs(&[1, 0]).is_none());
    }
}
