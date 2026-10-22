//! The system's alert sound.

use super::background_mode;
use super::os::sound as os;

/// The system's alert sound (the bell); never from a test instance in background mode, nor in tests.
pub fn beep() {
    if !cfg!(test) && !background_mode::on() {
        os::beep();
    }
}
