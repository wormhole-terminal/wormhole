//! Apple's code-signature reading: the Security framework and the socket's audit token.

use std::ffi::c_void;
use std::io;
use std::os::fd::{AsRawFd as _, RawFd};
use std::str::FromStr as _;

use crate::sys::socket::Stream;
use core_foundation::base::{CFType, TCFType as _};
use core_foundation::data::CFData;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};
use security_framework::os::macos::code_signing::{
    Flags, GuestAttributes, SecCode, SecRequirement, SecStaticCode,
};
use security_framework_sys::code_signing::{SecCSFlags, SecCodeRef, SecStaticCodeRef};

use crate::sys::codesign::{Refusal, Stage};

/// `kSecCSSigningInformation`: also read the signature's details, the team id among them.
const SIGNING_INFORMATION: SecCSFlags = 1 << 1;

/// An `audit_token_t`: eight 32-bit values, the pid the sixth.
const TOKEN_WORDS: usize = 8;
const TOKEN_PID: usize = 5;

// Not in `security-framework-sys`. The framework is linked already (by that crate).
#[allow(non_upper_case_globals, unsafe_code)] // declarations only; each call has its SAFETY note.
unsafe extern "C" {
    static kSecCodeInfoTeamIdentifier: CFStringRef;

    fn SecCodeCopyStaticCode(code: SecCodeRef, flags: SecCSFlags, out: *mut SecStaticCodeRef) -> i32;

    fn SecCodeCopySigningInformation(
        code: SecStaticCodeRef,
        flags: SecCSFlags,
        out: *mut CFDictionaryRef,
    ) -> i32;
}

/// A requirement a peer's signature must satisfy, compiled.
pub struct Requirement(SecRequirement);

/// A requirement compiled once from its text; `None` (said in the log) when it does not parse.
pub(crate) fn requirement(text: &str) -> Option<Requirement> {
    match SecRequirement::from_str(text) {
        Ok(requirement) => Some(Requirement(requirement)),
        Err(err) => {
            tracing::warn!(status = err.code(), "peer check: the requirement does not compile");
            None
        }
    }
}

/// Whether this process satisfies `requirement`.
pub(crate) fn self_satisfies(requirement: &Requirement) -> bool {
    SecCode::for_self(Flags::NONE).and_then(|own| own.check_validity(Flags::NONE, &requirement.0)).is_ok()
}

/// The team id in this process's own signature, if it has one.
#[allow(unsafe_code)] // Security framework FFI around two calls the crate does not wrap.
pub(crate) fn own_team_id() -> Option<String> {
    let code = SecCode::for_self(Flags::NONE).ok()?;
    let mut static_code: SecStaticCodeRef = std::ptr::null_mut();
    // SAFETY: `code` is a live SecCode; the call writes one owned reference (or nothing, on
    // failure) into a local.
    let status = unsafe { SecCodeCopyStaticCode(code.as_concrete_TypeRef(), 0, &mut static_code) };
    if status != 0 || static_code.is_null() {
        return None;
    }
    // SAFETY: the reference was returned by a Copy function, so this takes the one retain.
    let static_code = unsafe { SecStaticCode::wrap_under_create_rule(static_code) };
    let mut info: CFDictionaryRef = std::ptr::null();
    // SAFETY: as above; the call writes one owned dictionary (or nothing) into a local.
    let status = unsafe {
        SecCodeCopySigningInformation(static_code.as_concrete_TypeRef(), SIGNING_INFORMATION, &mut info)
    };
    if status != 0 || info.is_null() {
        return None;
    }
    // SAFETY: a Copy function's dictionary, taken once; its keys are CFStrings and its values
    // CFTypes, which is how it is read.
    let info: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_create_rule(info) };
    // SAFETY: the key is a constant CFString that lives as long as the framework.
    let key = unsafe { CFString::wrap_under_get_rule(kSecCodeInfoTeamIdentifier) };
    // An ad-hoc signature has no team id: the key is missing.
    info.find(&key)?.downcast::<CFString>().map(|team| team.to_string())
}

/// The peer's audit token: who is on the other end of the connection, in a form that cannot
/// be spoofed by a pid that is reused.
#[allow(unsafe_code)] // libc FFI; the descriptor is open for the call, the output is a local.
fn audit_token(fd: RawFd) -> io::Result<[u32; TOKEN_WORDS]> {
    let mut token = [0u32; TOKEN_WORDS];
    let mut len = size_of_val(&token) as libc::socklen_t;
    // SAFETY: getsockopt writes at most `len` bytes into `token`, which is that long.
    let status = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_LOCAL,
            libc::LOCAL_PEERTOKEN,
            token.as_mut_ptr().cast::<c_void>(),
            &mut len,
        )
    };
    if status == 0 && len as usize == size_of_val(&token) {
        Ok(token)
    } else if status == 0 {
        Err(io::Error::from(io::ErrorKind::InvalidData))
    } else {
        Err(io::Error::last_os_error())
    }
}

/// The pid of the peer, for a log line when there is no token.
#[allow(unsafe_code)] // libc FFI; the descriptor is open for the call, the output is a local.
fn peer_pid(fd: RawFd) -> Option<u32> {
    let mut pid: libc::pid_t = 0;
    let mut len = size_of_val(&pid) as libc::socklen_t;
    // SAFETY: getsockopt writes at most `len` bytes into `pid`, which is that long.
    let status = unsafe {
        libc::getsockopt(fd, libc::SOL_LOCAL, libc::LOCAL_PEERPID, (&raw mut pid).cast::<c_void>(), &mut len)
    };
    (status == 0).then_some(pid as u32)
}

/// Find the peer's code from its audit token and check it against `requirement`.
pub(crate) fn check_peer(stream: &Stream, requirement: &Requirement) -> Result<(), Refusal> {
    let fd = stream.as_raw_fd();
    let token = audit_token(fd).map_err(|err| Refusal {
        stage: Stage::Token,
        pid: peer_pid(fd),
        status: err.raw_os_error().unwrap_or(0),
    })?;
    let pid = Some(token[TOKEN_PID]);
    let bytes: Vec<u8> = token.iter().flat_map(|word| word.to_ne_bytes()).collect();
    let data = CFData::from_buffer(&bytes);
    let mut attributes = GuestAttributes::new();
    attributes.set_audit_token(data.as_concrete_TypeRef());
    let code = SecCode::copy_guest_with_attribues(None, &attributes, Flags::NONE).map_err(|err| Refusal {
        stage: Stage::Code,
        pid,
        status: err.code(),
    })?;
    code.check_validity(Flags::NONE, &requirement.0).map_err(|err| Refusal {
        stage: Stage::Requirement,
        pid,
        status: err.code(),
    })
}
