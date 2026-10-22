//! Owner authentication (Touch ID, login password, Apple Watch) for trusting a project's
//! `.wormhole/` folder before running its commands.
//!
//! Uses `LAPolicy::DeviceOwnerAuthentication`, so the system dialog offers Touch ID when it is
//! enrolled, an unlocked paired Apple Watch when available, and always the Mac login password.
//!
//! Bundling notes: macOS needs no Info.plist key for Touch ID or password prompts
//! (`NSFaceIDUsageDescription` only matters for Face ID on iOS/visionOS). The reason string passed
//! to [`authenticate`] is what the dialog shows, prefixed by the app name. Unbundled dev builds work
//! as well; the dialog then names the executable. Hardened-runtime/sandboxed builds need no extra
//! entitlement for LocalAuthentication.

use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Why an authentication attempt did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Owner authentication cannot be performed here (no password set, no UI session, non-macOS, ...).
    Unavailable(String),
    /// The user, the system or the app dismissed the dialog.
    Cancelled,
    /// The user did not prove ownership (wrong fingerprint/password, lockout, ...).
    Failed(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::Unavailable(msg) => write!(f, "authentication unavailable: {msg}"),
            AuthError::Cancelled => f.write_str("authentication cancelled"),
            AuthError::Failed(msg) => write!(f, "authentication failed: {msg}"),
        }
    }
}

impl std::error::Error for AuthError {}

/// `LAErrorDomain`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const LA_ERROR_DOMAIN: &str = "com.apple.LocalAuthentication";

/// `LAError` codes (see `LAError.h`). Kept as plain integers so the mapping is testable anywhere.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod code {
    pub const AUTHENTICATION_FAILED: isize = -1;
    pub const USER_CANCEL: isize = -2;
    pub const USER_FALLBACK: isize = -3;
    pub const SYSTEM_CANCEL: isize = -4;
    pub const PASSCODE_NOT_SET: isize = -5;
    pub const BIOMETRY_NOT_AVAILABLE: isize = -6;
    pub const BIOMETRY_NOT_ENROLLED: isize = -7;
    pub const BIOMETRY_LOCKOUT: isize = -8;
    pub const APP_CANCEL: isize = -9;
    pub const INVALID_CONTEXT: isize = -10;
    pub const COMPANION_NOT_AVAILABLE: isize = -11;
    pub const BIOMETRY_NOT_PAIRED: isize = -12;
    pub const BIOMETRY_DISCONNECTED: isize = -13;
    pub const INVALID_DIMENSIONS: isize = -14;
    pub const NOT_INTERACTIVE: isize = -1004;
}

/// Maps an `NSError` (domain, code, localized description) to an [`AuthError`].
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn map_error(domain: &str, code: isize, message: String) -> AuthError {
    if domain != LA_ERROR_DOMAIN {
        return AuthError::Failed(format!("{domain} ({code}): {message}"));
    }
    match code {
        code::USER_CANCEL | code::SYSTEM_CANCEL | code::APP_CANCEL => AuthError::Cancelled,
        // With `DeviceOwnerAuthentication` the password fallback is handled by the system, so
        // `UserFallback` and biometric lockout only surface if the user could not complete it.
        code::AUTHENTICATION_FAILED | code::USER_FALLBACK | code::BIOMETRY_LOCKOUT => {
            AuthError::Failed(message)
        }
        code::PASSCODE_NOT_SET
        | code::BIOMETRY_NOT_AVAILABLE
        | code::BIOMETRY_NOT_ENROLLED
        | code::COMPANION_NOT_AVAILABLE
        | code::BIOMETRY_NOT_PAIRED
        | code::BIOMETRY_DISCONNECTED
        | code::INVALID_CONTEXT
        | code::INVALID_DIMENSIONS
        | code::NOT_INTERACTIVE => AuthError::Unavailable(message),
        _ => AuthError::Failed(format!("LAError {code}: {message}")),
    }
}

/// How long an availability answer is reused. Asking means creating an `LAContext` and a round
/// trip to the system, and the UI asks while rendering; enrolling Touch ID shows up after this.
const AVAILABILITY_TTL: Duration = Duration::from_secs(10);

static AVAILABLE: Mutex<Option<(Instant, bool)>> = Mutex::new(None);
static BIOMETRICS: Mutex<Option<(Instant, bool)>> = Mutex::new(None);

/// `slot`'s answer if it is younger than `ttl`, else `ask()`'s (remembered).
fn cached(slot: &Mutex<Option<(Instant, bool)>>, ttl: Duration, ask: impl FnOnce() -> bool) -> bool {
    let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match *slot {
        Some((at, answer)) if at.elapsed() < ttl => answer,
        _ => {
            let answer = ask();
            *slot = Some((Instant::now(), answer));
            answer
        }
    }
}

/// Whether any owner authentication (Touch ID / password / Watch) is possible.
pub fn available() -> bool {
    cached(&AVAILABLE, AVAILABILITY_TTL, || imp::can_evaluate(false).is_ok())
}

/// Whether Touch ID specifically is set up (to word the UI: "Approve with Touch ID" vs "…with your
/// password").
pub fn biometrics_available() -> bool {
    cached(&BIOMETRICS, AVAILABILITY_TTL, || imp::can_evaluate(true).is_ok())
}

/// Ask the user to authenticate. `reason` is shown in the system dialog ("Wormhole wants to trust
/// the project folder …"). The callback is invoked exactly once, on an arbitrary thread.
pub fn authenticate(reason: &str, callback: impl FnOnce(Result<(), AuthError>) + Send + 'static) {
    imp::authenticate(reason, Box::new(callback));
}

/// Like [`authenticate`], but delivers the single result through a channel to `.await` (so no
/// thread waits while the dialog is open).
pub fn authenticate_rx(reason: &str) -> smol::channel::Receiver<Result<(), AuthError>> {
    let (tx, rx) = smol::channel::bounded(1);
    let deliver = move |result| {
        // The receiver may have been dropped; nobody is waiting then.
        let _ = tx.try_send(result);
    };
    #[cfg(feature = "devctl")]
    if let Some(answer) = scripted_answer() {
        deliver(answer);
        return rx;
    }
    authenticate(reason, deliver);
    rx
}

/// devctl builds: `WORMHOLE_AUTH=allow|deny|cancel` answers instead of the system dialog, so
/// scripted UI checks can click "Trust" without a Touch ID prompt (which they must never show).
#[cfg(feature = "devctl")]
fn scripted_answer() -> Option<Result<(), AuthError>> {
    match std::env::var("WORMHOLE_AUTH").ok()?.as_str() {
        "allow" => Some(Ok(())),
        "deny" => Some(Err(AuthError::Failed("denied by WORMHOLE_AUTH".into()))),
        "cancel" => Some(Err(AuthError::Cancelled)),
        other => {
            tracing::warn!("WORMHOLE_AUTH={other:?} is not allow, deny or cancel; asking the system");
            None
        }
    }
}

type Callback = Box<dyn FnOnce(Result<(), AuthError>) + Send + 'static>;

#[cfg(all(target_os = "macos", not(test)))]
mod imp {
    use super::{AuthError, Callback, map_error};
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LAContext, LAPolicy};
    use std::sync::{Mutex, PoisonError};

    fn from_ns_error(error: &NSError) -> AuthError {
        map_error(&error.domain().to_string(), error.code(), error.localizedDescription().to_string())
    }

    fn policy(biometrics_only: bool) -> LAPolicy {
        if biometrics_only {
            LAPolicy::DeviceOwnerAuthenticationWithBiometrics
        } else {
            LAPolicy::DeviceOwnerAuthentication
        }
    }

    #[allow(unsafe_code)]
    fn new_context() -> Retained<LAContext> {
        // SAFETY: `+[LAContext new]` has no preconditions and may be called from any thread.
        unsafe { LAContext::new() }
    }

    #[allow(unsafe_code)]
    fn can_evaluate_with(context: &LAContext, biometrics_only: bool) -> Result<(), AuthError> {
        // SAFETY: valid context and a known policy constant; objc2 handles the out-error pointer.
        unsafe { context.canEvaluatePolicy_error(policy(biometrics_only)) }
            .map_err(|error| from_ns_error(&error))
    }

    pub fn can_evaluate(biometrics_only: bool) -> Result<(), AuthError> {
        can_evaluate_with(&new_context(), biometrics_only)
    }

    #[allow(unsafe_code)]
    pub fn authenticate(reason: &str, callback: Callback) {
        let context = new_context();
        if let Err(error) = can_evaluate_with(&context, false) {
            callback(Err(error));
            return;
        }

        let reason = NSString::from_str(reason);
        let callback = Mutex::new(Some(callback));
        // The block owns a strong reference so the context lives until the reply has fired (the
        // framework releases the block afterwards, dropping the context with it).
        let keep_alive = context.clone();
        let reply = RcBlock::new(move |success: Bool, error: *mut NSError| {
            let _context = &keep_alive;
            let result = if success.as_bool() {
                Ok(())
            } else {
                // SAFETY: on failure LocalAuthentication passes a valid (or null) NSError that
                // outlives this call.
                match unsafe { error.as_ref() } {
                    Some(error) => Err(from_ns_error(error)),
                    None => Err(AuthError::Failed("unknown error".into())),
                }
            };
            let callback = callback.lock().unwrap_or_else(PoisonError::into_inner).take();
            if let Some(callback) = callback {
                callback(result);
            }
        });

        // SAFETY: LocalAuthentication calls the reply once, on a private queue, so the block must be
        // safe to run and drop on another thread. It touches a `Mutex` (thread-safe) and holds a
        // `Retained<LAContext>` only to keep the context alive; objc2 doesn't mark `LAContext`
        // `Send`, but the block never calls a method on it, and dropping it there is an atomic
        // `release`, as the framework's own references to the context are released there too.
        unsafe {
            context.evaluatePolicy_localizedReason_reply(
                LAPolicy::DeviceOwnerAuthentication,
                &reason,
                &reply,
            );
        }
    }
}

/// Unit tests never reach LocalAuthentication: `evaluatePolicy` would put a real Touch ID /
/// password dialog on the screen of whoever runs `cargo test`, and even `canEvaluatePolicy` asks the
/// system, so the answer would differ between a Mac with Touch ID, a CI runner and Linux. Here
/// owner authentication is not available, and a prompt that is forced anyway is cancelled at once.
#[cfg(test)]
mod imp {
    use super::{AuthError, Callback};

    pub fn can_evaluate(_biometrics_only: bool) -> Result<(), AuthError> {
        Err(AuthError::Unavailable("owner authentication is off in unit tests".into()))
    }

    pub fn authenticate(_reason: &str, callback: Callback) {
        callback(Err(AuthError::Cancelled));
    }
}

#[cfg(all(not(target_os = "macos"), not(test)))]
mod imp {
    use super::{AuthError, Callback};

    fn unavailable() -> AuthError {
        AuthError::Unavailable("owner authentication is only supported on macOS".into())
    }

    pub fn can_evaluate(_biometrics_only: bool) -> Result<(), AuthError> {
        Err(unavailable())
    }

    pub fn authenticate(_reason: &str, callback: Callback) {
        callback(Err(unavailable()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(code: isize) -> AuthError {
        map_error(LA_ERROR_DOMAIN, code, "msg".into())
    }

    #[test]
    fn cancellations() {
        for c in [code::USER_CANCEL, code::SYSTEM_CANCEL, code::APP_CANCEL] {
            assert_eq!(map(c), AuthError::Cancelled);
        }
    }

    #[test]
    fn failures() {
        for c in [code::AUTHENTICATION_FAILED, code::USER_FALLBACK, code::BIOMETRY_LOCKOUT] {
            assert_eq!(map(c), AuthError::Failed("msg".into()));
        }
        assert!(matches!(map(-9999), AuthError::Failed(m) if m.contains("-9999")));
    }

    #[test]
    fn unavailable() {
        for c in [
            code::NOT_INTERACTIVE,
            code::PASSCODE_NOT_SET,
            code::BIOMETRY_NOT_AVAILABLE,
            code::BIOMETRY_NOT_ENROLLED,
            code::COMPANION_NOT_AVAILABLE,
            code::INVALID_CONTEXT,
        ] {
            assert_eq!(map(c), AuthError::Unavailable("msg".into()));
        }
    }

    #[test]
    fn foreign_domain_is_failure() {
        assert!(matches!(map_error("NSCocoaErrorDomain", -2, "x".into()), AuthError::Failed(_)));
    }

    #[test]
    fn availability_is_asked_once_per_ttl() {
        let slot = Mutex::new(None);
        let mut asked = 0;
        for _ in 0..5 {
            assert!(cached(&slot, Duration::from_secs(60), || {
                asked += 1;
                true
            }));
        }
        assert_eq!(asked, 1);
        assert!(!cached(&slot, Duration::ZERO, || false), "an expired answer is asked again");
    }

    /// Pins the stub above. Before it, a test asked the real `LAContext` from `cargo test`, and one
    /// that reached `authenticate` would have shown a Touch ID prompt to whoever ran it.
    #[test]
    fn tests_never_reach_the_system_dialog() {
        assert!(!available() && !biometrics_available(), "nothing is available under test");
        let (tx, rx) = std::sync::mpsc::channel();
        authenticate("a test", move |result| {
            let _ = tx.send(result);
        });
        // Answered before `authenticate` returns; a real dialog would still be open (or, on a
        // runner without a session, fail with `Unavailable`).
        assert_eq!(rx.try_recv(), Ok(Err(AuthError::Cancelled)));
    }
}
