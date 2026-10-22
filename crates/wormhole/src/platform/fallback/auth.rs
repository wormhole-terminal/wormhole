//! `platform::auth` where owner authentication isn't supported: always unavailable.

use crate::platform::auth::{AuthError, Callback};

fn unavailable() -> AuthError {
    AuthError::Unavailable("owner authentication is only supported on macOS".into())
}

pub fn can_evaluate(_biometrics_only: bool) -> Result<(), AuthError> {
    Err(unavailable())
}

pub fn authenticate(_reason: &str, callback: Callback) {
    callback(Err(unavailable()));
}
