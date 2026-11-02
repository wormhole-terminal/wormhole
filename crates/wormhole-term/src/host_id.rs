//! What a session host's id looks like: 12 characters from `[a-z0-9]`. A name that fails this is never made into
//! a path, a socket name or a directory (`hosted::meta`, `checkpoint::store`).

/// Whether `id` is a host id.
pub(crate) fn valid(id: &str) -> bool {
    id.len() == 12 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}
