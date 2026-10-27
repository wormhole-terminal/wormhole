use super::*;

#[test]
fn every_agent_has_a_unique_id_and_command() {
    for (i, a) in AGENTS.iter().enumerate() {
        assert_eq!(agent(a.id).name, a.name);
        assert!(find(a.id).is_some_and(|found| std::ptr::eq(found, a)));
        assert!(AGENTS[i + 1..].iter().all(|b| b.id != a.id && b.command != a.command));
        assert!(a.executables.contains(&a.command), "{} must be detectable by its command", a.name);
    }
}

/// An agent is marked with two capitals: the initials of its name's words ("CLI"
/// aside), a one-word name's first two letters, or a mark fixed for it.
#[test]
fn marks_follow_the_naming_rule() {
    assert_eq!(agent(AgentId::Claude).monogram, "CC");
    assert_eq!(agent(AgentId::Codex).monogram, "CX");
    assert_eq!(agent(AgentId::Gemini).monogram, "GM");
    // Codex is CX so it isn't Claude Code's CC again; Gemini is GM;
    // OpenCode's two words are written as one.
    let fixed = [(AgentId::Codex, "CX"), (AgentId::Gemini, "GM"), (AgentId::OpenCode, "OC")];
    for (i, a) in AGENTS.iter().enumerate() {
        assert!(
            a.monogram.len() == 2 && a.monogram.chars().all(|c| c.is_ascii_uppercase()),
            "{}: {}",
            a.name,
            a.monogram
        );
        let words: Vec<&str> = a.name.split(' ').filter(|w| *w != "CLI").collect();
        let initials: String = match words.as_slice() {
            [one] => one.chars().take(2).collect(),
            many => many.iter().filter_map(|w| w.chars().next()).take(2).collect(),
        };
        let expected =
            fixed.iter().find(|(id, _)| *id == a.id).map_or(initials.to_uppercase(), |f| f.1.into());
        assert_eq!(a.monogram, expected, "{}", a.name);
        assert!(AGENTS[i + 1..].iter().all(|b| b.monogram != a.monogram), "{} twice", a.monogram);
    }
}

#[test]
fn short_names_drop_the_kind_of_program() {
    assert_eq!(agent(AgentId::Claude).short_name(), "Claude");
    assert_eq!(agent(AgentId::Codex).short_name(), "Codex");
    assert_eq!(agent(AgentId::Gemini).short_name(), "Gemini");
    assert_eq!(agent(AgentId::Cursor).short_name(), "Cursor");
    assert_eq!(agent(AgentId::Aider).short_name(), "Aider");
    for agent in AGENTS {
        assert!(!agent.short_name().is_empty(), "{}", agent.name);
    }
}

/// A brand's colour carries a white logo (`tile.mark.brand`), so it must stand at least 3:1 against
/// white, the contrast WCAG asks of a graphic. A gradient is checked at both ends.
#[test]
fn every_tile_colour_carries_a_white_logo() {
    // WCAG's relative luminance of an sRGB colour.
    fn luminance(rgb: u32) -> f64 {
        let channel = |shift: u32| {
            let c = f64::from((rgb >> shift) & 0xff) / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }
    for a in AGENTS {
        let colours: &[u32] = match &a.tile {
            Tile::Black => &[],
            Tile::Solid(c) => std::slice::from_ref(c),
            Tile::Gradient(from, to) => &[*from, *to],
        };
        for &c in colours {
            assert!(c <= 0xff_ffff, "{}: {c:#x} is not 0xRRGGBB", a.name);
            let contrast = 1.05 / (luminance(c) + 0.05);
            assert!(contrast >= 3.0, "{}: #{c:06x} is {contrast:.2}:1 against white", a.name);
        }
    }
}
