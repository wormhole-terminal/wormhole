use super::*;

#[test]
fn bundled_icons_resolve() {
    for a in AGENTS {
        if let Some(key) = a.icon() {
            assert!(key.starts_with(ICON_PREFIX));
            let svg = std::str::from_utf8(icon_bytes(key).expect("bytes")).expect("utf8");
            assert!(svg.contains("<svg"), "{key}");
        }
    }
}

/// The registry names the logo's key and core holds the bytes behind it: the two lists agree.
#[test]
fn every_bundled_logo_belongs_to_an_agent() {
    for key in icons::keys() {
        assert!(AGENTS.iter().any(|a| a.icon() == Some(key)), "{key} is no agent's logo");
    }
}
