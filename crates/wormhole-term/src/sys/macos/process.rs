//! What libproc says of a process. Called every couple of seconds per terminal, so each is one or two
//! calls with a checked buffer, and nothing is spawned.

use std::ffi::c_void;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub(crate) use super::ports::listening_tcp_ports;
pub(crate) use super::procargs::{command_line, environment_names};

/// `PROC_PGRP_ONLY` from `<libproc.h>` (not exported by the libc crate).
const PROC_PGRP_ONLY: u32 = 2;
/// `PROC_PPID_ONLY` from `<libproc.h>` (not exported by the libc crate).
const PROC_PPID_ONLY: u32 = 6;

#[allow(unsafe_code)] // libproc FFI; buffer sizes are checked below.
pub(crate) fn cwd(pid: u32) -> Option<PathBuf> {
    // SAFETY: `proc_vnodepathinfo` is a plain C struct of integers and byte arrays, for which all-zero
    // bytes is a valid value.
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

#[allow(unsafe_code)] // libproc FFI; every buffer size is checked.
fn bsd_info(pid: u32) -> Option<libc::proc_bsdinfo> {
    // SAFETY: proc_bsdinfo is plain old data; all-zero is a valid value.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = size_of::<libc::proc_bsdinfo>() as i32;
    // SAFETY: `info` is a correctly sized, writable buffer for this flavor.
    let written = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    (written == size).then_some(info)
}

/// The bytes of memory the process has resident (`pti_resident_size`).
#[allow(unsafe_code)] // libproc FFI; the buffer size is checked.
pub(crate) fn resident_bytes(pid: u32) -> Option<u64> {
    // SAFETY: `proc_taskinfo` is plain old data; all-zero is a valid value.
    let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
    let size = size_of::<libc::proc_taskinfo>() as i32;
    // SAFETY: `info` is a correctly sized, writable buffer for this flavor.
    let written = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTASKINFO,
            0,
            (&mut info as *mut libc::proc_taskinfo).cast(),
            size,
        )
    };
    (written == size).then_some(info.pti_resident_size)
}

/// The name the kernel keeps for the process (`pbi_name`, else `pbi_comm`): what it was started as,
/// still there after the file it runs from was replaced or deleted (an update does that to a host
/// that outlives the app).
pub(crate) fn command_name(pid: u32) -> Option<String> {
    let info = bsd_info(pid)?;
    let text = |raw: &[libc::c_char]| {
        let bytes: Vec<u8> = raw.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    };
    let name = text(&info.pbi_name);
    Some(if name.is_empty() { text(&info.pbi_comm) } else { name })
}

/// The effective user id the process runs as.
pub(crate) fn owner(pid: u32) -> Option<u32> {
    bsd_info(pid).map(|info| info.pbi_uid)
}

/// The process group that owns the terminal of the process `shell_pid` right now.
pub(crate) fn foreground_group(shell_pid: u32) -> Option<u32> {
    bsd_info(shell_pid).map(|info| info.e_tpgid).filter(|&pgid| pgid > 0)
}

pub(crate) fn parent(pid: u32) -> Option<u32> {
    bsd_info(pid).map(|info| info.pbi_ppid)
}

/// When the process started.
pub(crate) fn start_time(pid: u32) -> Option<SystemTime> {
    let info = bsd_info(pid)?;
    let since = Duration::from_secs(info.pbi_start_tvsec) + Duration::from_micros(info.pbi_start_tvusec);
    SystemTime::UNIX_EPOCH.checked_add(since)
}

/// [`start_time`] for a process of another user too (a setuid-root program such as `top`), which
/// `proc_pidinfo` refuses and the kernel's process table still answers. Kept apart: who a peer is and
/// whether it is the same process (`start_time`) is decided only from what the system shows of one's own.
pub(crate) fn start_time_any_user(pid: u32) -> Option<SystemTime> {
    let since = kernel_start_time(pid)?;
    SystemTime::UNIX_EPOCH.checked_add(since)
}

/// When a process started since the epoch, from the kernel's process table (`kinfo_proc`), which any user may
/// read for any process. Its first field is a `timeval` (seconds as `i64`, then microseconds as `i32`): the
/// layout is the system's ABI, and the buffer is far larger than the struct, so only that prefix is read.
#[allow(unsafe_code)] // sysctl FFI; the buffer is sized and the kernel is told its size.
fn kernel_start_time(pid: u32) -> Option<Duration> {
    let pid = i32::try_from(pid).ok().filter(|p| *p > 0)?;
    let mut mib = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_PID, pid];
    let mut buf = [0u8; 2048];
    let mut size: libc::size_t = buf.len();
    // SAFETY: `buf` is `size` bytes and `size` tells the kernel so.
    let ok = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            buf.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if ok != 0 || size < 16 {
        return None;
    }
    let secs = i64::from_ne_bytes(buf[..8].try_into().ok()?);
    let micros = i32::from_ne_bytes(buf[8..12].try_into().ok()?);
    // A process that is gone answers with an empty (zeroed) record at times.
    (secs > 0).then(|| Duration::from_secs(secs as u64) + Duration::from_micros(micros.max(0) as u64))
}

/// The controlling terminal's device.
pub(crate) fn tty(pid: u32) -> Option<u32> {
    bsd_info(pid).map(|info| info.e_tdev).filter(|&dev| dev != 0 && dev != u32::MAX)
}

#[allow(unsafe_code)] // libproc FFI; every buffer size is checked.
pub(crate) fn group_members(pgid: u32) -> Vec<u32> {
    let mut pids = vec![0 as libc::c_int; 256];
    let bytes = (pids.len() * size_of::<libc::c_int>()) as libc::c_int;
    // SAFETY: the buffer holds `bytes` bytes; the call writes at most that much.
    let written =
        unsafe { libc::proc_listpids(PROC_PGRP_ONLY, pgid, pids.as_mut_ptr().cast::<c_void>(), bytes) };
    if written <= 0 {
        return Vec::new();
    }
    let count = (written as usize / size_of::<libc::c_int>()).min(pids.len());
    pids[..count].iter().filter(|&&pid| pid > 0).map(|&pid| pid as u32).collect()
}

#[allow(unsafe_code)] // libproc FFI; every buffer size is checked.
pub(crate) fn children(pid: u32) -> Vec<u32> {
    let mut pids = vec![0 as libc::c_int; 128];
    let bytes = (pids.len() * size_of::<libc::c_int>()) as libc::c_int;
    // SAFETY: the buffer holds `bytes` bytes; the call writes at most that much.
    let written =
        unsafe { libc::proc_listpids(PROC_PPID_ONLY, pid, pids.as_mut_ptr().cast::<c_void>(), bytes) };
    if written <= 0 {
        return Vec::new();
    }
    let count = (written as usize / size_of::<libc::c_int>()).min(pids.len());
    pids[..count].iter().filter(|&&child| child > 0).map(|&child| child as u32).collect()
}

/// The path of the process's executable.
#[allow(unsafe_code)] // libproc FFI; the buffer's size is passed along with it.
pub(crate) fn executable(pid: u32) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt as _;
    let pid = i32::try_from(pid).ok().filter(|p| *p > 0)?;
    // PROC_PIDPATHINFO_MAXSIZE is 4 * MAXPATHLEN.
    let mut buf = vec![0u8; 4 * libc::PATH_MAX as usize];
    // SAFETY: `buf` is writable for the length given; proc_pidpath writes at most that many bytes.
    let len = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    let len = usize::try_from(len).ok().filter(|len| *len > 0)?;
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(&buf[..len.min(buf.len())])))
}
