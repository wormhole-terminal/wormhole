//! LocalAuthentication's side of `platform::auth`: an `LAContext` asked whether it can evaluate the
//! owner policy, and asked to.

use crate::platform::auth::{AuthError, Callback, map_error};
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
    unsafe { context.canEvaluatePolicy_error(policy(biometrics_only)) }.map_err(|error| from_ns_error(&error))
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
        context.evaluatePolicy_localizedReason_reply(LAPolicy::DeviceOwnerAuthentication, &reason, &reply);
    }
}
