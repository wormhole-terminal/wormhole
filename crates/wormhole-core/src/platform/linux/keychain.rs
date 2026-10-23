use std::io;

use zeroize::Zeroizing;

fn unavailable() -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, "no Keychain on this system")
}

pub(crate) fn find(_service: &str, _account: &str) -> io::Result<Option<Zeroizing<Vec<u8>>>> {
    Err(unavailable())
}

pub(crate) fn add(_service: &str, _account: &str, _secret: &[u8]) -> io::Result<()> {
    Err(unavailable())
}

pub(crate) fn delete(_service: &str, _account: &str) -> io::Result<()> {
    Ok(())
}
