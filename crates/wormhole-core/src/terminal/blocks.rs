//! Command blocks: each prompt with its command line and output, as a shell with integration
//! (OSC 133) divides them.
//!
//! Where a block is comes from the grid (`marks`: the shell's marks on the cells), so it moves
//! with the text through scrolling, reflow and the history filling up. What the grid can't hold,
//! when the command ran and how it ended, the PTY thread writes into a [`CommandLog`] as the
//! marks arrive, keyed by the prompt's number, which is also the cells' id. The two meet in
//! [`Block`]. A row without a mark (a blank line) belongs to the block above it; the blank rows
//! under the last block, on the live screen, belong to none.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use wormhole_vt::types::{Column, Line, Point, TermMode};

use super::marks::{self, Part, RowMark};
use super::osc::{BODY_MAX_CHARS, Mark, clean};
use super::screen::Screen;

/// How many commands the log remembers. A block older than that (a prompt pressed Enter at tens
/// of thousands of times ago) reads as finished without a code or a time.
const LOG_MAX: usize = 32_768;

/// Where a block's command stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockState {
    /// The shell is at this prompt; nothing has run.
    Prompt,
    Running {
        since: Instant,
    },
    /// The command finished (`None` code: the shell gave none, as after an empty line, where
    /// nothing ran and `took` is `None` too).
    Finished {
        code: Option<i32>,
        took: Option<Duration>,
    },
}

impl BlockState {
    /// Whether the command failed (a non-zero exit code).
    pub fn failed(&self) -> bool {
        matches!(self, BlockState::Finished { code: Some(code), .. } if *code != 0)
    }

    /// Whether a command ran here: the shell reported it started or gave an exit code.
    pub fn ran(&self) -> bool {
        match self {
            BlockState::Prompt => false,
            BlockState::Running { .. } => true,
            BlockState::Finished { code, took } => code.is_some() || took.is_some(),
        }
    }
}

/// What the PTY thread knows about each command: when it started and how it ended.
#[derive(Debug, Default)]
pub(super) struct CommandLog {
    /// By prompt number, ascending; at most [`LOG_MAX`].
    records: VecDeque<Record>,
}

#[derive(Debug)]
struct Record {
    prompt: u64,
    started: Option<Instant>,
    finished: Option<Instant>,
    code: Option<i32>,
}

impl CommandLog {
    /// A mark of the `prompt`-th prompt arrived (0: before any prompt, which the log ignores).
    pub(super) fn mark(&mut self, mark: Mark, prompt: u64, now: Instant) {
        if prompt == 0 {
            return;
        }
        if self.records.back().is_none_or(|last| last.prompt < prompt) {
            self.records.push_back(Record { prompt, started: None, finished: None, code: None });
            if self.records.len() > LOG_MAX {
                self.records.pop_front();
            }
        }
        let Some(record) = self.records.iter_mut().rev().find(|r| r.prompt == prompt) else { return };
        match mark {
            Mark::PromptStart(_) | Mark::CommandStart => {}
            Mark::OutputStart => record.started = Some(now),
            Mark::Finished(code) => {
                record.finished = Some(now);
                record.code = code;
            }
        }
    }

    /// Where the `prompt`-th prompt's command stands.
    pub(super) fn state(&self, prompt: u64) -> BlockState {
        let at = self.records.partition_point(|r| r.prompt < prompt);
        match self.records.get(at).filter(|r| r.prompt == prompt) {
            Some(Record { started: Some(started), finished: Some(finished), code, .. }) => {
                BlockState::Finished { code: *code, took: Some(finished.saturating_duration_since(*started)) }
            }
            Some(Record { finished: Some(_), code, .. }) => BlockState::Finished { code: *code, took: None },
            Some(Record { started: Some(since), .. }) => BlockState::Running { since: *since },
            Some(_) => BlockState::Prompt,
            None => BlockState::Finished { code: None, took: None },
        }
    }
}

/// One command block: where it is on the grid and how its command went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The prompt's number: the block's id for as long as its cells are on the grid.
    pub id: u64,
    /// Its first line: the prompt's.
    pub start: Line,
    /// The first line of its output, when a command ran and left room for any.
    pub output: Option<Line>,
    /// Its last line that holds something of the block.
    pub end: Line,
    /// The command line as the shell echoed it, lines of a multi-line command joined by `\n`.
    pub command: String,
    pub state: BlockState,
}

impl Block {
    /// The output's lines `[first, last]`, if there is any output.
    pub fn output_lines(&self) -> Option<(Line, Line)> {
        self.output.filter(|first| *first <= self.end).map(|first| (first, self.end))
    }
}

/// Every block on the grid, oldest first; none while a full-screen program has the screen.
pub(super) fn blocks(screen: &Screen, log: &CommandLog) -> Vec<Block> {
    let mut out = Vec::new();
    for start in marks::prompt_lines(screen) {
        if let Some(block) = block_from(screen, start, log) {
            out.push(block);
        }
    }
    out
}

/// The block whose text `line` is in, if any.
pub(super) fn block_at(screen: &Screen, line: Line, log: &CommandLog) -> Option<Block> {
    if screen.mode().contains(TermMode::ALT_SCREEN) {
        return None;
    }
    // The nearest marked row at or above `line` says which block, and the block's prompt is the
    // nearest prompt start at or above that.
    let mut at = line.min(screen.bottommost_line());
    let id = loop {
        if at < screen.topmost_line() {
            return None;
        }
        if let Some(mark) = marks::mark_at(screen, at) {
            break mark.id;
        }
        at -= 1;
    };
    let start = marks::prompt_line(screen, at + 1, marks::Direction::Up)?;
    let block = block_from(screen, start, log)?;
    (block.id == id && block.end >= line).then_some(block)
}

/// The block with this id, if its cells are still on the grid.
pub(super) fn block(screen: &Screen, id: u64, log: &CommandLog) -> Option<Block> {
    if screen.mode().contains(TermMode::ALT_SCREEN) {
        return None;
    }
    // Newer blocks are nearer the bottom: the search starts there.
    let mut line = screen.bottommost_line();
    while line >= screen.topmost_line() {
        match marks::mark_at(screen, line) {
            Some(mark) if mark.id == id => {
                let start = marks::prompt_line(screen, line + 1, marks::Direction::Up)?;
                return block_from(screen, start, log).filter(|block| block.id == id);
            }
            Some(mark) if mark.id < id => return None,
            _ => line -= 1,
        }
    }
    None
}

/// The block whose prompt starts on `start`.
fn block_from(screen: &Screen, start: Line, log: &CommandLog) -> Option<Block> {
    let first = marks::mark_at(screen, start).filter(|mark| mark.prompt)?;
    let id = first.id;
    let bottom = screen.bottommost_line();
    let mut end = start;
    let mut last_command_line = start;
    let mut command = String::new();
    let mut wrapped = false;
    let mut line = start;
    while line <= bottom {
        let mark = if line == start { Some(first) } else { marks::mark_at(screen, line) };
        match mark {
            Some(RowMark { id: other, .. }) if other != id => break,
            Some(mark) => {
                end = line;
                if mark.is_command_line() {
                    last_command_line = line;
                }
                if mark.command {
                    if !command.is_empty() && !wrapped {
                        command.push('\n');
                    }
                    command.push_str(screen.part_text(line, id, Part::Command).trim_end());
                }
            }
            None => {}
        }
        wrapped = screen.wraps(line);
        line += 1;
    }
    let output = (last_command_line < end).then_some(last_command_line + 1);
    Some(Block { id, start, output, end, command: command.trim_end().to_string(), state: log.state(id) })
}

/// What a command that has just finished with a failing exit code left, for telling about it
/// outside the grid (a banner, a row): the command line, how long it ran and the last thing it
/// printed. Text from the program, so cleaned like its notifications are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ended {
    pub command: String,
    pub took: Option<Duration>,
    /// The last non-blank line of its output, if it printed any and it is still on the grid.
    pub last_line: Option<String>,
}

/// How many lines at the end of a command's output are looked through for its last line.
const LAST_LINE_WINDOW: i32 = 40;

/// The command that most recently finished with exit code `code`, if its block is on the grid.
/// The shell has printed its next prompt by the time anyone asks, so the finished block is the
/// last one or the one before it.
pub(super) fn ended(screen: &Screen, log: &CommandLog, code: i32) -> Option<Ended> {
    if screen.mode().contains(TermMode::ALT_SCREEN) {
        return None;
    }
    let mut at = screen.bottommost_line();
    let newest = loop {
        if at < screen.topmost_line() {
            return None;
        }
        if let Some(mark) = marks::mark_at(screen, at) {
            break mark.id;
        }
        at -= 1;
    };
    let block = [newest, newest.saturating_sub(1)]
        .into_iter()
        .filter_map(|id| block(screen, id, log))
        .find(|block| matches!(block.state, BlockState::Finished { code: Some(c), .. } if c == code))?;
    let took = match block.state {
        BlockState::Finished { took, .. } => took,
        _ => None,
    };
    let last_line = block.output_lines().and_then(|(first, last)| {
        let first = first.max(Line(last.0 - LAST_LINE_WINDOW));
        let text = lines_text(screen, first, last);
        let line =
            text.lines().rev().map(|line| clean(line, BODY_MAX_CHARS)).find(|line| !line.is_empty())?;
        Some(line)
    });
    Some(Ended { command: clean(&block.command, BODY_MAX_CHARS), took, last_line })
}

/// The text of lines `first..=last`, trailing blanks trimmed, for copying a block's output (an
/// image's placeholders aren't text).
pub(super) fn lines_text(screen: &Screen, first: Line, last: Line) -> String {
    let start = Point::new(first.max(screen.topmost_line()), Column(0));
    let end = Point::new(last.min(screen.bottommost_line()), screen.last_column());
    let text = screen.text_between(start, end);
    super::images::strip_placeholders(&text).trim_end().to_string()
}

/// How long a command took, as the readout says it: "0.4s", "3.5s", "12s", "1m 5s", "2h 3m".
pub fn took_text(took: Duration) -> String {
    let secs = took.as_secs();
    match secs {
        0..10 => format!("{:.1}s", took.as_secs_f64()),
        10..60 => format!("{secs}s"),
        60..3600 => format!("{}m {}s", secs / 60, secs % 60),
        _ => format!("{}h {}m", secs / 3600, secs % 3600 / 60),
    }
}

/// What the readout beside a finished block says: "exit 101 · 3.5s", "exit 0 · 12s", "3.5s"
/// when the shell gave no code; nothing for a block whose command hasn't finished.
pub fn readout_text(state: BlockState) -> Option<String> {
    let BlockState::Finished { code, took } = state else { return None };
    let parts: Vec<String> =
        [code.map(|code| format!("exit {code}")), took.map(took_text)].into_iter().flatten().collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

#[cfg(test)]
pub(super) mod tests {
    use std::time::{Duration, Instant};

    use wormhole_vt::types::{Line, Scroll};

    use super::{
        BlockState, CommandLog, Ended, block, block_at, blocks, ended, lines_text, readout_text, took_text,
    };
    use crate::terminal::osc::{Mark, PromptKind};
    use crate::terminal::testing::TestTerm;

    /// A terminal `rows` tall that has shown `bytes`, and the log the marks left.
    pub(in crate::terminal) fn shown(rows: u16, bytes: &[u8]) -> (TestTerm, CommandLog) {
        let mut term = TestTerm::new(20, rows, 100);
        let mut log = CommandLog::default();
        let now = Instant::now();
        for (mark, prompt) in term.feed_with_marks(bytes) {
            log.mark(mark, prompt, now);
        }
        (term, log)
    }

    /// Two commands as a shell with integration shows them: `ls` (two lines of output, exit 0)
    /// and `make` (one line, exit 2), then the prompt again.
    pub(in crate::terminal) const SESSION: &[u8] =
        b"\x1b]133;A\x07$ \x1b]133;B\x07ls\r\n\x1b]133;C\x07a.rs\r\nb.rs\r\n\x1b]133;D;0\x07\
        \x1b]133;A\x07$ \x1b]133;B\x07make\r\n\x1b]133;C\x07error\r\n\r\n\x1b]133;D;2\x07\
        \x1b]133;A\x07$ \x1b]133;B\x07";

    #[test]
    fn blocks_are_read_back_from_the_grid_with_their_states() {
        let (term, log) = shown(10, SESSION);
        let found = blocks(term.screen(), &log);
        let summary: Vec<(u64, i32, Option<i32>, i32, &str)> = found
            .iter()
            .map(|b| (b.id, b.start.0, b.output.map(|l| l.0), b.end.0, b.command.as_str()))
            .collect();
        assert_eq!(summary, [(1, 0, Some(1), 2, "ls"), (2, 3, Some(4), 4, "make"), (3, 6, None, 6, "")]);
        assert!(matches!(found[0].state, BlockState::Finished { code: Some(0), took: Some(_) }));
        assert!(found[1].state.failed());
        assert_eq!(found[2].state, BlockState::Prompt);
        assert_eq!(found[1].output_lines(), Some((Line(4), Line(4))), "the blank line after is nobody's");
        assert_eq!(lines_text(term.screen(), Line(1), Line(2)), "a.rs\nb.rs");
        // By a line in it, and by id.
        assert_eq!(block_at(term.screen(), Line(2), &log).map(|b| b.id), Some(1));
        assert_eq!(block_at(term.screen(), Line(5), &log), None, "the blank row under a finished block");
        assert_eq!(block_at(term.screen(), Line(6), &log).map(|b| b.id), Some(3));
        assert_eq!(block_at(term.screen(), Line(9), &log), None, "below the last block");
        assert_eq!(block(term.screen(), 2, &log).map(|b| (b.start.0, b.end.0)), Some((3, 4)));
        assert_eq!(block(term.screen(), 7, &log), None);
    }

    #[test]
    fn a_command_that_just_failed_is_read_back_with_its_last_line() {
        // `make` exited 2 and printed "error" (and a blank line); the prompt after it is the newest
        // block, so the failed one is the one before.
        let (term, log) = shown(10, SESSION);
        let found = ended(term.screen(), &log, 2).expect("the failed block is on the grid");
        assert_eq!(
            found,
            Ended { command: "make".into(), took: found.took, last_line: Some("error".into()) }
        );
        assert!(found.took.is_some());
        // Another code is another command; `ls` (exit 0) is two blocks back, further than a command
        // that just finished can be.
        assert_eq!(ended(term.screen(), &log, 3), None);
        assert_eq!(ended(term.screen(), &log, 0), None);
    }

    #[test]
    fn what_a_command_printed_is_cleaned_and_only_the_end_of_it_is_read() {
        let mut session = b"\x1b]133;A\x07$ \x1b]133;B\x07build\r\n\x1b]133;C\x07".to_vec();
        for n in 0..60 {
            session.extend_from_slice(format!("line {n}\r\n").as_bytes());
        }
        // Direction overrides and the like are not shown; the last line that has anything in it.
        session.extend_from_slice("boom \u{202e}here\r\n   \r\n".as_bytes());
        session.extend_from_slice(b"\x1b]133;D;101\x07\x1b]133;A\x07$ \x1b]133;B\x07");
        let (term, log) = shown(10, &session);
        let found = ended(term.screen(), &log, 101).expect("on the grid");
        assert_eq!(found.last_line.as_deref(), Some("boom here"));
        // A command that printed nothing has no last line.
        let (term, log) = shown(
            10,
            b"\x1b]133;A\x07$ \x1b]133;B\x07false\r\n\x1b]133;C\x07\x1b]133;D;1\x07\x1b]133;A\x07$ \x1b]133;B\x07",
        );
        let found = ended(term.screen(), &log, 1).expect("on the grid");
        assert_eq!((found.command.as_str(), found.last_line), ("false", None));
    }

    #[test]
    fn a_full_screen_program_or_a_forgotten_block_has_nothing_to_read() {
        let (term, log) = shown(10, b"\x1b[?1049h\x1b]133;A\x07$ ");
        assert_eq!(ended(term.screen(), &log, 1), None, "the alternate screen has no blocks");
        let (term, log) = shown(10, b"plain text, no marks at all");
        assert_eq!(ended(term.screen(), &log, 1), None);
    }

    #[test]
    fn a_multi_line_command_and_a_wrapped_one_read_as_one_command() {
        let input = b"\x1b]133;A\x07$ \x1b]133;B\x07for x in a b\r\n\x1b]133;A;k=s\x07> \x1b]133;B\x07echo $x\r\n\x1b]133;A;k=s\x07> \x1b]133;B\x07done\r\n\x1b]133;C\x07a\r\nb\r\n\x1b]133;D;0\x07\
            \x1b]133;A\x07$ \x1b]133;B\x07echo 0123456789abcdefghij\r\n\x1b]133;C\x070123456789abcdefghij\r\n\x1b]133;D;0\x07";
        let (term, log) = shown(12, input);
        let found = blocks(term.screen(), &log);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].command, "for x in a b\necho $x\ndone");
        assert_eq!((found[0].start.0, found[0].output.map(|l| l.0), found[0].end.0), (0, Some(3), 4));
        // The second command wraps onto a second row (20 columns).
        assert_eq!(found[1].command, "echo 0123456789abcdefghij");
        assert_eq!((found[1].start.0, found[1].output.map(|l| l.0), found[1].end.0), (5, Some(7), 7));
    }

    #[test]
    fn blocks_follow_their_text_into_the_history_and_none_show_on_the_alternate_screen() {
        let (mut term, log) = shown(4, SESSION);
        // Seven lines on four rows: three are in the history.
        let found = blocks(term.screen(), &log);
        assert_eq!(found.iter().map(|b| b.start.0).collect::<Vec<_>>(), [-3, 0, 3]);
        assert_eq!(block_at(term.screen(), Line(-2), &log).map(|b| b.id), Some(1));
        term.scroll_display(Scroll::Delta(2));
        assert_eq!(blocks(term.screen(), &log).len(), 3, "scrolling changes nothing");
        term.feed(b"\x1b[?1049hvim");
        assert!(blocks(term.screen(), &log).is_empty());
        assert_eq!(block_at(term.screen(), Line(0), &log), None);
    }

    #[test]
    fn the_log_follows_each_command_from_its_prompt_to_its_exit() {
        let mut log = CommandLog::default();
        let t0 = Instant::now();
        let at = |secs| t0 + Duration::from_secs(secs);
        // The shell's first `D` comes before any prompt and means nothing.
        log.mark(Mark::Finished(Some(0)), 0, at(0));
        log.mark(Mark::PromptStart(PromptKind::Initial), 1, at(1));
        assert_eq!(log.state(1), BlockState::Prompt);
        log.mark(Mark::CommandStart, 1, at(1));
        log.mark(Mark::OutputStart, 1, at(2));
        assert_eq!(log.state(1), BlockState::Running { since: at(2) });
        log.mark(Mark::Finished(Some(3)), 1, at(5));
        assert_eq!(log.state(1), BlockState::Finished { code: Some(3), took: Some(Duration::from_secs(3)) });
        assert!(log.state(1).failed() && log.state(1).ran());
        // An empty line: `D` without a code and without a `C`.
        log.mark(Mark::PromptStart(PromptKind::Initial), 2, at(6));
        log.mark(Mark::Finished(None), 2, at(6));
        assert_eq!(log.state(2), BlockState::Finished { code: None, took: None });
        assert!(!log.state(2).ran() && !log.state(2).failed());
        // A shell that skips `C`: the code without a time.
        log.mark(Mark::PromptStart(PromptKind::Initial), 3, at(7));
        log.mark(Mark::Finished(Some(0)), 3, at(9));
        assert_eq!(log.state(3), BlockState::Finished { code: Some(0), took: None });
        assert!(log.state(3).ran());
        // A secondary prompt adds no record.
        log.mark(Mark::PromptStart(PromptKind::Initial), 4, at(10));
        log.mark(Mark::PromptStart(PromptKind::Secondary), 4, at(10));
        assert_eq!(log.records.len(), 4);
        assert_eq!(log.state(9), BlockState::Finished { code: None, took: None }, "unknown reads as nothing");
    }

    #[test]
    fn the_log_is_bounded() {
        let mut log = CommandLog::default();
        let now = Instant::now();
        for prompt in 1..=(super::LOG_MAX as u64 + 10) {
            log.mark(Mark::PromptStart(PromptKind::Initial), prompt, now);
            log.mark(Mark::OutputStart, prompt, now);
        }
        assert_eq!(log.records.len(), super::LOG_MAX);
        assert_eq!(log.state(1), BlockState::Finished { code: None, took: None }, "forgotten");
        assert_eq!(log.state(super::LOG_MAX as u64 + 10), BlockState::Running { since: now });
    }

    #[test]
    fn times_and_readouts_read_as_people_write_them() {
        let secs = Duration::from_secs;
        assert_eq!(took_text(Duration::from_millis(420)), "0.4s");
        assert_eq!(took_text(Duration::from_millis(3480)), "3.5s");
        assert_eq!(took_text(secs(12)), "12s");
        assert_eq!(took_text(secs(65)), "1m 5s");
        assert_eq!(took_text(secs(2 * 3600 + 3 * 60 + 40)), "2h 3m");
        let finished = |code, took| BlockState::Finished { code, took };
        assert_eq!(readout_text(finished(Some(101), Some(secs(3)))).as_deref(), Some("exit 101 · 3.0s"));
        assert_eq!(readout_text(finished(Some(0), None)).as_deref(), Some("exit 0"));
        assert_eq!(readout_text(finished(None, Some(secs(1)))).as_deref(), Some("1.0s"));
        assert_eq!(readout_text(finished(None, None)), None);
        assert_eq!(readout_text(BlockState::Running { since: Instant::now() }), None);
        assert_eq!(readout_text(BlockState::Prompt), None);
    }
}
