//! What a notice is. Only its kind and the numbers its sentence needs; never the copied text itself.

use crate::agents::AgentId;

/// A short message the chip says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Notice {
    /// Something was put on the clipboard.
    Copied(Copied),
    /// A program that controls Wormhole (a coding agent, a script) acted on this terminal.
    Driven(Driven),
    /// Something the person asked for could not be done.
    Failed(Failed),
    /// Something the person should know and that stays until they have seen it: the chip waits for a
    /// click (see [`Notice::is_sticky`]).
    Warned(Warning),
    /// A message devctl made up (`notice <text>`, which has no sentence in the catalog); the number tells
    /// one from another and the words come with the post. Nothing but devctl makes one.
    Test(u64),
}

/// What a program that controls Wormhole did to a terminal, and which program it is: only what, never the
/// text read or typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Driven {
    /// It read what the terminal shows.
    Reading(Driver),
    /// It typed into the terminal, or had a command run in it.
    Typing(Driver),
}

/// Who did it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Driver {
    /// A coding agent of the registry.
    Agent(AgentId),
    /// Anything else: a script, or a command typed at a prompt.
    Scripts,
}

/// What could not be done.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Failed {
    /// A copy of a commit's message or author, which needed details git could not read.
    CopyCommit,
    /// A copy of a path with characters in it that would act when pasted into a terminal (a newline), or
    /// that is not text.
    CopyPath,
    /// A rename (Files tab) with a name that can't be one.
    RenameName,
    /// A rename onto a name that something has already.
    RenameExists,
    /// A rename the system does not allow.
    RenameDenied,
    /// A rename of something that is gone, or whose folder is, or that is behind a link.
    RenameGone,
    /// Any other failure of a rename.
    RenameFailed,
}

/// What the person is warned about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Warning {
    /// The helper that keeps terminals running couldn't start, so this terminal ends when Wormhole quits.
    HostFallback,
    /// A sticky message devctl made up (`notice sticky <text>`); the number tells one from another and the
    /// words come with the post. Nothing but devctl makes one.
    Test(u64),
}

/// What was copied, and how much.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Copied {
    /// Several lines of text, from a terminal's screen or a program's write to the clipboard.
    Lines(usize),
    /// A single line of it, told by its characters. Only what is said is kept: two copies that say the
    /// same words are the same notice.
    Characters(usize),
    /// A command block's output.
    Output { lines: usize },
    /// A command block's command line.
    Command,
    /// A folder's path.
    Path,
    /// A setting's key in `config.json`, from the command palette.
    SettingKey,
    /// A git error's message.
    Error,
    /// A git object's id.
    ObjectId,
    /// A commit's full hash, from the History tab.
    CommitHash,
    /// A commit's short hash.
    CommitShortHash,
    /// A commit's whole message.
    CommitMessage,
    /// A commit's message that was too long to read whole: its start.
    CommitMessageCut,
    /// A commit's subject line.
    CommitSubject,
    /// A commit's author, as `Name <email>`.
    CommitAuthor,
}

/// The picture beside the words: an SF Symbol on macOS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    /// Two sheets: a copy.
    Copy,
    /// An eye: something reads the terminal.
    Eye,
    /// A keyboard: something types into the terminal.
    Keyboard,
    /// A triangle with an exclamation mark: a warning.
    Warning,
}

impl Notice {
    #[must_use]
    pub fn glyph(self) -> Glyph {
        match self {
            Self::Copied(_) | Self::Failed(_) | Self::Test(_) => Glyph::Copy,
            Self::Driven(Driven::Reading(_)) => Glyph::Eye,
            Self::Driven(Driven::Typing(_)) => Glyph::Keyboard,
            Self::Warned(_) => Glyph::Warning,
        }
    }

    /// The chip keeps this one until the person clicks it, instead of leaving after its life. Only a
    /// warning is sticky: a copy says what happened and is gone.
    #[must_use]
    pub fn is_sticky(self) -> bool {
        matches!(self, Self::Warned(_))
    }

    /// There is more to say about it than the chip's line: an ⓘ opens it.
    #[must_use]
    pub fn has_info(self) -> bool {
        matches!(self, Self::Warned(Warning::HostFallback))
    }
}

impl Copied {
    /// Copied `text`: it is counted here and not kept.
    #[must_use]
    pub fn text(text: &str) -> Self {
        match text.lines().count() {
            0 | 1 => Self::Characters(text.chars().count()),
            lines => Self::Lines(lines),
        }
    }

    /// Copied the output `text` of a block.
    #[must_use]
    pub fn output(text: &str) -> Self {
        Self::Output { lines: text.lines().count().max(1) }
    }
}

impl From<Failed> for Notice {
    fn from(failed: Failed) -> Self {
        Self::Failed(failed)
    }
}

impl From<Copied> for Notice {
    fn from(copied: Copied) -> Self {
        Self::Copied(copied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_counted_and_not_kept() {
        assert_eq!(Copied::text("one"), Copied::Characters(3));
        assert_eq!(Copied::text("a\nbb\nccc"), Copied::Lines(3));
        assert_eq!(Copied::text("a\nb\n"), Copied::Lines(2), "a last break adds no line");
        assert_eq!(Copied::text("\n"), Copied::Characters(1));
        assert_eq!(Copied::text("日本"), Copied::Characters(2), "characters, not bytes");
        assert_eq!(Copied::text("a\nb"), Copied::text("c\nd"), "the same words are the same notice");
        assert_eq!(Copied::output("x\ny"), Copied::Output { lines: 2 });
    }

    #[test]
    fn every_copy_has_the_copy_glyph() {
        for copied in [Copied::Command, Copied::Path, Copied::SettingKey, Copied::Error, Copied::ObjectId] {
            assert_eq!(Notice::from(copied).glyph(), Glyph::Copy);
        }
    }

    #[test]
    fn a_program_driving_a_terminal_has_the_glyph_of_what_it_does() {
        let agent = Driver::Agent(AgentId::Claude);
        assert_eq!(Notice::Driven(Driven::Reading(agent)).glyph(), Glyph::Eye);
        assert_eq!(Notice::Driven(Driven::Typing(Driver::Scripts)).glyph(), Glyph::Keyboard);
    }

    #[test]
    fn only_a_warning_is_sticky_and_it_has_the_warning_glyph() {
        let warned = Notice::Warned(Warning::HostFallback);
        assert!(warned.is_sticky());
        assert_eq!(warned.glyph(), Glyph::Warning);
        assert!(!Notice::from(Copied::Path).is_sticky());
        assert!(!Notice::from(Failed::CopyCommit).is_sticky());
        assert!(!Notice::Test(1).is_sticky());
    }

    #[test]
    fn a_notice_is_small_and_copyable() {
        // It is passed around by value and kept in a map key: no text in it.
        assert!(std::mem::size_of::<Notice>() <= 24);
    }
}
