//! The terminals whose settings can be read.

/// A terminal (or an editor's terminal) whose settings Wormhole can read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    Ghostty,
    ITerm2,
    TerminalApp,
    Alacritty,
    Kitty,
    WezTerm,
    Warp,
    /// The terminal built into the Zed editor.
    Zed,
    /// The integrated terminal of VS Code (and its builds with the same settings folder layout).
    VsCode,
}

impl Source {
    /// Every source, in the order they are looked for.
    pub const ALL: [Source; 9] = [
        Source::Ghostty,
        Source::ITerm2,
        Source::TerminalApp,
        Source::Alacritty,
        Source::Kitty,
        Source::WezTerm,
        Source::Warp,
        Source::Zed,
        Source::VsCode,
    ];

    /// The stable name devctl and config files use for it.
    pub fn key(self) -> &'static str {
        match self {
            Source::Ghostty => "ghostty",
            Source::ITerm2 => "iterm2",
            Source::TerminalApp => "terminal-app",
            Source::Alacritty => "alacritty",
            Source::Kitty => "kitty",
            Source::WezTerm => "wezterm",
            Source::Warp => "warp",
            Source::Zed => "zed",
            Source::VsCode => "vscode",
        }
    }

    /// The source a [`key`](Self::key) names.
    pub fn from_key(key: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|source| source.key() == key)
    }

    /// The product's name as it is written, for the names of the theme files and shell profiles an
    /// import saves. It is a proper name, not interface text: the Settings window words the rest.
    pub(crate) fn brand(self) -> &'static str {
        match self {
            Source::Ghostty => "Ghostty",
            Source::ITerm2 => "iTerm2",
            Source::TerminalApp => "Terminal",
            Source::Alacritty => "Alacritty",
            Source::Kitty => "kitty",
            Source::WezTerm => "WezTerm",
            Source::Warp => "Warp",
            Source::Zed => "Zed",
            Source::VsCode => "VS Code",
        }
    }
}
