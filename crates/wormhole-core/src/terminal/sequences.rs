//! The escape sequences Wormhole reads itself, before alacritty's parser sees the bytes (the PTY
//! filters: `pty_filter::osc` for the OSCs, `pty_filter::images` for the pictures, `osc_tap` for
//! the full reset). What alacritty handles on its own (titles, colours, modes, queries, the kitty
//! keyboard protocol) is listed by hand in `docs/escape-sequences.md`.
//!
//! This table is the list's source: the doc's "Read by Wormhole" table is [`table`], and a test
//! fails when the parser acts on an OSC number the table doesn't name, or the table names one the
//! parser has dropped, so the list can't drift from the code.

/// One sequence, as a reader of the list needs it.
#[derive(Clone, Copy, Debug)]
pub struct Sequence {
    /// How it starts, as people write it: `OSC 7501`, `APC G`, `ESC c`.
    pub sequence: &'static str,
    /// What it is.
    pub name: &'static str,
    /// Who defined it, and the revision implemented where the spec has revisions.
    pub source: &'static str,
    /// What Wormhole does with it.
    pub does: &'static str,
}

pub const SEQUENCES: &[Sequence] = &[
    Sequence {
        sequence: "OSC 7",
        name: "Current folder (`file://host/path`)",
        source: "VTE / iTerm2 shell integration",
        does: "The terminal's folder: its row, the Changes panel, where a new terminal opens. A host \
               other than this Mac, or a folder that isn't there, is ignored.",
    },
    Sequence {
        sequence: "OSC 8",
        name: "Hyperlink",
        source: "Egmont Koblinger's hyperlink spec",
        does: "Parsed by alacritty; Wormhole watches for a link's end to keep a command block's \
               prompt mark.",
    },
    Sequence {
        sequence: "OSC 9",
        name: "Notification (`OSC 9 ; text`)",
        source: "iTerm2",
        does: "A notification in the program's words, under the notification settings. A one- or \
               two-digit first parameter is ConEmu's, below.",
    },
    Sequence {
        sequence: "OSC 9;4",
        name: "Progress",
        source: "ConEmu, Windows Terminal",
        does: "Kept per terminal, not shown yet.",
    },
    Sequence {
        sequence: "OSC 9;9",
        name: "Current folder",
        source: "ConEmu",
        does: "As OSC 7, without a host.",
    },
    Sequence {
        sequence: "OSC 99",
        name: "Notification",
        source: "kitty",
        does: "As OSC 9, with a title; chunked ones are joined (a few at a time, capped).",
    },
    Sequence {
        sequence: "OSC 133",
        name: "Prompt marks (`A` `B` `C` `D;code`)",
        source: "FinalTerm",
        does: "Command blocks, a command's exit code, and whether the terminal is busy or at its \
               prompt.",
    },
    Sequence {
        sequence: "OSC 777",
        name: "Notification (`notify ; title ; body`)",
        source: "rxvt-unicode, VTE",
        does: "As OSC 99.",
    },
    Sequence {
        sequence: "OSC 1337 ; File=",
        name: "Inline image",
        source: "iTerm2",
        does: "Shown in the grid (`docs/inline-images.md`).",
    },
    Sequence {
        sequence: "OSC 7501",
        name: "Program status",
        source: "Program Status Protocol, revision 0.4 (2026-10-10)",
        does: "A program's state (idle, working, blocked, done, error) is the terminal's status. \
               `OSC 7501 ; ?` is answered with `OSC 7501 ; ?`: every default state and kind.",
    },
    Sequence {
        sequence: "APC G",
        name: "Graphics",
        source: "kitty graphics protocol",
        does: "Shown in the grid, and answered as the protocol asks (`docs/inline-images.md`).",
    },
    Sequence {
        sequence: "DCS q",
        name: "Sixel",
        source: "DEC",
        does: "Shown in the grid (`docs/inline-images.md`).",
    },
    Sequence {
        sequence: "ESC c",
        name: "Full reset (RIS)",
        source: "ECMA-48",
        does: "Reset by alacritty; Wormhole also forgets every program status record (a soft \
               reset, `CSI ! p`, doesn't).",
    },
];

/// The list as the doc shows it: a Markdown table, one row per sequence.
pub fn table() -> String {
    let mut out = String::from("| Sequence | What | Defined by | What Wormhole does |\n|---|---|---|---|\n");
    for s in SEQUENCES {
        out.push_str(&format!("| `{}` | {} | {} | {} |\n", s.sequence, s.name, s.source, s.does));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::{SEQUENCES, table};

    /// The OSC numbers the parser matches on: each `[b"<number>"` that opens an arm of `parse`.
    fn parsed_osc_numbers() -> BTreeSet<String> {
        let source = include_str!("pty_filter/osc.rs");
        let parse = source.split("pub(super) fn parse(").nth(1).expect("osc::parse");
        let body = &parse[..parse.find("\n}\n").expect("the end of parse")];
        body.lines()
            .filter_map(|line| line.trim_start().strip_prefix("[b\""))
            .filter_map(|rest| rest.split('"').next())
            .map(str::to_string)
            .collect()
    }

    /// The OSC numbers the list names (`OSC 9;4` is OSC 9's).
    fn listed_osc_numbers() -> BTreeSet<String> {
        SEQUENCES
            .iter()
            .filter_map(|s| s.sequence.strip_prefix("OSC "))
            .map(|rest| rest.split([';', ' ']).next().unwrap_or(rest).to_string())
            .collect()
    }

    #[test]
    fn every_osc_the_parser_acts_on_is_listed_and_nothing_else() {
        let parsed = parsed_osc_numbers();
        assert!(parsed.contains("7501"), "the scan found the parser's arms: {parsed:?}");
        let mut listed = listed_osc_numbers();
        // Read by the image filter, not by `osc::parse`.
        assert!(listed.remove("1337"));
        assert!(include_str!("pty_filter/images.rs").contains("1337"));
        assert_eq!(parsed, listed, "the OSCs Wormhole reads (left) and the list in sequences.rs (right)");
    }

    #[test]
    fn each_row_is_one_line_of_the_table() {
        for s in SEQUENCES {
            for text in [s.sequence, s.name, s.source, s.does] {
                assert!(!text.is_empty() && !text.contains('\n') && !text.contains('|'), "{s:?}");
            }
        }
        assert_eq!(table().lines().count(), SEQUENCES.len() + 2);
    }

    /// The doc lives in the private docs folder, which a checkout may not have: checked where it
    /// is, so the list people read is the one the code keeps.
    #[test]
    fn the_doc_shows_this_table() {
        let doc =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/repo-docs/docs/escape-sequences.md");
        let Ok(text) = std::fs::read_to_string(&doc) else { return };
        let (start, end) = ("<!-- sequences:start -->\n", "<!-- sequences:end -->");
        let from = text.find(start).expect("the start marker") + start.len();
        let to = text.find(end).expect("the end marker");
        assert!(
            text[from..to] == table(),
            "docs/escape-sequences.md is out of date; put this between its markers:\n\n{}",
            table()
        );
    }
}
