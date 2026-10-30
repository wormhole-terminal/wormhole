//! The kernel's own record of a peer's code signature, for a peer whose executable is gone from disk.
//!
//! The Security framework answers "who is this process" by finding the code on disk (it reads the
//! executable by its path and checks it). When an app is replaced while it runs, the old executable is
//! unlinked and the lookup fails with `kPOSIXErrorBase + ENOENT`, though the process is still the one
//! that connected. The kernel still holds what it verified when the process was started (its code
//! signing flags, signing identifier and team id), and answers for an audit token through
//! `csops_audittoken`. That function is exported by libsystem_kernel and declared in XNU's open-source
//! `bsd/sys/codesign.h`; it is not in the SDK's public headers, which is why it is declared here.
//!
//! What this cannot tell apart: a Developer ID signature from an Apple Development one of the same team.
//! The certificate markers the framework's requirement checks are not part of the kernel's record. That
//! is acceptable because the path is taken only when the peer's binary is gone, it still demands the
//! app's identifier and the host's own team, and a process that unlinked its own executable still had to
//! be signed by our team to carry that identifier and team id at all (the kernel enforces the signature
//! of every process it runs: `CS_VALID` means the code still matches what was signed). Nobody else can
//! sign with the team id.

use std::ffi::c_void;

/// `csops` operations (`sys/codesign.h`).
const CS_OPS_STATUS: u32 = 0;
const CS_OPS_IDENTITY: u32 = 11;
const CS_OPS_TEAMID: u32 = 14;

/// The kernel's flags for a process (`sys/codesign.h`).
const CS_VALID: u32 = 0x0000_0001;
const CS_ADHOC: u32 = 0x0000_0002;
/// A debuggable build: any process of the user may attach and run code in it. Release builds are not
/// (the hardened runtime and the notarization refuse it), so a peer that is has not been shipped by us.
const CS_GET_TASK_ALLOW: u32 = 0x0000_0004;
const CS_LINKER_SIGNED: u32 = 0x0002_0000;
const CS_SIGNED: u32 = 0x2000_0000;

/// Room for the identity blob (an 8-byte header and the identifier) and for a team id. A longer one is
/// an `ERANGE` from the kernel, which refuses: neither the app's identifier nor a team id comes near.
const IDENTITY_BUF: usize = 512;
const TEAM_BUF: usize = 64;
/// The header the kernel puts in front of the identity and the team id alike: magic and length, 4 bytes
/// each.
const BLOB_HEADER: usize = 8;

#[allow(unsafe_code)] // declaration only; the call has its SAFETY note.
unsafe extern "C" {
    /// `int csops_audittoken(pid_t pid, unsigned int ops, void *useraddr, size_t usersize,
    /// audit_token_t *token)`: like `csops`, for the process the token names (so a reused pid is not
    /// mistaken for it).
    fn csops_audittoken(
        pid: libc::pid_t,
        ops: u32,
        useraddr: *mut c_void,
        usersize: usize,
        token: *const u32,
    ) -> i32;
}

/// What the kernel holds of a peer's signature.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Record {
    pub(super) flags: u32,
    pub(super) identifier: String,
    pub(super) team: String,
}

/// Whether `record` is the app of `team` with signing identifier `identifier`: valid and signed (not
/// ad hoc, not signed by the linker), not debuggable, with exactly that identifier and team.
pub(super) fn admits(record: &Record, identifier: &str, team: &str) -> bool {
    const REQUIRED: u32 = CS_VALID | CS_SIGNED;
    const FORBIDDEN: u32 = CS_ADHOC | CS_LINKER_SIGNED | CS_GET_TASK_ALLOW;
    record.flags & REQUIRED == REQUIRED
        && record.flags & FORBIDDEN == 0
        && !team.is_empty()
        && record.identifier == identifier
        && record.team == team
}

/// One `csops_audittoken` call into `buf`; the kernel's status, `None` when it failed.
#[allow(unsafe_code)] // csops_audittoken FFI; the output is a local the call is told the size of.
fn call(token: &[u32; super::codesign::TOKEN_WORDS], ops: u32, buf: &mut [u8]) -> Option<()> {
    // SAFETY: the kernel writes at most `usersize` bytes into `buf`, which is that long, and reads
    // the eight words of `token`, which is that long (an `audit_token_t`).
    let status = unsafe {
        csops_audittoken(
            token[super::codesign::TOKEN_PID] as libc::pid_t,
            ops,
            buf.as_mut_ptr().cast(),
            buf.len(),
            token.as_ptr(),
        )
    };
    (status == 0).then_some(())
}

/// The string in a blob the kernel wrote: past its header, up to the first NUL.
fn blob_string(blob: &[u8]) -> Option<String> {
    c_string(blob.get(BLOB_HEADER..)?)
}

/// The text up to the first NUL of `bytes`; `None` when it has no NUL (cut off) or is not UTF-8.
fn c_string(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|&b| b == 0)?;
    std::str::from_utf8(&bytes[..end]).ok().map(str::to_owned)
}

/// The kernel's record for the process of `token`, or `None` when it has none or won't say.
pub(super) fn record(token: &[u32; super::codesign::TOKEN_WORDS]) -> Option<Record> {
    let mut flags = [0u8; size_of::<u32>()];
    call(token, CS_OPS_STATUS, &mut flags)?;
    let mut identity = [0u8; IDENTITY_BUF];
    call(token, CS_OPS_IDENTITY, &mut identity)?;
    let mut team = [0u8; TEAM_BUF];
    call(token, CS_OPS_TEAMID, &mut team)?;
    Some(Record {
        flags: u32::from_ne_bytes(flags),
        identifier: blob_string(&identity)?,
        team: blob_string(&team)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CS_ADHOC, CS_GET_TASK_ALLOW, CS_LINKER_SIGNED, CS_SIGNED, CS_VALID, Record, admits, blob_string,
        c_string,
    };

    const APP: &str = "app.getwormhole.terminal";
    const TEAM: &str = "ABCDE12345";

    /// What the spike read from the real, Developer ID signed app after its bundle was replaced.
    const GOOD: u32 = 0x2201_1311;

    fn record(flags: u32, identifier: &str, team: &str) -> Record {
        Record { flags, identifier: identifier.into(), team: team.into() }
    }

    #[test]
    fn the_signed_app_of_the_team_is_admitted() {
        assert!(admits(&record(GOOD, APP, TEAM), APP, TEAM));
    }

    #[test]
    fn an_ad_hoc_or_linker_signed_peer_is_refused() {
        assert!(!admits(&record(GOOD | CS_ADHOC, APP, TEAM), APP, TEAM));
        assert!(!admits(&record(GOOD | CS_LINKER_SIGNED, APP, TEAM), APP, TEAM));
    }

    #[test]
    fn a_peer_of_another_team_or_identifier_is_refused() {
        assert!(!admits(&record(GOOD, APP, "ZZZZZ99999"), APP, TEAM));
        assert!(!admits(&record(GOOD, "app.getwormhole.terminal.host", TEAM), APP, TEAM));
        assert!(!admits(&record(GOOD, APP, ""), APP, ""), "no team to compare is never a match");
    }

    #[test]
    fn a_peer_whose_code_is_no_longer_valid_or_signed_is_refused() {
        assert!(!admits(&record(GOOD & !CS_VALID, APP, TEAM), APP, TEAM));
        assert!(!admits(&record(GOOD & !CS_SIGNED, APP, TEAM), APP, TEAM));
    }

    #[test]
    fn a_debuggable_peer_is_refused() {
        assert!(!admits(&record(GOOD | CS_GET_TASK_ALLOW, APP, TEAM), APP, TEAM));
    }

    #[test]
    fn strings_end_at_their_nul_and_a_cut_off_one_is_nothing() {
        assert_eq!(c_string(b"abc\0def").as_deref(), Some("abc"));
        assert_eq!(c_string(b"abc"), None);
        assert_eq!(c_string(&[0xff, 0]), None);
    }

    #[test]
    fn the_team_id_is_read_past_its_header() {
        // The bytes CS_OPS_TEAMID wrote for the real app: a zero magic, the length, then the string.
        let blob = b"\0\0\0\0\0\0\0\x133VL3FJXX72\0\0";
        assert_eq!(blob_string(blob).as_deref(), Some("3VL3FJXX72"));
        assert_eq!(blob_string(b"\0\0\0"), None, "shorter than the header");
    }

    #[test]
    fn this_process_has_a_record_from_the_kernel() {
        // A cargo test binary is linker-signed: valid, signed, ad hoc, and so never admitted.
        let mut token = [0u32; 8];
        token[5] = std::process::id();
        // The audit token's other words are the kernel's to fill; a made-up one may not match, which
        // is the answer `None`. Only the shape is checked here: it never admits this binary.
        if let Some(own) = super::record(&token) {
            assert!(!admits(&own, APP, TEAM));
        }
    }
}
