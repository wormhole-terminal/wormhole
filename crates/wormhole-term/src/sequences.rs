//! The escape sequences that mean something to Wormhole beyond drawing: the engine hands them to
//! the host (`engine::host`), which acts on them. What the engine does on its own (titles,
//! colours, modes, queries, the kitty keyboard protocol, the pictures' placement) is listed by
//! hand in `docs/escape-sequences.md`.
//!
//! This table is the list's source: the doc's "Read by Wormhole" table is [`table`], and a test
//! fails when the engine acts on an OSC number the table (or the test's list of the engine's own)
//! doesn't name, or the table names one the engine has dropped, so the list can't drift from the
//! code.

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
        does: "Links in the text. ⌘-click opens web and mail links and shows a `file:` link on this Mac \
               in the Finder; any other scheme is left alone.",
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
        does: "Resets the screen; Wormhole also forgets every program status record (a soft reset, \
               `CSI ! p`, doesn't).",
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

    use wormhole_vt::Terminal;
    use wormhole_vt::passthrough::{Passthrough, PassthroughError};

    use super::{SEQUENCES, table};

    /// The OSC numbers the engine acts on: the ones it reads itself, which a host may not claim
    /// for passthrough (asked of the engine, which lives in a repository of its own).
    fn engine_osc_numbers() -> BTreeSet<String> {
        (0..=99_999_u32)
            .filter(|&n| {
                matches!(
                    Terminal::builder().passthrough(Passthrough::new().osc(n, 1)),
                    Err(PassthroughError::ReservedOsc(_))
                )
            })
            .map(|n| n.to_string())
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

    /// What the engine does by itself and the host never hears of: titles, palette, special and
    /// dynamic colors, the clipboard and their resets (the doc lists them by hand), and the
    /// pointer shape (OSC 22), which the engine reports and the app does not act on.
    const THE_ENGINES_OWN: [&str; 29] = [
        "0", "1", "2", "4", "5", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "22", "52",
        "104", "105", "110", "111", "112", "113", "114", "115", "116", "117", "118", "119",
    ];

    #[test]
    fn every_osc_the_engine_acts_on_is_listed_and_nothing_else() {
        let acted_on = engine_osc_numbers();
        assert!(acted_on.contains("7501"), "the scan found the engine's arms: {acted_on:?}");
        let listed = listed_osc_numbers();
        let own: BTreeSet<String> = THE_ENGINES_OWN.iter().map(|n| n.to_string()).collect();
        assert!(
            listed.is_disjoint(&own),
            "listed twice: {:?}",
            listed.intersection(&own).collect::<Vec<_>>()
        );
        let named: BTreeSet<String> = listed.union(&own).cloned().collect();
        assert_eq!(
            acted_on, named,
            "the OSCs the engine reads (left), the list and the engine's own (right)"
        );
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
        let doc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../repo-docs/docs/escape-sequences.md");
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
