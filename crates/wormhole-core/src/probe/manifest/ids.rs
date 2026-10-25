//! The spelling of ids: a Probe's, and the local id of each thing it contributes.

/// A Probe's id: `[a-z0-9-]{2,32}`. It names the Probe's folder, so nothing in it can lead a path
/// out of the data folder.
pub fn is_probe_id(id: &str) -> bool {
    (2..=32).contains(&id.len()) && id.bytes().all(id_byte)
}

/// The id of one contribution inside its Probe: `[a-z0-9-]{1,32}`. Its global id is
/// `<probe id>/<local id>`.
pub fn is_local_id(id: &str) -> bool {
    (1..=32).contains(&id.len()) && id.bytes().all(id_byte)
}

fn id_byte(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_probe_id_is_two_to_thirty_two_lowercase_letters_digits_and_dashes() {
        for good in ["ab", "acme-tools", "a1", "--", &"a".repeat(32)] {
            assert!(is_probe_id(good), "{good}");
        }
        for bad in ["", "a", "Acme", "acme_tools", "acme tools", "../x", "a/b", "é1", &"a".repeat(33)] {
            assert!(!is_probe_id(bad), "{bad}");
        }
    }

    #[test]
    fn a_local_id_may_be_one_character() {
        assert!(is_local_id("a"));
        assert!(!is_local_id(""));
        assert!(!is_local_id("A"));
        assert!(!is_local_id(&"a".repeat(33)));
    }
}
