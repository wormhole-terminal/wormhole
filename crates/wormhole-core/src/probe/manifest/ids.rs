//! The spelling of ids: a Probe's, and the local id of each thing it contributes.

/// A Probe's id: 2 to 32 of `[a-z0-9-]`, starting and ending with `[a-z0-9]`. It names the Probe's folder, so nothing in it can lead a path
/// out of the data folder.
pub fn is_probe_id(id: &str) -> bool {
    (2..=32).contains(&id.len()) && id.bytes().all(id_byte) && edges_ok(id)
}

/// The id of one contribution inside its Probe: 1 to 32 of `[a-z0-9-]`, starting and ending with `[a-z0-9]`. Its global id is
/// `<probe id>/<local id>`.
pub fn is_local_id(id: &str) -> bool {
    (1..=32).contains(&id.len()) && id.bytes().all(id_byte) && edges_ok(id)
}

/// An id starts and ends with a letter or a digit: `-rf` would be an option wherever an id reaches a
/// command line, and `--` ends one.
fn edges_ok(id: &str) -> bool {
    let alnum = |b: Option<&u8>| b.is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
    alnum(id.as_bytes().first()) && alnum(id.as_bytes().last())
}

fn id_byte(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_probe_id_is_two_to_thirty_two_lowercase_letters_digits_and_dashes() {
        for good in ["ab", "acme-tools", "a1", "a-b", &"a".repeat(32)] {
            assert!(is_probe_id(good), "{good}");
        }
        for bad in [
            "",
            "a",
            "--",
            "-rf",
            "x-",
            "-a",
            "a-",
            "Acme",
            "acme_tools",
            "acme tools",
            "../x",
            "a/b",
            "é1",
            &"a".repeat(33),
        ] {
            assert!(!is_probe_id(bad), "{bad}");
        }
    }

    #[test]
    fn a_local_id_may_be_one_character() {
        assert!(is_local_id("a"));
        assert!(!is_local_id(""));
        for bad in ["-", "-a", "a-", "--", "-rf"] {
            assert!(!is_local_id(bad), "{bad}");
        }
        assert!(!is_local_id("A"));
        assert!(!is_local_id(&"a".repeat(33)));
    }
}
