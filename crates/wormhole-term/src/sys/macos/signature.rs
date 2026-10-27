//! Who signed the program behind a pid: the Security framework's signing information, read for a
//! running process by its pid (not an audit token: the agents asked about are ancestors of the
//! connecting process, and the caller checks that the pid still names the process it saw).

use std::fmt::Write as _;
use std::str::FromStr as _;

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType as _};
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::{CFString, CFStringRef};
use security_framework::certificate::SecCertificate;
use security_framework::os::macos::code_signing::{
    Flags, GuestAttributes, SecCode, SecRequirement, SecStaticCode,
};
use security_framework_sys::code_signing::{SecCSFlags, SecCodeRef, SecStaticCodeRef};

use crate::sys::codesign::{Signature, SignerKind};

/// `kSecCSSigningInformation`: also read the signature's details.
const SIGNING_INFORMATION: SecCSFlags = 1 << 1;
/// `errSecCSUnsigned`: the code has no signature at all.
const UNSIGNED: i32 = -67062;

/// Apple's own system software.
const APPLE: &str = "anchor apple";
/// The Developer ID Application chain (Apple's root, the Developer ID CA, the leaf): the same clauses the
/// session host holds its peers to.
const DEVELOPER_ID_CHAIN: &str = "anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] \
                                  and certificate leaf[field.1.2.840.113635.100.6.1.13]";

/// `csops` operations and the kernel's flags for the running process (`<sys/codesign.h>`): what the kernel
/// holds of the code that runs, not what the file on disk says now.
const CS_OPS_STATUS: u32 = 0;
const CS_OPS_CDHASH: u32 = 5;
/// `CS_ADHOC`.
const CS_ADHOC: u32 = 0x2;
/// `CS_LINKER_SIGNED`: signed ad hoc by the linker, as a `cargo build` is.
const CS_LINKER_SIGNED: u32 = 0x2_0000;
/// `CS_RUNTIME`: the hardened runtime, without which a library can be injected into the process.
const CS_RUNTIME: u32 = 0x1_0000;

// Not in `security-framework-sys`. The framework is linked already (by that crate).
#[allow(non_upper_case_globals, unsafe_code)] // declarations only; each use has its SAFETY note.
unsafe extern "C" {
    static kSecCodeInfoIdentifier: CFStringRef;
    static kSecCodeInfoTeamIdentifier: CFStringRef;
    static kSecCodeInfoCertificates: CFStringRef;

    fn csops(pid: libc::pid_t, ops: u32, useraddr: *mut std::ffi::c_void, usersize: usize) -> i32;

    fn SecCodeCopyStaticCode(code: SecCodeRef, flags: SecCSFlags, out: *mut SecStaticCodeRef) -> i32;

    fn SecCodeCopySigningInformation(
        code: SecStaticCodeRef,
        flags: SecCSFlags,
        out: *mut CFDictionaryRef,
    ) -> i32;
}

type Info = CFDictionary<CFString, CFType>;

/// The kernel's code-signing flags of the running process `pid`.
#[allow(unsafe_code)] // csops FFI; the output is a local the call is told the size of.
fn running_flags(pid: libc::pid_t) -> Option<u32> {
    let mut flags: u32 = 0;
    // SAFETY: csops writes at most `usersize` bytes into `flags`, which is that long.
    let status = unsafe { csops(pid, CS_OPS_STATUS, (&raw mut flags).cast(), size_of::<u32>()) };
    (status == 0).then_some(flags)
}

/// The code directory hash of the code that is running as `pid`, as the kernel holds it.
#[allow(unsafe_code)] // csops FFI; the output is a local the call is told the size of.
fn running_cdhash(pid: libc::pid_t) -> Option<String> {
    let mut hash = [0u8; 20];
    // SAFETY: csops writes at most `usersize` bytes into `hash`, which is that long.
    let status = unsafe { csops(pid, CS_OPS_CDHASH, hash.as_mut_ptr().cast(), hash.len()) };
    (status == 0).then(|| hex(&hash))
}

/// What a Developer ID program must satisfy to be the one its identifier and team name: the chain, the
/// identifier and the team in the leaf (the same shape the session host's peer check builds). `None` for an
/// identifier or team of a shape that must not be put into a requirement.
fn developer_requirement(identifier: &str, team: &str) -> Option<String> {
    let identifier_ok = !identifier.is_empty()
        && identifier.len() <= 128
        && identifier.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
    let team_ok = team.len() == 10 && team.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    (identifier_ok && team_ok).then(|| {
        format!("identifier \"{identifier}\" and {DEVELOPER_ID_CHAIN} and certificate leaf[subject.OU] = \"{team}\"")
    })
}

/// The signature of the running process `pid`; [`SignerKind::Unreadable`] when the system will not say. What
/// the program is comes from the running code (the kernel's hash and flags, and a check of the live process
/// against the identity the file claims): a file replaced after the program started changes nothing here.
pub(crate) fn of_pid(pid: u32) -> Signature {
    let Ok(pid) = libc::pid_t::try_from(pid) else { return Signature::of(SignerKind::Unreadable) };
    let mut attributes = GuestAttributes::new();
    attributes.set_pid(pid);
    let Ok(code) = SecCode::copy_guest_with_attribues(None, &attributes, Flags::NONE) else {
        return Signature::of(SignerKind::Unreadable);
    };
    let Some(flags) = running_flags(pid) else { return Signature::of(SignerKind::Unreadable) };
    let info = match signing_information(&code) {
        Ok(info) => info,
        Err(UNSIGNED) => return Signature::of(SignerKind::Unsigned),
        Err(_) => return Signature::of(SignerKind::Unreadable),
    };
    let identifier = string(&info, key_identifier);
    let team_id = string(&info, key_team);
    let cdhash = running_cdhash(pid);
    let kind = if flags & (CS_ADHOC | CS_LINKER_SIGNED) != 0 {
        SignerKind::AdHoc
    } else if satisfies(&code, APPLE) {
        SignerKind::AppleSystem
    } else if let (Some(id), Some(team)) = (&identifier, &team_id)
        && developer_requirement(id, team).is_some_and(|text| satisfies(&code, &text))
    {
        SignerKind::DeveloperId
    } else if team_id.is_none() {
        // No certificate at all: ad hoc, whatever the flags say.
        SignerKind::AdHoc
    } else {
        SignerKind::Other
    };
    let subject = (kind == SignerKind::DeveloperId).then(|| organisation(&info)).flatten();
    // A Developer ID program without the hardened runtime can have a library put into it, so its signer says
    // nothing about the code that runs: only its hash does.
    if kind == SignerKind::DeveloperId && flags & CS_RUNTIME == 0 {
        return Signature { kind: SignerKind::AdHoc, identifier, team_id, subject, cdhash, injectable: true };
    }
    Signature { kind, identifier, team_id, subject, cdhash, injectable: false }
}

/// Whether the running `code` satisfies the requirement `text`.
fn satisfies(code: &SecCode, text: &str) -> bool {
    SecRequirement::from_str(text)
        .is_ok_and(|requirement| code.check_validity(Flags::NONE, &requirement).is_ok())
}

/// The signing information of `code`, or the system's status.
#[allow(unsafe_code)] // Security framework FFI around two calls the crate does not wrap.
fn signing_information(code: &SecCode) -> Result<Info, i32> {
    let mut static_code: SecStaticCodeRef = std::ptr::null_mut();
    // SAFETY: `code` is a live SecCode; the call writes one owned reference (or nothing, on failure)
    // into a local.
    let status = unsafe { SecCodeCopyStaticCode(code.as_concrete_TypeRef(), 0, &mut static_code) };
    if status != 0 || static_code.is_null() {
        return Err(if status == 0 { -1 } else { status });
    }
    // SAFETY: the reference was returned by a Copy function, so this takes the one retain.
    let static_code = unsafe { SecStaticCode::wrap_under_create_rule(static_code) };
    let mut info: CFDictionaryRef = std::ptr::null();
    // SAFETY: as above; the call writes one owned dictionary (or nothing) into a local.
    let status = unsafe {
        SecCodeCopySigningInformation(static_code.as_concrete_TypeRef(), SIGNING_INFORMATION, &mut info)
    };
    if status != 0 || info.is_null() {
        return Err(if status == 0 { -1 } else { status });
    }
    // SAFETY: a Copy function's dictionary, taken once; its keys are CFStrings and its values CFTypes,
    // which is how it is read.
    Ok(unsafe { CFDictionary::wrap_under_create_rule(info) })
}

// The framework's key constants, each read in a function of its own.
#[allow(unsafe_code)] // SAFETY (all five): a constant CFString that lives as long as the framework.
fn key_identifier() -> CFString {
    unsafe { CFString::wrap_under_get_rule(kSecCodeInfoIdentifier) }
}
#[allow(unsafe_code)]
fn key_team() -> CFString {
    unsafe { CFString::wrap_under_get_rule(kSecCodeInfoTeamIdentifier) }
}
#[allow(unsafe_code)]
fn key_certificates() -> CFString {
    unsafe { CFString::wrap_under_get_rule(kSecCodeInfoCertificates) }
}

fn string(info: &Info, key: fn() -> CFString) -> Option<String> {
    info.find(key())?.downcast::<CFString>().map(|text| text.to_string())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// The organisation a Developer ID certificate names: its subject summary reads
/// `Developer ID Application: Anthropic PBC (Q6L2SF6YDW)`.
fn organisation(info: &Info) -> Option<String> {
    let certificates = info.find(key_certificates())?.downcast::<CFArray>()?;
    let leaf = certificates.get(0)?;
    // SAFETY-free: the array holds SecCertificates; a value of another type is ignored by the type id.
    let leaf = leaf_certificate(&leaf)?;
    let summary = leaf.subject_summary();
    let name = summary.split_once(": ").map_or(summary.as_str(), |(_, rest)| rest);
    let name = name.rsplit_once(" (").map_or(name, |(organisation, _)| organisation);
    Some(name.to_string())
}

#[allow(unsafe_code)] // reads a CFArray element as the certificate the signing information says it is.
fn leaf_certificate(
    item: &core_foundation::base::ItemRef<'_, *const std::ffi::c_void>,
) -> Option<SecCertificate> {
    use core_foundation::base::CFGetTypeID;
    use security_framework_sys::base::SecCertificateRef;
    let pointer: *const std::ffi::c_void = **item;
    if pointer.is_null() {
        return None;
    }
    // SAFETY: a non-null CoreFoundation object out of an array that is alive; its type is checked
    // before it is wrapped, and the get rule takes its own retain.
    unsafe {
        (CFGetTypeID(pointer) == SecCertificate::type_id())
            .then(|| SecCertificate::wrap_under_get_rule(pointer as SecCertificateRef))
    }
}
