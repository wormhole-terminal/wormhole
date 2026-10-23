use std::io;

use core_foundation::data::CFData;
use security_framework::base::Error;
use security_framework::item::{ItemAddOptions, ItemAddValue, ItemClass};
use zeroize::Zeroizing;

const ITEM_NOT_FOUND: i32 = -25300;
const DUPLICATE_ITEM: i32 = -25299;

/// A duplicate item is `AlreadyExists`; anything else carries the system's words.
fn error(err: &Error) -> io::Error {
    let kind = if err.code() == DUPLICATE_ITEM { io::ErrorKind::AlreadyExists } else { io::ErrorKind::Other };
    io::Error::new(kind, err.to_string())
}

pub(crate) fn find(service: &str, account: &str) -> io::Result<Option<Zeroizing<Vec<u8>>>> {
    match security_framework::passwords::get_generic_password(service, account) {
        Ok(found) => Ok(Some(Zeroizing::new(found))),
        Err(err) if err.code() == ITEM_NOT_FOUND => Ok(None),
        // Denied, cancelled, a locked Keychain.
        Err(err) => Err(error(&err)),
    }
}

/// `SecItemAdd` alone: unlike `set_generic_password`, it never replaces an item that is there.
pub(crate) fn add(service: &str, account: &str, secret: &[u8]) -> io::Result<()> {
    let mut options = ItemAddOptions::new(ItemAddValue::Data {
        class: ItemClass::generic_password(),
        data: CFData::from_buffer(secret),
    });
    options.set_service(service).set_account_name(account).set_label(service);
    options.add().map_err(|err| error(&err))
}

pub(crate) fn delete(service: &str, account: &str) -> io::Result<()> {
    match security_framework::passwords::delete_generic_password(service, account) {
        Err(err) if err.code() != ITEM_NOT_FOUND => Err(error(&err)),
        _ => Ok(()),
    }
}
