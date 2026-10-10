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
fn bundled_icons_resolve() {
    for a in AGENTS {
        if let Some(key) = a.icon() {
            assert!(key.starts_with(ICON_PREFIX));
            let svg = std::str::from_utf8(icon_bytes(key).expect("bytes")).expect("utf8");
            assert!(svg.contains("<svg"), "{key}");
        }
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
