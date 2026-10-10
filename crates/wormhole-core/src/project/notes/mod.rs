//! `notes.md`: shared notes and next steps, synced both ways with the app.
//!
//! The file is meant to be edited by hand too, so parsing is forgiving (missing sections, `*`/`+`
//! bullets, plain bullets as tasks, stray text) and rendering normalizes: `render(parse(x))` may
//! differ from `x` once, after which it is a fixed point.

mod merge;
mod sync;

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

pub use merge::merge_notes;
pub use sync::{NotesBase, NotesBaseStore, NotesFile, SyncPlan, plan_sync};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesDoc {
    pub tasks: Vec<TaskItem>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskItem {
    pub text: String,
    pub done: bool,
}

const TITLE: &str = "# Notes";
const MARKER: &str = "<!-- Synced with Wormhole. Tasks are \"- [ ]\" items under \"## Next steps\"; \
                      notes are separated by \"---\". -->";
const MARKER_NEEDLE: &str = "Synced with Wormhole";
const TASKS_HEADING: &str = "## Next steps";
const NOTES_HEADING: &str = "## Notes";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    /// Before any recognized `##` heading: checkboxes are tasks, everything else is notes.
    Preamble,
    Tasks,
    Notes,
}

struct Fence {
    ch: char,
    len: usize,
}

/// Parse `notes.md`.
///
/// - `## Next steps` (also `## Tasks`, `## Todo`, case-insensitive) holds tasks: `-`/`*`/`+`
///   bullets, with `[ ]`/`[x]` or without a checkbox (not done). Indented non-checkbox lines
///   continue the previous task. Other text there is kept as a note rather than dropped.
/// - `## Notes` holds notes separated by lines of three or more dashes. Fenced code blocks are
///   opaque, so a `---` inside them doesn't split.
/// - Content before either heading (after the `# ` title and the sync marker) is treated like
///   the notes section, except that checkbox items become tasks.
/// - Any other `##` heading starts note content (it's kept verbatim inside the note).
pub fn parse_notes_md(text: &str) -> NotesDoc {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut doc = NotesDoc::default();
    let mut section = Section::Preamble;
    let mut fence: Option<Fence> = None;
    let mut current: Vec<&str> = Vec::new();
    let mut seen_content = false;
    // Whether an indented line may continue the last task (reset by blank lines, as in Markdown).
    let mut task_open = false;

    for line in text.lines() {
        if let Some(open) = &fence {
            if closes_fence(open, line) {
                fence = None;
            }
            current.push(line);
            continue;
        }
        let trimmed = line.trim();
        if section == Section::Preamble && !seen_content && !trimmed.is_empty() {
            seen_content = true;
            if trimmed.starts_with("# ") {
                continue; // document title; re-rendered as `# Notes`
            }
        }
        // Text that render escaped because it would read as structure (see `escape_note`).
        if let Some(text) = line.strip_prefix('\\')
            && escapable(text)
        {
            current.push(text);
            continue;
        }
        if is_marker(trimmed) {
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            let next = section_heading(heading);
            flush_note(&mut current, &mut doc.notes);
            task_open = false;
            match next {
                Some(next) => {
                    section = next;
                    continue;
                }
                // A custom heading can't be a task; it begins a note.
                None if section == Section::Tasks => section = Section::Notes,
                None => {}
            }
        }
        if is_separator(trimmed) {
            flush_note(&mut current, &mut doc.notes);
            task_open = false;
            continue;
        }
        if let Some(open) = open_fence(line) {
            fence = Some(open);
            task_open = false;
            current.push(line);
            continue;
        }
        if section != Section::Notes {
            let indented = line.starts_with("  ") || line.starts_with('\t');
            if let Some(task) = parse_task(line, false) {
                flush_note(&mut current, &mut doc.notes);
                push_task(&mut doc.tasks, task);
                task_open = true;
                continue;
            }
            if section == Section::Tasks && indented && task_open && !trimmed.is_empty() {
                if let Some(last) = doc.tasks.last_mut() {
                    let rest = parse_task(line, true).map_or(trimmed.to_string(), |t| t.text);
                    last.text = normalize_task(&format!("{} {rest}", last.text));
                }
                continue;
            }
            if section == Section::Tasks
                && let Some(task) = parse_task(line, true)
            {
                flush_note(&mut current, &mut doc.notes);
                push_task(&mut doc.tasks, task);
                task_open = true;
                continue;
            }
            if trimmed.is_empty() {
                task_open = false;
            }
        }
        current.push(line);
    }
    flush_note(&mut current, &mut doc.notes);
    doc
}

/// Render in the canonical format: `# Notes`, the sync marker comment, `## Next steps` with one
/// `- [ ]`/`- [x]` line per task, then `## Notes` with notes separated by blank-line-padded `---`.
pub fn render_notes_md(doc: &NotesDoc) -> String {
    let doc = normalize(doc);
    let mut out = format!("{TITLE}\n{MARKER}\n\n{TASKS_HEADING}\n");
    for task in &doc.tasks {
        out.push_str(if task.done { "- [x] " } else { "- [ ] " });
        out.push_str(&task.text);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(NOTES_HEADING);
    out.push('\n');
    for (ix, note) in doc.notes.iter().enumerate() {
        if ix > 0 {
            out.push_str("\n---\n");
        }
        out.push('\n');
        out.push_str(&escape_note(note));
        out.push('\n');
    }
    out
}

/// Lines [`parse_notes_md`] reads as structure rather than text (outside a fence): separators,
/// `## ` headings, the sync marker and fence openers.
fn structural(line: &str) -> bool {
    let trimmed = line.trim();
    is_separator(trimmed) || line.starts_with("## ") || is_marker(trimmed) || open_fence(line).is_some()
}

/// A structural line, or one already escaped (any number of times): parse removes one `\` from
/// these, so render adds one to every such line it means literally.
fn escapable(line: &str) -> bool {
    structural(line) || line.strip_prefix('\\').is_some_and(escapable)
}

/// One note as it goes into notes.md. A line that would read as structure (a `---`, which would
/// split the note; `## Todo`, which would turn the rest into tasks; a fence that never closes,
/// which would swallow the notes after it) gets a leading `\`. That is Markdown's own escape, so
/// the file still shows `---` and `## Todo` as text. Everything else is written as it is: a fence
/// that closes, or a custom heading on a note's first line, already round-trips.
fn escape_note(note: &str) -> Cow<'_, str> {
    let lines: Vec<&str> = note.lines().collect();
    let mut fence: Option<Fence> = None;
    let mut escaped: Option<Vec<Cow<'_, str>>> = None;
    for (ix, line) in lines.iter().enumerate() {
        let escape = match &fence {
            Some(open) => {
                if closes_fence(open, line) {
                    fence = None;
                }
                false
            }
            None => match (open_fence(line), line.strip_prefix("## ")) {
                (Some(open), _) => {
                    let closes = lines[ix + 1..].iter().any(|l| closes_fence(&open, l));
                    if closes {
                        fence = Some(open);
                    }
                    !closes
                }
                // A custom heading may start a note; anywhere else it would start a new one.
                (None, Some(heading)) => ix > 0 || section_heading(heading).is_some(),
                (None, None) => structural(line) || line.strip_prefix('\\').is_some_and(escapable),
            },
        };
        if escape {
            let out = escaped.get_or_insert_with(|| lines[..ix].iter().map(|l| Cow::Borrowed(*l)).collect());
            out.push(Cow::Owned(format!("\\{line}")));
        } else if let Some(out) = &mut escaped {
            out.push(Cow::Borrowed(*line));
        }
    }
    match escaped {
        Some(lines) => Cow::Owned(lines.join("\n")),
        None => Cow::Borrowed(note),
    }
}

/// The section a `## ` heading's text names, if it is one we recognize.
fn section_heading(heading: &str) -> Option<Section> {
    match heading.trim().trim_end_matches(':').to_lowercase().as_str() {
        "next steps" | "tasks" | "todo" | "to do" => Some(Section::Tasks),
        "notes" => Some(Section::Notes),
        _ => None,
    }
}

fn is_marker(trimmed: &str) -> bool {
    trimmed.starts_with("<!--") && trimmed.contains(MARKER_NEEDLE) && trimmed.ends_with("-->")
}

/// The form `parse(render(doc))` returns: trimmed notes/tasks, empty ones dropped.
fn normalize(doc: &NotesDoc) -> NotesDoc {
    NotesDoc {
        tasks: doc
            .tasks
            .iter()
            .map(|t| TaskItem { text: normalize_task(&t.text), done: t.done })
            .filter(|t| !t.text.is_empty())
            .collect(),
        notes: doc.notes.iter().filter_map(|n| normalize_note(n)).collect(),
    }
}

/// Tasks are single-line in the app, so any internal line breaks become spaces.
fn normalize_task(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Drop blank edge lines and trailing whitespace; keep inner formatting (indentation, blank
/// lines) untouched since it's Markdown.
fn normalize_note(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let first = lines.iter().position(|l| !l.trim().is_empty())?;
    let last = lines.iter().rposition(|l| !l.trim().is_empty())?;
    Some(lines[first..=last].iter().map(|l| l.trim_end()).collect::<Vec<_>>().join("\n"))
}

fn flush_note(lines: &mut Vec<&str>, notes: &mut Vec<String>) {
    if let Some(note) = normalize_note(&lines.join("\n")) {
        notes.push(note);
    }
    lines.clear();
}

fn push_task(tasks: &mut Vec<TaskItem>, task: TaskItem) {
    // `- [ ]` with no text is an empty template line; nothing to keep.
    if !task.text.is_empty() {
        tasks.push(task);
    }
}

/// A `-`/`*`/`+` bullet with a checkbox, or (with `allow_plain`) without one.
fn parse_task(line: &str, allow_plain: bool) -> Option<TaskItem> {
    let rest = line.trim_start();
    let rest = ["- ", "* ", "+ "].iter().find_map(|b| rest.strip_prefix(b))?;
    for (box_, done) in [("[ ]", false), ("[x]", true), ("[X]", true)] {
        if let Some(text) = rest.strip_prefix(box_)
            && (text.is_empty() || text.starts_with(char::is_whitespace))
        {
            return Some(TaskItem { text: normalize_task(text), done });
        }
    }
    allow_plain.then(|| TaskItem { text: normalize_task(rest), done: false })
}

fn is_separator(trimmed: &str) -> bool {
    trimmed.len() >= 3 && trimmed.chars().all(|c| c == '-')
}

/// CommonMark fence: up to three spaces, then three or more backticks or tildes.
fn open_fence(line: &str) -> Option<Fence> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let ch = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let len = rest.chars().take_while(|c| *c == ch).count();
    (len >= 3).then_some(Fence { ch, len })
}

fn closes_fence(fence: &Fence, line: &str) -> bool {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return false;
    }
    let len = trimmed.chars().take_while(|c| *c == fence.ch).count();
    len >= fence.len && trimmed[len * fence.ch.len_utf8()..].trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) const SAMPLE: &str = "# Notes
<!-- Synced with Wormhole. Tasks are \"- [ ]\" items under \"## Next steps\"; notes are separated by \"---\". -->

## Next steps
- [ ] Fix the flaky login test
- [x] Update README

## Notes

First note, can span
multiple lines

---

Second note
";

    pub(super) fn task(text: &str, done: bool) -> TaskItem {
        TaskItem { text: text.into(), done }
    }

    pub(super) fn doc(tasks: &[(&str, bool)], notes: &[&str]) -> NotesDoc {
        NotesDoc {
            tasks: tasks.iter().map(|(t, d)| task(t, *d)).collect(),
            notes: notes.iter().map(|n| n.to_string()).collect(),
        }
    }

    #[test]
    fn parses_canonical_sample_and_renders_identically() {
        let parsed = parse_notes_md(SAMPLE);
        assert_eq!(
            parsed,
            doc(
                &[("Fix the flaky login test", false), ("Update README", true)],
                &["First note, can span\nmultiple lines", "Second note"]
            )
        );
        assert_eq!(render_notes_md(&parsed), SAMPLE);
    }

    #[test]
    fn empty_and_minimal() {
        assert_eq!(parse_notes_md(""), NotesDoc::default());
        let rendered = render_notes_md(&NotesDoc::default());
        assert_eq!(parse_notes_md(&rendered), NotesDoc::default());
        assert!(rendered.contains(TASKS_HEADING) && rendered.contains(NOTES_HEADING));
    }

    #[test]
    fn tolerates_hand_edits() {
        let text = "# My project\r\n\r\n## TODO:\r\n* [ ] star bullet\r\n+ [X] plus bullet\r\n- plain bullet\r\n\
                    - [ ] wrapped task\r\n  continues here\r\n- [ ]\r\n\r\n\r\nstray text in tasks\r\n\r\n\
                    ## notes\r\n\r\n\r\n\r\none\r\n\r\n-----\r\n\r\n---\r\ntwo   \r\n\r\n\r\n";
        let parsed = parse_notes_md(text);
        assert_eq!(
            parsed.tasks,
            [
                task("star bullet", false),
                task("plus bullet", true),
                task("plain bullet", false),
                task("wrapped task continues here", false)
            ]
        );
        assert_eq!(parsed.notes, ["stray text in tasks", "one", "two"]);
    }

    #[test]
    fn missing_sections() {
        // Only notes, no headings at all.
        let parsed = parse_notes_md("just a thought\n\n---\n\nanother\n");
        assert_eq!(parsed, doc(&[], &["just a thought", "another"]));
        // Only a checklist.
        let parsed = parse_notes_md("- [ ] a\n- [x] b\n");
        assert_eq!(parsed, doc(&[("a", false), ("b", true)], &[]));
        // Plain bullets outside the tasks section are note content, not tasks.
        let parsed = parse_notes_md("## Notes\n- [ ] in a note\n- bullet\n");
        assert_eq!(parsed, doc(&[], &["- [ ] in a note\n- bullet"]));
        // Notes before tasks.
        let parsed = parse_notes_md("## Notes\nn\n## Next steps\n- [ ] t\n");
        assert_eq!(parsed, doc(&[("t", false)], &["n"]));
    }

    #[test]
    fn fenced_code_does_not_split() {
        let text = "## Notes\n\nConfig:\n\n```yaml\n---\nkey: v\n## Next steps\n- [ ] no\n```\n\nafter\n\n---\n\n\
                    ~~~~\n---\n~~~\n---\n~~~~\n";
        let parsed = parse_notes_md(text);
        assert!(parsed.tasks.is_empty());
        assert_eq!(
            parsed.notes,
            [
                "Config:\n\n```yaml\n---\nkey: v\n## Next steps\n- [ ] no\n```\n\nafter",
                "~~~~\n---\n~~~\n---\n~~~~"
            ]
        );
        // Unclosed fence swallows the rest, like Markdown renders it.
        let parsed = parse_notes_md("## Notes\n```\na\n---\nb\n");
        assert_eq!(parsed.notes, ["```\na\n---\nb"]);
        // Fenced code must survive a round trip.
        assert_eq!(parse_notes_md(&render_notes_md(&parse_notes_md(text))), parse_notes_md(text));
    }

    #[test]
    fn custom_headings_are_kept_as_notes() {
        let parsed = parse_notes_md("## Next steps\n- [ ] t\n## Decisions\nUse SQLite\n");
        assert_eq!(parsed, doc(&[("t", false)], &["## Decisions\nUse SQLite"]));
    }

    #[test]
    fn round_trip_is_stable_after_one_normalization() {
        let inputs = [
            SAMPLE,
            "",
            "random\ntext",
            "# T\n- [ ] a\n\nb\n---\nc\n## Notes\n```\n---\n```\n",
            "## Next steps\n\n\n- x\n  - sub\n\n## Notes\n\n  indented note\n\n---\n---\n",
            "## Notes\nline with trailing spaces   \n\n\n\ninner blank lines kept\n",
        ];
        for input in inputs {
            let once = render_notes_md(&parse_notes_md(input));
            let twice = render_notes_md(&parse_notes_md(&once));
            assert_eq!(once, twice, "input: {input:?}");
        }
        // Documents built in the app round-trip after normalization.
        let built = doc(&[("  multi\nline  ", false), ("", true)], &["\n\nnote\n\n", "  ", "a\n\nb"]);
        let parsed = parse_notes_md(&render_notes_md(&built));
        assert_eq!(parsed, doc(&[("multi line", false)], &["note", "a\n\nb"]));
    }

    /// `["Title\n---\nbody", "## Todo\n- [ ] x"]` came back as notes `["Title", "body"]`
    /// plus a task, and the merge then wrote the mangled form back into the app.
    #[test]
    fn notes_with_separator_and_heading_lines_round_trip() {
        let cases: &[&[&str]] = &[
            &["Title\n---\nbody", "## Todo\n- [ ] x"],
            &["front matter?\n---\ntitle: x\n---\nrest"],
            &["a\n## Decisions\nb", "## Decisions\nkept as a heading"],
            &["## Notes\nnot a section", "## Next steps\n- [ ] not a task"],
            &["<!-- Synced with Wormhole. Tasks are ... -->\nkept"],
            &["```\nunclosed fence", "the note after it", "---"],
            &["\\---\nliteral backslash", "\\\\## Todo"],
            &["  ---  \nindented rule", "# Title-like line\nbody"],
        ];
        for notes in cases {
            let built = doc(&[("a task", false)], notes);
            let text = render_notes_md(&built);
            assert_eq!(parse_notes_md(&text), normalize(&built), "{notes:?} rendered as\n{text}");
            assert_eq!(render_notes_md(&parse_notes_md(&text)), text, "stable: {notes:?}");
        }
    }

    /// Plain Markdown stays plain: only lines that would be misread get a `\`.
    #[test]
    fn escaping_is_minimal_and_markdown_friendly() {
        let built = doc(&[], &["## Heading first\ntext\n```\n---\n```", "x\n---\ny"]);
        let text = render_notes_md(&built);
        assert!(text.contains("\n## Heading first\ntext\n```\n---\n```\n"), "{text}");
        assert!(text.contains("\nx\n\\---\ny\n"), "{text}");
    }

    /// Small deterministic xorshift, so the property test needs no extra crate.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
            items[(self.next() % items.len() as u64) as usize]
        }
    }

    /// `parse(render(doc)) == normalize(doc)` for notes made of the lines the parser treats
    /// specially (separators, section headings, the sync marker, fences, backslashes).
    #[test]
    fn round_trip_property() {
        const LINES: &[&str] = &[
            "---",
            "-----",
            "  ---",
            "## Todo",
            "## Next steps",
            "## notes",
            "## Notes:",
            "## Custom",
            "# Title",
            "```",
            "~~~",
            "````rust",
            "<!-- Synced with Wormhole. -->",
            "\\---",
            "\\\\## Todo",
            "\\",
            "\\x",
            "- [ ] looks like a task",
            "* [x] done-ish",
            "text",
            "",
            "   ",
            "  indented",
            "\tab",
            "<!-- other comment -->",
        ];
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        for case in 0..3000 {
            let notes: Vec<String> = (0..rng.next() % 4)
                .map(|_| (0..1 + rng.next() % 6).map(|_| rng.pick(LINES)).collect::<Vec<_>>().join("\n"))
                .collect();
            let tasks: Vec<TaskItem> = (0..rng.next() % 3)
                .map(|_| TaskItem { text: rng.pick(LINES).into(), done: rng.next().is_multiple_of(2) })
                .collect();
            let built = NotesDoc { tasks, notes };
            let text = render_notes_md(&built);
            assert_eq!(parse_notes_md(&text), normalize(&built), "case {case}: {built:?}\n{text}");
        }
    }
}
