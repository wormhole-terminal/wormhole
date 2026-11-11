//! The tones a rule or a hook may paint with: design tokens by name, never a colour.

/// A tone, which the app resolves through the active theme (`token`), so a Probe's decoration
/// follows light and dark, the contrast settings and any theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tone {
    Link,
    Danger,
    Warning,
    Success,
    Muted,
}

impl Tone {
    pub const ALL: [Tone; 5] = [Self::Link, Self::Danger, Self::Warning, Self::Success, Self::Muted];

    /// The tone a manifest names, when it is one (`validate` has refused any other).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tone| tone.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Danger => "danger",
            Self::Warning => "warning",
            Self::Success => "success",
            Self::Muted => "muted",
        }
    }

    /// The design token the tone is drawn with, as text (`theme::tokens`' catalog has each).
    pub fn token(self) -> &'static str {
        match self {
            Self::Link => "text.link",
            Self::Danger => "status.danger.text",
            Self::Warning => "status.warning.text",
            Self::Success => "status.success.text",
            Self::Muted => "text.muted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Tone;
    use crate::probe::validate::TONES;
    use crate::theme::tokens::UI_TOKENS;

    #[test]
    fn the_tones_are_the_ones_validate_accepts_and_each_token_exists() {
        let names: Vec<_> = Tone::ALL.iter().map(|tone| tone.name()).collect();
        assert_eq!(names, TONES);
        for tone in Tone::ALL {
            assert_eq!(Tone::from_name(tone.name()), Some(tone));
            assert!(UI_TOKENS.iter().any(|t| t.name == tone.token()), "{}", tone.token());
        }
        assert_eq!(Tone::from_name("#ff0000"), None);
    }
}
