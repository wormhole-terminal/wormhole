//! A window drawn as a sketch on a character grid: the sidebar's rows, the title row's parts, the
//! terminal's lines, the minimap along its edge, the right panel with a tab for each tool, the lines
//! between the columns and the status bar, in box-drawing characters. It is what the walk's answers are
//! shown as in `Wormhole welcome`, as the intro window's `style::window_sketch` is there, and it reads the
//! same [`Shown`]: the real title row's parts, whether the lines show, how many tabs.
//!
//! The sketch is pure: [`sketch`] says which character goes in which cell and what that cell *is*
//! ([`Part`]); the page decides how each part is coloured. The columns keep their places whatever is
//! shown (the lines between them take their cell either way), so two answers differ only where the
//! answer does.

use crate::config::feature_table::FeatureId;
use crate::config::presets::{Shown, ToolsShown};
use crate::view::layout::title_parts::PartKind;

/// The smallest sketch that still shows a window: room for the sidebar and a terminal beside it, and a
/// title row, a line of text and the status bar below each other.
pub(super) const MIN_COLS: usize = 14;
pub(super) const MIN_ROWS: usize = 5;

/// The sidebar's width inside the frame, and the right panel's (four tabs and a cell of room each side).
const SIDEBAR: usize = 3;
const PANEL: usize = 6;
/// The panel's tabs: the two that are always there, and the most it draws.
const TABS_ALWAYS: usize = 2;
const TABS_MAX: usize = 4;
/// The terminal's lines of text, as shares of the line's width in tenths.
const LINES: [usize; 4] = [8, 5, 7, 3];
/// The sidebar's rows.
const ROWS: usize = 3;

/// What a cell of the sketch is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Blank,
    /// The window's outline.
    Frame,
    /// A line between two columns.
    Rule,
    /// A sidebar row, and the one for the current terminal.
    Row,
    CurrentRow,
    /// A crumb of the title row (folder, branch, terminal).
    Crumb,
    /// The rest of the title row: its marks, chips and buttons.
    Mark,
    /// The terminal's prompt and its lines of text.
    Prompt,
    Text,
    Minimap,
    /// The status bar along the bottom.
    Status,
    /// A tab of the right panel, and the current one.
    Tab,
    CurrentTab,
}

/// A sketch: `rows` by `cols` cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Sketch {
    cols: usize,
    rows: usize,
    cells: Vec<(char, Part)>,
}

impl Sketch {
    fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows, cells: vec![(' ', Part::Blank); cols * rows] }
    }

    pub(super) fn rows(&self) -> usize {
        self.rows
    }

    /// Row `row`'s cells, left to right.
    pub(super) fn row(&self, row: usize) -> &[(char, Part)] {
        &self.cells[row * self.cols..(row + 1) * self.cols]
    }

    /// A cell inside the frame, `x` and `y` counted from its inside; a cell outside the sketch is left
    /// alone, so a part that does not fit is simply cut.
    fn put(&mut self, x: usize, y: usize, ch: char, part: Part) {
        if x < self.cols.saturating_sub(2) && y < self.rows.saturating_sub(2) {
            self.cells[(y + 1) * self.cols + x + 1] = (ch, part);
        }
    }

    fn bar(&mut self, x: usize, y: usize, len: usize, ch: char, part: Part) {
        for at in x..x + len {
            self.put(at, y, ch, part);
        }
    }

    /// The sketch as plain text, one string per row (what the tests compare).
    #[cfg(test)]
    pub(super) fn text(&self) -> Vec<String> {
        (0..self.rows).map(|row| self.row(row).iter().map(|(ch, _)| *ch).collect()).collect()
    }

    #[cfg(test)]
    pub(super) fn has(&self, part: Part) -> bool {
        self.cells.iter().any(|(_, p)| *p == part)
    }
}

/// The window `shown` makes, `cols` by `rows` cells including its frame. `offered` is the optional tools
/// this build has, so that "all of them" draws as many tabs and a tool of the person's own that is not
/// among them, or is always there, draws none. `None` below
/// [`MIN_COLS`] by [`MIN_ROWS`], where there is no room to show a window.
pub(super) fn sketch(shown: &Shown, offered: &[FeatureId], cols: usize, rows: usize) -> Option<Sketch> {
    if cols < MIN_COLS || rows < MIN_ROWS {
        return None;
    }
    let mut out = Sketch::new(cols, rows);
    frame(&mut out);
    let (inside_w, inside_h) = (cols - 2, rows - 2);
    let panel = shown.panel_open.then_some(PANEL);
    // The terminal sits between the sidebar's line and the panel's, which keep their cells whether or not
    // the line shows.
    let term_from = SIDEBAR + 1;
    let term_to = inside_w - panel.map_or(0, |width| width + 1);
    let body_to = inside_h - usize::from(shown.status_bar);

    sidebar(&mut out, body_to);
    let rule = if shown.column_dividers { ('│', Part::Rule) } else { (' ', Part::Blank) };
    for y in 0..body_to {
        out.put(SIDEBAR, y, rule.0, rule.1);
        if panel.is_some() {
            out.put(term_to, y, rule.0, rule.1);
        }
    }
    title_row(&mut out, shown, term_from, term_to);
    terminal(&mut out, term_from, term_to, body_to, shown.minimap);
    if let Some(width) = panel {
        right_panel(&mut out, term_to + 1, width, body_to, panel_tabs(shown, offered));
    }
    if shown.status_bar {
        status_bar(&mut out, inside_w, inside_h - 1);
    }
    Some(out)
}

fn frame(out: &mut Sketch) {
    let (cols, rows) = (out.cols, out.rows);
    let mut set = |col: usize, row: usize, ch: char| out.cells[row * cols + col] = (ch, Part::Frame);
    for col in 1..cols - 1 {
        set(col, 0, '─');
        set(col, rows - 1, '─');
    }
    for row in 1..rows - 1 {
        set(0, row, '│');
        set(cols - 1, row, '│');
    }
    set(0, 0, '╭');
    set(cols - 1, 0, '╮');
    set(0, rows - 1, '╰');
    set(cols - 1, rows - 1, '╯');
}

/// The sidebar's rows under its header, the first the current terminal.
fn sidebar(out: &mut Sketch, body_to: usize) {
    for row in 0..ROWS {
        let y = row + 1;
        if y >= body_to {
            break;
        }
        let part = if row == 0 { Part::CurrentRow } else { Part::Row };
        out.put(1, y, '▪', part);
        out.put(2, y, '━', part);
    }
}

/// How many tabs the panel draws: the two that are always there and one for each optional tool.
fn panel_tabs(shown: &Shown, offered: &[FeatureId]) -> usize {
    let more = match &shown.tools {
        ToolsShown::Few => 0,
        ToolsShown::All => offered.len(),
        ToolsShown::Own(on) => offered.iter().filter(|id| on.contains(id)).count(),
    };
    (TABS_ALWAYS + more).min(TABS_MAX)
}

fn right_panel(out: &mut Sketch, from: usize, width: usize, body_to: usize, tabs: usize) {
    for tab in 0..tabs.min(width.saturating_sub(2)) {
        out.put(from + 1 + tab, 0, '▪', if tab == 0 { Part::CurrentTab } else { Part::Tab });
    }
    for (line, share) in [3, 2].into_iter().enumerate() {
        let y = line + 1;
        if y < body_to {
            out.bar(from + 1, y, share.min(width.saturating_sub(2)), '━', Part::Text);
        }
    }
}

/// The prompt and the lines of text under the title row, and the minimap along the terminal's edge.
fn terminal(out: &mut Sketch, from: usize, to: usize, body_to: usize, minimap: bool) {
    let edge = if minimap && to > from + 5 { to - 1 } else { to };
    if edge < to {
        for y in 1..body_to {
            out.put(edge, y, '░', Part::Minimap);
        }
    }
    let room = edge.saturating_sub(from + 2);
    for (line, tenths) in LINES.iter().enumerate() {
        let y = line + 1;
        if y >= body_to || room == 0 {
            break;
        }
        let len = (room * tenths / 10).max(1);
        if line == 0 {
            out.put(from + 1, y, '$', Part::Prompt);
            out.bar(from + 3, y, len.saturating_sub(2), '━', Part::Prompt);
        } else {
            out.bar(from + 1, y, len, '━', Part::Text);
        }
    }
}

fn status_bar(out: &mut Sketch, width: usize, y: usize) {
    out.bar(0, y, width, ' ', Part::Status);
    out.bar(1, y, 3, '━', Part::Status);
    out.bar(width.saturating_sub(3), y, 2, '━', Part::Status);
}

// ------------------------------------------------------------------------------------- the title row

/// How wide each kind of part is drawn, and whether the parts are kept apart by a cell.
#[derive(Clone, Copy)]
struct Metrics {
    crumb: usize,
    chip: usize,
    gaps: bool,
}

/// From the roomiest drawing to the tightest; the first that fits is used. After these, the marks that
/// only decorate a crumb (the dot after a branch, the status after a terminal) are left out, and last of
/// all the ends of the row are cut.
const LEVELS: [Metrics; 3] = [
    Metrics { crumb: 3, chip: 2, gaps: true },
    Metrics { crumb: 2, chip: 1, gaps: true },
    Metrics { crumb: 1, chip: 1, gaps: false },
];

fn width(kind: PartKind, metrics: Metrics) -> usize {
    match kind {
        PartKind::Crumb => metrics.crumb,
        PartKind::Chip => metrics.chip,
        PartKind::Inline | PartKind::Button => 1,
        PartKind::Space => 0,
    }
}

/// The room a group of parts takes: their widths, and a cell between two of them unless the second is a
/// mark of the first's.
fn extent(group: &[PartKind], metrics: Metrics) -> usize {
    let mut taken = 0;
    for (ix, kind) in group.iter().enumerate() {
        if ix > 0 && metrics.gaps && *kind != PartKind::Inline {
            taken += 1;
        }
        taken += width(*kind, metrics);
    }
    taken
}

/// The parts before the row's space and after it.
fn ends(parts: &[PartKind]) -> (Vec<PartKind>, Vec<PartKind>) {
    let at = parts.iter().position(|kind| *kind == PartKind::Space).unwrap_or(parts.len());
    let after = parts.get(at + 1..).unwrap_or_default();
    (parts[..at].to_vec(), after.to_vec())
}

fn title_row(out: &mut Sketch, shown: &Shown, from: usize, to: usize) {
    let kinds: Vec<PartKind> = shown.title.row().parts().iter().map(|part| part.kind()).collect();
    let room = to.saturating_sub(from + 2);
    let (mut left, mut right) = ends(&kinds);
    let roomiest = |left: &[PartKind], right: &[PartKind]| {
        LEVELS.iter().copied().find(|metrics| extent(left, *metrics) + 1 + extent(right, *metrics) <= room)
    };
    let metrics = match roomiest(&left, &right) {
        Some(metrics) => metrics,
        None => {
            left.retain(|kind| *kind != PartKind::Inline);
            right.retain(|kind| *kind != PartKind::Inline);
            roomiest(&left, &right).unwrap_or_else(|| {
                // Still too much: the row is cut where it is longest, the ends being what it is for.
                let tightest = LEVELS[LEVELS.len() - 1];
                while extent(&left, tightest) + 1 + extent(&right, tightest) > room
                    && left.len() + right.len() > 1
                {
                    if left.len() > 1 {
                        left.pop();
                    } else {
                        right.remove(0);
                    }
                }
                tightest
            })
        }
    };
    draw_group(out, &left, metrics, from + 1);
    let right_width = extent(&right, metrics);
    draw_group(out, &right, metrics, to.saturating_sub(1 + right_width));
}

fn draw_group(out: &mut Sketch, group: &[PartKind], metrics: Metrics, from: usize) {
    let mut at = from;
    for (ix, kind) in group.iter().enumerate() {
        if ix > 0 && metrics.gaps && *kind != PartKind::Inline {
            at += 1;
        }
        let len = width(*kind, metrics);
        match kind {
            PartKind::Crumb => out.bar(at, 0, len, '━', Part::Crumb),
            PartKind::Inline => out.bar(at, 0, len, '·', Part::Mark),
            PartKind::Chip => out.bar(at, 0, len, '▫', Part::Mark),
            PartKind::Button => out.bar(at, 0, len, '□', Part::Mark),
            PartKind::Space => {}
        }
        at += len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::presets::{Preset, TitleShown};
    use crate::onboarding::tailor::Question;

    fn zen() -> Shown {
        Shown::of(Preset::Zen)
    }

    /// The tools a default build offers, and one it doesn't (Worktrees is always there, with no tab).
    const OFFERED: [FeatureId; 3] = [FeatureId::History, FeatureId::Notes, FeatureId::Project];

    fn drawn(shown: &Shown) -> Sketch {
        sketch(shown, &OFFERED, 24, 8).expect("room")
    }

    #[test]
    fn a_window_is_a_frame_with_the_sidebar_and_a_terminal_in_it() {
        let balanced = drawn(&Shown::of(Preset::Balanced)).text();
        assert_eq!(
            balanced,
            [
                "╭──────────────────────╮",
                "│   │ ━━·━·      ▫▫▫□□ │",
                "│ ▪━│ $ ━━━━━━━━━━     │",
                "│ ▪━│ ━━━━━━━━         │",
                "│ ▪━│ ━━━━━━━━━━━      │",
                "│   │ ━━━━             │",
                "│   │                  │",
                "╰──────────────────────╯",
            ]
        );
    }

    #[test]
    fn zen_is_the_name_and_the_splits_and_nothing_else() {
        let zen = drawn(&zen());
        assert_eq!(zen.text()[1], "│     ━━━            □ │");
        for part in [Part::Rule, Part::Minimap, Part::Status, Part::Tab, Part::Mark] {
            // The splits are the one mark Zen keeps.
            assert_eq!(zen.has(part), part == Part::Mark, "{part:?}");
        }
    }

    #[test]
    fn each_part_of_a_window_is_drawn_where_it_belongs() {
        let everything = drawn(&Shown::of(Preset::Everything));
        for part in [Part::Rule, Part::Minimap, Part::Status, Part::Tab, Part::CurrentTab, Part::Crumb] {
            assert!(everything.has(part), "{part:?}");
        }
        // The status bar is the row above the frame's bottom, across the whole inside.
        assert!(everything.row(6).iter().skip(1).take(22).all(|(_, part)| *part == Part::Status));
        // The minimap is the terminal's right edge, the panel's rule the column before the panel.
        assert!(everything.row(3).iter().any(|(ch, part)| *ch == '░' && *part == Part::Minimap));
        let tabs = everything.row(1).iter().filter(|(_, part)| matches!(part, Part::Tab | Part::CurrentTab));
        assert_eq!(tabs.count(), 4, "two always there and the three optional tools, at most four");
    }

    #[test]
    fn the_right_panel_has_a_tab_for_each_tool() {
        let tabs = |tools: ToolsShown| {
            let shown = Shown { panel_open: true, tools, ..Shown::of(Preset::Balanced) };
            drawn(&shown)
                .row(1)
                .iter()
                .filter(|(_, part)| matches!(part, Part::Tab | Part::CurrentTab))
                .count()
        };
        assert_eq!(tabs(ToolsShown::Few), 2);
        assert_eq!(tabs(ToolsShown::All), 4);
        assert_eq!(tabs(ToolsShown::Own(vec![FeatureId::History])), 3);
        // A tool this build doesn't offer, or one that is always there, is no further tab.
        assert_eq!(
            tabs(ToolsShown::Own(vec![FeatureId::History, FeatureId::Changes, FeatureId::Worktrees])),
            3
        );
    }

    #[test]
    fn the_columns_keep_their_places_whether_or_not_the_lines_show() {
        let with = drawn(&Shown { column_dividers: true, panel_open: true, ..zen() });
        let without = drawn(&Shown { column_dividers: false, panel_open: true, ..zen() });
        let cells = |sketch: &Sketch| -> Vec<Vec<char>> {
            (0..sketch.rows())
                .map(|r| {
                    sketch
                        .row(r)
                        .iter()
                        .map(|(ch, part)| if *part == Part::Rule { ' ' } else { *ch })
                        .collect()
                })
                .collect()
        };
        assert_eq!(cells(&with), cells(&without), "only the lines differ");
    }

    #[test]
    fn a_title_row_wider_than_the_sketch_gives_up_room_part_by_part() {
        let title = |title: TitleShown, panel_open: bool| {
            drawn(&Shown { title, panel_open, ..Shown::of(Preset::Balanced) }).text()[1].clone()
        };
        // Everything has the marks after the crumbs while there is room for them.
        assert!(title(TitleShown::Everything, false).contains('·'));
        // With the panel beside it the row is tight: the marks go, the crumbs and chips stay.
        let tight = title(TitleShown::Everything, true);
        assert!(!tight.contains('·') && tight.contains('▫') && tight.contains('□'), "{tight}");
        assert_ne!(tight, title(TitleShown::Path, true), "and it still differs from the shorter row");
    }

    #[test]
    fn nothing_is_drawn_outside_the_frame_at_any_size() {
        for preset in Preset::ALL {
            let shown = Shown::of(preset);
            for cols in 0..40 {
                for rows in 0..14 {
                    let Some(sketch) = sketch(&shown, &OFFERED, cols, rows) else {
                        assert!(cols < MIN_COLS || rows < MIN_ROWS);
                        continue;
                    };
                    assert_eq!((sketch.row(0).len(), sketch.rows()), (cols, rows));
                    for r in 0..rows {
                        let edge = r == 0 || r == rows - 1;
                        let (first, last) = (sketch.row(r)[0], sketch.row(r)[cols - 1]);
                        assert!(first.1 == Part::Frame && last.1 == Part::Frame, "{preset:?} {cols}x{rows}");
                        assert_eq!(edge, first.0 != '│', "{preset:?} {cols}x{rows} row {r}");
                    }
                }
            }
        }
    }

    /// The cards of a question must look different, or there is nothing to choose by.
    #[test]
    fn no_two_answers_of_a_question_draw_the_same_sketch() {
        for preset in Preset::ALL {
            let shown = Shown::of(preset);
            for question in Question::ALL {
                for (cols, rows) in [(24, 8), (30, 8), (22, 7), (20, 6)] {
                    let sketches: Vec<Sketch> = (0..question.answers().len())
                        .map(|ix| sketch(&question.preview(ix, &shown), &OFFERED, cols, rows).expect("room"))
                        .collect();
                    for (a, first) in sketches.iter().enumerate() {
                        for second in &sketches[a + 1..] {
                            assert_ne!(first, second, "{question:?} from {preset:?} at {cols}x{rows}");
                        }
                    }
                }
            }
        }
    }
}
