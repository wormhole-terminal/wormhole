//! What the terminal's title row gives up when it is narrow:
//! the path losing its middle, and the order the rest of the row gives way in.
//!
//! The row reads folder › branch › the terminal's agent and name, with Open in… and the splits at
//! its right end. Nothing here measures text: the app measures with GPUI's text system and hands
//! the widths in (or a `measure` callback), so the rules are testable without fonts.

use crate::elide;

/// Which text of the path is being measured: the folders before the last one (regular weight) or
/// the last folder, which is the one that says where the terminal is (medium).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Parents,
    Last,
}

/// The path as the row draws it: `parents` in regular weight, `last` in medium.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathText {
    /// `~/…/Icssaddons/`: the start, the `…` that stands for what was left out, and the parents
    /// that were kept, each with its slash.
    pub parents: String,
    /// `Tube`.
    pub last: String,
}

impl PathText {
    pub fn full(&self) -> String {
        format!("{}{}", self.parents, self.last)
    }
}

/// One way of drawing the path, and how wide it is.
#[derive(Clone, Debug, PartialEq)]
pub struct PathStep {
    pub text: PathText,
    pub width: f32,
}

/// The ways to draw `shown` (a path as `display_path` writes it), the widest first: whole, then
/// with parents dropped out of the middle, the ones nearest the start first, down to the start,
/// the `…` and the last folder (`~/…/Tube`). The start (`~`, or the first folder of a path
/// outside the home folder) always stays, and so does the last folder, which is cut in its middle
/// only when it alone is wider than `last_max`.
///
/// `measure` gives the width of a text in the face it is drawn in.
pub fn path_steps(shown: &str, last_max: f32, measure: impl Fn(Face, &str) -> f32) -> Vec<PathStep> {
    let (head, mids, last) = split_path(shown);
    let last = elide::fit(&last, last_max, |text| measure(Face::Last, text), elide::middle).into_owned();
    let last_width = measure(Face::Last, &last);
    (0..=mids.len())
        .rev()
        .map(|kept| {
            let mut parents = head.clone();
            if kept < mids.len() {
                parents.push(elide::ELLIPSIS);
                parents.push('/');
            }
            for name in &mids[mids.len() - kept..] {
                parents.push_str(name);
                parents.push('/');
            }
            let width = measure(Face::Parents, &parents) + last_width;
            PathStep { text: PathText { parents, last: last.clone() }, width }
        })
        .collect()
}

/// A path as its start (with the slash after it), the folders between the start and the last one,
/// and the last. A path of one name (`~`, `/Users`) is all last.
fn split_path(shown: &str) -> (String, Vec<String>, String) {
    let rooted = shown.starts_with('/');
    let mut names: Vec<&str> = shown.split('/').filter(|name| !name.is_empty()).collect();
    let Some(&last) = names.last() else { return (String::new(), Vec::new(), shown.to_string()) };
    let with_root = |name: &str| if rooted { format!("/{name}") } else { name.to_string() };
    if names.len() == 1 {
        return (String::new(), Vec::new(), with_root(last));
    }
    let head = format!("{}/", with_root(names[0]));
    names.remove(0);
    names.pop();
    (head, names.into_iter().map(str::to_string).collect(), last.to_string())
}

/// The branch chip's two widths: whole, and its glyph alone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chip {
    pub natural: f32,
    pub icon: f32,
}

/// What else the row can carry, each on a setting of its own: a dot and the
/// ahead and behind counts after the branch chip, the model and context window of a coding
/// agent, the ports a terminal listens on, and the branch's pull request. None is ever worth the
/// path or the terminal's name, so they give way right after the status words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extra {
    BranchStatus,
    Gauge,
    Ports,
    PullRequest,
}

impl Extra {
    /// The order they go in when the row is narrow, the first to go first: the pull request
    /// (the one that is about somewhere else), the ports, the agent's model and context, and
    /// last the branch's status, which belongs to the chip it follows.
    pub const GIVE_WAY: [Extra; 4] = [Extra::PullRequest, Extra::Ports, Extra::Gauge, Extra::BranchStatus];

    fn slot(self) -> usize {
        self as usize
    }
}

/// How wide each extra is, with the gap that separates it from what is before it; `None` for one
/// the row doesn't have (switched off, or nothing to say).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Extras([Option<f32>; 4]);

impl Extras {
    pub fn with(mut self, extra: Extra, width: Option<f32>) -> Self {
        self.0[extra.slot()] = width;
        self
    }

    pub fn width(&self, extra: Extra) -> Option<f32> {
        self.0[extra.slot()]
    }

    /// Whether the row draws it.
    pub fn shows(&self, extra: Extra) -> bool {
        self.width(extra).is_some()
    }

    fn total(&self) -> f32 {
        self.0.iter().flatten().sum()
    }
}

/// What the row wants, each of the parts that can give way as wide as it likes to be.
#[derive(Clone, Debug, PartialEq)]
pub struct RowWants {
    /// The path's steps (`path_steps`), widest first, each with the gap after the folder icon.
    /// Empty for a terminal without a folder.
    pub path: Vec<f32>,
    /// How the status words ("working · 2m") are drawn with their gap, or 0 for none.
    pub words: f32,
    pub chip: Option<Chip>,
    /// The extras there are to show, each with its gap (`Extra`).
    pub extras: Extras,
    /// The ports can give way a step at a time before they go: each of these is the width of
    /// the ports with one chip fewer than the step before (the others are behind the "+n"),
    /// widest first, so a row a little short of room keeps some of them. Empty when there is
    /// nothing to take out.
    pub ports_narrower: Vec<f32>,
    /// Open in… and the splits, and the one `⋯` button they fold into.
    pub group: f32,
    pub folded: f32,
    /// The terminal's name as it likes to be, and the least worth showing (about three letters).
    pub name: f32,
    pub name_min: f32,
    /// Everything else on the row, which never gives way: the folder icon, the agent's icon and
    /// status, the separators, the gaps and the row's padding.
    pub fixed: f32,
}

/// How the path is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathShown {
    /// No folder.
    None,
    /// The folder's icon alone.
    Icon,
    /// This step of `RowWants::path`.
    Step(usize),
}

/// What the row draws.
#[derive(Clone, Debug, PartialEq)]
pub struct RowFit {
    pub path: PathShown,
    pub words: bool,
    /// The chip's width; its `icon` width is the glyph alone. `None` without a branch.
    pub chip: Option<f32>,
    /// The extras that are drawn.
    pub extras: Extras,
    /// How many of `RowWants::ports_narrower`'s steps the ports took: 0 draws them as they are
    /// first wanted, 2 with two chips fewer.
    pub ports_step: usize,
    /// Open in… and the splits are one `⋯` menu.
    pub folded: bool,
    pub name: f32,
}

/// The row's order of giving way in `avail` points (each step only when the one before did not
/// make the row fit):
///
/// 1. the path loses its middle, down to `~/…/last folder`;
/// 2. the status words go, whole;
/// 3. the extras go, one at a time and whole, in the order of [`Extra::GIVE_WAY`] (the ports first
///    lose their chips one by one, the rest being behind the "+n");
/// 4. the branch chip shrinks to its glyph;
/// 5. Open in… and the splits fold into one `⋯` menu;
/// 6. the name is cut at its end, to a few letters;
/// 7. the path folds to its folder's icon, and the name takes what that frees.
///
/// Never given up: the agent's icon and status, the name's first letters and the `⋯`. The name
/// is left whole through steps 1 to 5, and comes back whole after the fold when it fits.
pub fn fit_row(avail: f32, wants: &RowWants) -> RowFit {
    let mut path = if wants.path.is_empty() { PathShown::None } else { PathShown::Step(0) };
    let mut words = wants.words > 0.0;
    let mut extras = wants.extras;
    let mut chip = wants.chip.map(|chip| chip.natural);
    let mut folded = false;

    // What the row takes besides the name.
    let rest = |path: PathShown, words: bool, extras: Extras, chip: Option<f32>, folded: bool| {
        wants.fixed
            + match path {
                PathShown::Step(ix) => wants.path[ix],
                PathShown::None | PathShown::Icon => 0.0,
            }
            + if words { wants.words } else { 0.0 }
            + extras.total()
            + chip.unwrap_or(0.0)
            + if folded { wants.folded } else { wants.group }
    };
    let fits =
        |path, words, extras, chip, folded| rest(path, words, extras, chip, folded) + wants.name <= avail;

    while let PathShown::Step(ix) = path
        && ix + 1 < wants.path.len()
        && !fits(path, words, extras, chip, folded)
    {
        path = PathShown::Step(ix + 1);
    }
    if !fits(path, words, extras, chip, folded) {
        words = false;
    }
    let mut ports_step = 0;
    for extra in Extra::GIVE_WAY {
        // The ports give up a chip at a time before they go.
        while extra == Extra::Ports
            && extras.shows(extra)
            && ports_step < wants.ports_narrower.len()
            && !fits(path, words, extras, chip, folded)
        {
            extras = extras.with(extra, Some(wants.ports_narrower[ports_step]));
            ports_step += 1;
        }
        if !fits(path, words, extras, chip, folded) {
            extras = extras.with(extra, None);
        }
    }
    if let (Some(width), Some(shape)) = (chip, wants.chip)
        && !fits(path, words, extras, chip, folded)
    {
        let over = rest(path, words, extras, chip, folded) + wants.name - avail;
        chip = Some((width - over).max(shape.icon.min(width)));
    }
    if !fits(path, words, extras, chip, folded) {
        folded = true;
    }

    let floor = wants.name_min.min(wants.name);
    let mut name = wants.name.min(avail - rest(path, words, extras, chip, folded));
    if name < floor && matches!(path, PathShown::Step(_)) {
        path = PathShown::Icon;
        name = wants.name.min(avail - rest(path, words, extras, chip, folded));
    }
    // Whatever is left of the row, which clips the name if even that is nothing.
    let name = name.max(0.0);
    RowFit { path, words, chip, extras, ports_step, folded, name }
}

#[cfg(test)]
mod tests {
    use super::{Chip, Extra, Extras, Face, PathShown, PathText, RowWants, fit_row, path_steps};

    /// One point per character, as a monospace font would measure.
    fn chars(_: Face, text: &str) -> f32 {
        text.chars().count() as f32
    }

    fn texts(shown: &str, last_max: f32) -> Vec<String> {
        path_steps(shown, last_max, chars).into_iter().map(|step| step.text.full()).collect()
    }

    #[test]
    fn a_path_loses_its_middle_from_the_start_side() {
        assert_eq!(
            texts("~/Projects/misc/wormhole/Icssaddons/Tube", 100.0),
            [
                "~/Projects/misc/wormhole/Icssaddons/Tube",
                "~/…/misc/wormhole/Icssaddons/Tube",
                "~/…/wormhole/Icssaddons/Tube",
                "~/…/Icssaddons/Tube",
                "~/…/Tube",
            ]
        );
    }

    #[test]
    fn the_steps_say_which_part_is_the_last_folder() {
        let steps = path_steps("~/a/bb/ccc", 100.0, chars);
        assert_eq!(steps[1].text, PathText { parents: "~/…/bb/".into(), last: "ccc".into() });
        assert_eq!(steps[1].width, 10.0);
        assert_eq!(steps.last().map(|step| step.text.full()), Some("~/…/ccc".to_string()));
    }

    #[test]
    fn a_short_path_has_one_way_to_be_drawn() {
        assert_eq!(texts("~/Tube", 100.0), ["~/Tube"]);
        assert_eq!(texts("~", 100.0), ["~"]);
        assert_eq!(texts("/", 100.0), ["/"]);
        // Two folders: nothing in the middle to leave out.
        assert_eq!(texts("~/Projects/Tube", 100.0), ["~/Projects/Tube", "~/…/Tube"]);
    }

    #[test]
    fn a_path_outside_the_home_folder_keeps_its_first_folder() {
        assert_eq!(
            texts("/Volumes/Work/clients/acme/api", 100.0),
            [
                "/Volumes/Work/clients/acme/api",
                "/Volumes/…/clients/acme/api",
                "/Volumes/…/acme/api",
                "/Volumes/…/api",
            ]
        );
        assert_eq!(texts("/Users", 100.0), ["/Users"]);
    }

    #[test]
    fn a_last_folder_wider_than_its_room_is_cut_in_its_own_middle() {
        let steps = path_steps("~/a/b/a_very_long_folder_name_indeed", 12.0, chars);
        let last = &steps[steps.len() - 1];
        assert_eq!(last.text.last.chars().count(), 12);
        assert!(last.text.last.contains('…') && last.text.last.starts_with("a_ve"));
        // Every step carries the cut name.
        assert!(steps.iter().all(|step| step.text.last == last.text.last));
    }

    fn wants() -> RowWants {
        RowWants {
            // ~/…/aaaa/Tube, ~/…/Tube.
            path: vec![40.0, 30.0, 20.0],
            words: 30.0,
            chip: Some(Chip { natural: 30.0, icon: 10.0 }),
            extras: Extras::default(),
            ports_narrower: Vec::new(),
            group: 100.0,
            folded: 28.0,
            name: 60.0,
            name_min: 20.0,
            fixed: 50.0,
        }
    }

    /// The row with all four extras: a pull request 20, ports 50, the model and context 40, the
    /// branch's status 10.
    fn with_extras() -> RowWants {
        let extras = Extras::default()
            .with(Extra::PullRequest, Some(20.0))
            .with(Extra::Ports, Some(50.0))
            .with(Extra::Gauge, Some(40.0))
            .with(Extra::BranchStatus, Some(10.0));
        RowWants { extras, ..wants() }
    }

    /// All of it: 50 + 40 + 30 + 30 + 100 + 60 = 310.
    const WHOLE: f32 = 310.0;

    #[test]
    fn everything_fits_in_a_wide_row() {
        let fit = fit_row(WHOLE, &wants());
        assert_eq!(
            fit,
            super::RowFit {
                path: PathShown::Step(0),
                words: true,
                chip: Some(30.0),
                extras: Extras::default(),
                ports_step: 0,
                folded: false,
                name: 60.0
            }
        );
        assert_eq!(fit_row(2000.0, &wants()), fit);
    }

    /// Everything and the four extras (120 more): 310 + 120 = 430.
    #[test]
    fn the_extras_are_drawn_while_there_is_room() {
        let wants = with_extras();
        let fit = fit_row(430.0, &wants);
        assert_eq!(fit.extras, wants.extras);
        assert_eq!(
            (fit.path, fit.words, fit.chip, fit.folded, fit.name),
            (PathShown::Step(0), true, Some(30.0), false, 60.0)
        );
        assert_eq!(fit_row(5000.0, &wants), fit);
    }

    /// Right after the status words, one at a time, each only when the row still doesn't fit:
    /// the pull request, the ports, the model and context, the branch's status.
    #[test]
    fn then_the_extras_go_one_by_one_the_least_wanted_first() {
        let wants = with_extras();
        // Path at its shortest, words gone: 430 - 20 (path) - 30 (words) = 380 for everything else.
        let drawn = |avail: f32| {
            let fit = fit_row(avail, &wants);
            let shows = Extra::GIVE_WAY.map(|extra| fit.extras.shows(extra));
            (fit.path, fit.words, shows, fit.chip, fit.folded, fit.name)
        };
        // [pull request, ports, gauge, branch status]
        let (path, words, shows, chip, folded, name) = drawn(380.0);
        assert_eq!((path, words, shows), (PathShown::Step(2), false, [true; 4]));
        assert_eq!((chip, folded, name), (Some(30.0), false, 60.0));
        assert_eq!(drawn(379.0).2, [false, true, true, true], "the pull request first");
        assert_eq!(drawn(360.0).2, [false, true, true, true]);
        assert_eq!(drawn(359.0).2, [false, false, true, true], "then the ports");
        assert_eq!(drawn(310.0).2, [false, false, true, true]);
        assert_eq!(drawn(309.0).2, [false, false, false, true], "then the agent's model and context");
        assert_eq!(drawn(270.0).2, [false, false, false, true]);
        assert_eq!(drawn(269.0).2, [false, false, false, false], "last the branch's status");
        // With none left, the chip shrinks next, as it did before there were extras.
        let (_, _, shows, chip, folded, _) = drawn(255.0);
        assert_eq!((shows, chip.map(|c| c < 30.0), folded), ([false; 4], Some(true), false));
    }

    /// The extras go before the chip shrinks, the right group folds or the name is cut: the path
    /// and the terminal's name are never the price.
    #[test]
    fn the_extras_are_given_up_before_anything_that_matters_more() {
        let wants = with_extras();
        for avail in [269.0, 262.0, 250.0] {
            let fit = fit_row(avail, &wants);
            assert!(!Extra::GIVE_WAY.iter().any(|extra| fit.extras.shows(*extra)), "{avail}");
        }
        // 270 still has the branch's status, and the name, the group and the chip are untouched.
        let fit = fit_row(270.0, &wants);
        assert!(fit.extras.shows(Extra::BranchStatus));
        assert_eq!((fit.chip, fit.folded, fit.name), (Some(30.0), false, 60.0));
    }

    /// A row a little short of room keeps some of the ports: they lose a chip at a time (the others
    /// go behind the "+n"), and only then are they gone.
    #[test]
    fn the_ports_lose_their_chips_one_at_a_time_before_they_go() {
        // Ports 50 wide, 30 with one chip fewer, 15 with two fewer; nothing else but the base row.
        let wants = RowWants {
            extras: Extras::default().with(Extra::Ports, Some(50.0)),
            ports_narrower: vec![30.0, 15.0],
            ..wants()
        };
        // Path at its shortest and no words: 310 - 50 = 260, and the ports on top.
        let at = |avail: f32| {
            let fit = fit_row(avail, &wants);
            (fit.extras.width(Extra::Ports), fit.ports_step)
        };
        assert_eq!(at(310.0), (Some(50.0), 0));
        assert_eq!(at(309.0), (Some(30.0), 1), "one chip fewer");
        assert_eq!(at(290.0), (Some(30.0), 1));
        assert_eq!(at(289.0), (Some(15.0), 2), "two fewer");
        assert_eq!(at(275.0), (Some(15.0), 2));
        assert_eq!(at(274.0).0, None, "then none");
        // The pull request goes first, and before any chip of the ports does.
        let both = RowWants { extras: wants.extras.with(Extra::PullRequest, Some(20.0)), ..wants.clone() };
        let fit = fit_row(325.0, &both);
        assert!(!fit.extras.shows(Extra::PullRequest) && fit.ports_step == 0, "{fit:?}");
        assert_eq!(fit.extras.width(Extra::Ports), Some(50.0));
    }

    /// An extra the row doesn't have costs nothing, and the others go in the same order.
    #[test]
    fn a_missing_extra_is_not_counted_and_the_rest_keep_their_order() {
        let wants = RowWants {
            extras: Extras::default().with(Extra::Ports, Some(50.0)).with(Extra::Gauge, Some(40.0)),
            ..wants()
        };
        // 310 + 90, the path at its shortest and no words: 400 - 50 = 350.
        assert!(fit_row(350.0, &wants).extras.shows(Extra::Ports));
        let fit = fit_row(349.0, &wants);
        assert!(!fit.extras.shows(Extra::Ports) && fit.extras.shows(Extra::Gauge));
        assert!(!fit.extras.shows(Extra::PullRequest) && !fit.extras.shows(Extra::BranchStatus));
    }

    #[test]
    fn the_path_gives_way_first_down_to_its_last_folder() {
        assert_eq!(fit_row(WHOLE - 5.0, &wants()).path, PathShown::Step(1));
        let fit = fit_row(WHOLE - 20.0, &wants());
        assert_eq!(
            (fit.path, fit.words, fit.chip, fit.folded),
            (PathShown::Step(2), true, Some(30.0), false)
        );
    }

    #[test]
    fn then_the_status_words_go_whole() {
        let fit = fit_row(WHOLE - 21.0, &wants());
        assert_eq!(fit.path, PathShown::Step(2));
        assert!(!fit.words);
        assert_eq!((fit.chip, fit.folded, fit.name), (Some(30.0), false, 60.0));
        // Words that are gone free their whole width: no half of them is drawn.
        let fit = fit_row(WHOLE - 20.0 - 30.0, &wants());
        assert!(!fit.words && fit.chip == Some(30.0));
    }

    #[test]
    fn then_the_chip_shrinks_to_its_glyph() {
        let whole_less_words = WHOLE - 20.0 - 30.0;
        let fit = fit_row(whole_less_words - 8.0, &wants());
        assert_eq!((fit.words, fit.chip, fit.folded, fit.name), (false, Some(22.0), false, 60.0));
        let fit = fit_row(whole_less_words - 20.0, &wants());
        assert_eq!((fit.chip, fit.folded), (Some(10.0), false));
    }

    #[test]
    fn then_the_right_group_folds_into_one_menu_and_the_name_stays_whole() {
        // Chip at its glyph: 50 + 20 + 10 + 100 + 60 = 240; folded it is 50 + 20 + 10 + 28 + 60 = 168.
        let fit = fit_row(239.0, &wants());
        assert_eq!((fit.chip, fit.folded, fit.name), (Some(10.0), true, 60.0));
        let fit = fit_row(168.0, &wants());
        assert_eq!((fit.chip, fit.folded, fit.name), (Some(10.0), true, 60.0));
    }

    #[test]
    fn then_the_name_is_cut_at_its_end() {
        let fit = fit_row(160.0, &wants());
        assert_eq!((fit.path, fit.folded, fit.name), (PathShown::Step(2), true, 52.0));
        let fit = fit_row(168.0 - 40.0, &wants());
        assert_eq!((fit.path, fit.name), (PathShown::Step(2), 20.0));
    }

    #[test]
    fn last_the_path_folds_to_its_icon_and_the_name_takes_the_room() {
        // The name would be 19 with the path; 41 once the path's 20 are free.
        let fit = fit_row(168.0 - 41.0, &wants());
        assert_eq!((fit.path, fit.name), (PathShown::Icon, 60.0 - 41.0 + 20.0));
        let fit = fit_row(168.0 - 20.0 - 40.0, &wants());
        assert_eq!((fit.path, fit.name), (PathShown::Icon, 20.0));
        // Past that the name keeps what is left of the row, never a negative width.
        assert_eq!(fit_row(10.0, &wants()).name, 0.0);
    }

    #[test]
    fn without_a_branch_or_words_or_a_folder_there_is_less_to_give() {
        let bare = RowWants { path: vec![], words: 0.0, chip: None, ..wants() };
        let fit = fit_row(1000.0, &bare);
        assert_eq!((fit.path, fit.words, fit.chip), (PathShown::None, false, None));
        // 50 + 100 + 60 = 210, folded 138.
        assert!(fit_row(209.0, &bare).folded);
        assert!(!fit_row(210.0, &bare).folded);
        // No folder: the name gives way, and there is no path to fold.
        let fit = fit_row(138.0 - 30.0, &bare);
        assert_eq!((fit.path, fit.name), (PathShown::None, 30.0));
    }
}
