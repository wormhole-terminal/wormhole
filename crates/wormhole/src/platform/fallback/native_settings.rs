//! `platform::native_settings` without a native window: there is none.

pub fn available() -> bool {
    false
}

pub fn open(_: &str, _: bool, _: fn(&str)) {}

pub fn update(_: &str) {}

pub fn is_open() -> bool {
    false
}

pub fn shot(_: &str) -> bool {
    false
}
