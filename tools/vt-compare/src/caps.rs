//! What an engine's adapter can answer, so a case it cannot check is "n/a" for it and not a failure.
//!
//! A conformance case is a script; one step it cannot run (a reply from an engine that has no
//! channel to write to, an underline color from an engine that has none) makes the whole case
//! "n/a" for that engine. The alternative, running the steps it can, would pass cases for the
//! wrong reason.

use wormhole_vt_bench::Attrs;
use wormhole_vt_bench::vtt::{Case, CellCheck, StepKind};

/// The questions an adapter can answer. Everything else is answered by every engine.
#[derive(Debug, Clone, Copy)]
pub struct Caps {
    /// It writes answers to queries (DSR, DA, DECRQM, color requests) and the case can read them.
    pub replies: bool,
    /// The window title the program set (OSC 0 and 2).
    pub title: bool,
    /// OSC 8 hyperlinks on cells.
    pub links: bool,
    /// The color of an underline (SGR 58).
    pub underline_color: bool,
    /// The attributes a cell can report; a case that checks others is n/a.
    pub attrs: Attrs,
    /// Whether a row says it continues in the next (`wrapped`, `lines`).
    pub wrapped: bool,
    /// Whether a row says how big it is drawn (DECDWL, DECDHL; `line-size`).
    pub line_sizes: bool,
    /// Lines scrolled off the top, readable one by one.
    pub scrollback: bool,
}

impl Caps {
    /// An engine that answers every question.
    pub const ALL: Caps = Caps {
        replies: true,
        title: true,
        links: true,
        underline_color: true,
        attrs: Attrs::all(),
        wrapped: true,
        line_sizes: true,
        scrollback: true,
    };
}

/// The capabilities of the engine called `name`; an engine this crate does not know answers
/// everything (the main workspace's own engines are held to the whole suite).
pub fn caps_of(name: &str) -> Caps {
    match name {
        // No output channel, and a cell is five attributes and two colors.
        "vt100" => Caps {
            replies: false,
            links: false,
            underline_color: false,
            line_sizes: false,
            attrs: Attrs::BOLD | Attrs::DIM | Attrs::ITALIC | Attrs::UNDERLINE | Attrs::INVERSE,
            ..Caps::ALL
        },
        // The adapters of the other engines do not read a row's size.
        "alacritty" | "ghostty" | "wezterm" => Caps { line_sizes: false, ..Caps::ALL },
        _ => Caps::ALL,
    }
}

/// Why `case` cannot be run on an engine with `caps`, if it cannot.
pub fn not_applicable(case: &Case, caps: &Caps) -> Option<&'static str> {
    for step in &case.steps {
        match &step.kind {
            StepKind::Reply(_) if !caps.replies => return Some("no replies"),
            StepKind::Title(_) if !caps.title => return Some("no title"),
            StepKind::Wrapped { .. } | StepKind::Lines(_) if !caps.wrapped => return Some("no wrap flag"),
            StepKind::LineSize { .. } if !caps.line_sizes => return Some("no line sizes"),
            StepKind::Scrollback(_) | StepKind::ScrollbackLen(_) if !caps.scrollback => {
                return Some("no history");
            }
            StepKind::Cell { checks, .. } => {
                for check in checks {
                    match check {
                        CellCheck::Link(_) if !caps.links => return Some("no hyperlinks"),
                        CellCheck::Underline(_) if !caps.underline_color => {
                            return Some("no underline color");
                        }
                        CellCheck::Attrs(want) if !caps.attrs.contains(*want) => {
                            return Some("attribute not kept");
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use wormhole_vt_bench::vtt::parse_file;

    use super::{Caps, caps_of, not_applicable};

    fn case(body: &str) -> wormhole_vt_bench::vtt::Case {
        let text = format!("test t\nspec s\nsize 10 2\n{body}");
        parse_file("t.vtt", &text).unwrap_or_else(|e| panic!("{e:?}")).remove(0)
    }

    #[test]
    fn a_step_the_engine_cannot_answer_makes_the_case_not_applicable() {
        let vt100 = caps_of("vt100");
        assert_eq!(not_applicable(&case("feed \\e[5n\nreply \\e[0n\n"), &vt100), Some("no replies"));
        assert_eq!(
            not_applicable(&case("feed a\ncell 1,1 attrs=strike\n"), &vt100),
            Some("attribute not kept")
        );
        assert_eq!(not_applicable(&case("feed a\ncell 1,1 attrs=bold,italic\n"), &vt100), None);
        assert_eq!(not_applicable(&case("feed a\ncell 1,1 link=none\n"), &vt100), Some("no hyperlinks"));
    }

    #[test]
    fn engines_that_answer_everything_run_every_case() {
        let all = Caps::ALL;
        assert_eq!(not_applicable(&case("feed \\e[5n\nreply \\e[0n\ntitle none\n"), &all), None);
        assert_eq!(caps_of("ghostty").replies, caps_of("alacritty").replies);
    }
}
