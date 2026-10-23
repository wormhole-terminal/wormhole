//! What libproc says of a process. Called every couple of seconds per terminal, so each is one or two
//! calls with a checked buffer, and nothing is spawned.

use std::ffi::c_void;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub(crate) use super::ports::listening_tcp_ports;
pub(crate) use super::procargs::command_line;

/// `PROC_PGRP_ONLY` from `<libproc.h>` (not exported by the libc crate).
const PROC_PGRP_ONLY: u32 = 2;
/// `PROC_PPID_ONLY` from `<libproc.h>` (not exported by the libc crate).
const PROC_PPID_ONLY: u32 = 6;

#[allow(unsafe_code)] // libproc FFI; buffer sizes are checked below.
pub(crate) fn cwd(pid: u32) -> Option<PathBuf> {
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
