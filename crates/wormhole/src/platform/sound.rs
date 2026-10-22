//! The system's alert sound.

use super::background_mode;
use super::os::sound as os;

/// The system's alert sound (the bell); never from a test instance in background mode.
pub fn beep() {
    if !background_mode::on() {
        os::beep();
    }
}
