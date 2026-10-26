//! What a notice is. Only its kind and the numbers its sentence needs; never the copied text itself.

/// A short message the chip says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Notice {
    /// Something was put on the clipboard.
    Copied(Copied),
    /// A message devctl made up (`notice <text>`); the number tells one from another. The app has no words
    /// for it: they come with the post.
    Test(u64),
}

/// What was copied, and how much.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Copied {
    /// Text from a terminal's screen, or a program's write to the clipboard.
    Text { lines: usize, chars: usize },
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
}

/// The picture beside the words: an SF Symbol on macOS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Glyph {
    /// Two sheets: a copy.
    Copy,
}

impl Notice {
    #[must_use]
    pub fn glyph(self) -> Glyph {
        match self {
            Self::Copied(_) | Self::Test(_) => Glyph::Copy,
        }
    }
}

impl Copied {
    /// Copied `text`: it is counted here and not kept.
    #[must_use]
    pub fn text(text: &str) -> Self {
        // A selection that is only a line break still copied one line.
        Self::Text { lines: text.lines().count().max(1), chars: text.chars().count() }
    }

    /// Copied the output `text` of a block.
    #[must_use]
    pub fn output(text: &str) -> Self {
        Self::Output { lines: text.lines().count().max(1) }
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
        assert_eq!(Copied::text("one"), Copied::Text { lines: 1, chars: 3 });
        assert_eq!(Copied::text("a\nbb\nccc"), Copied::Text { lines: 3, chars: 8 });
        assert_eq!(Copied::text("a\nb\n"), Copied::Text { lines: 2, chars: 4 }, "a last break adds no line");
        assert_eq!(Copied::text("\n"), Copied::Text { lines: 1, chars: 1 });
        assert_eq!(Copied::text("日本"), Copied::Text { lines: 1, chars: 2 }, "characters, not bytes");
        assert_eq!(Copied::output("x\ny"), Copied::Output { lines: 2 });
    }

    #[test]
    fn every_copy_has_the_copy_glyph() {
        for copied in [Copied::Command, Copied::Path, Copied::SettingKey, Copied::Error, Copied::ObjectId] {
            assert_eq!(Notice::from(copied).glyph(), Glyph::Copy);
        }
    }

    #[test]
    fn a_notice_is_small_and_copyable() {
        // It is passed around by value and kept in a map key: no text in it.
        assert!(std::mem::size_of::<Notice>() <= 24);
    }
}
