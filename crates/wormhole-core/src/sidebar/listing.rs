//! The terminal list as the sidebar draws it: folder bands (or one flat list), pins on top,
//! the pinned folders' bands first and kept without a terminal in them, folded bands without
//! their rows, the filter applied, and the ⌘1–9 numbers on the first nine terminals you can see.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::filter::{Filter, Searchable};
use super::group::{Grouping, group_by_folder_in};
use super::order::{SidebarNumbers, SidebarOrder, stay_numbered, with_pins};
use super::split::{self, SplitMarks, Splits};

/// How many terminals get a ⌘ number.
pub const NUMBERED: usize = 9;

/// The sidebar's state that belongs to one window and is saved with it: how it groups, what
/// it filters, which bands are folded.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSidebar {
    pub grouping: Grouping,
    pub filter: Filter,
    /// Folded bands, by [`folder_key`].
    pub folded: BTreeSet<String>,
}

impl WindowSidebar {
    pub fn is_folded(&self, folder: Option<&Path>) -> bool {
        self.folded.contains(&folder_key(folder))
    }

    /// What the list is built from while the filter can't be seen (its feature flag is off):
    /// the same view with no filter, so a saved Show, Hide idle or search neither hides rows
    /// nor shows a chip the person can't clear. The saved view itself stays as it is, so
    /// turning the flag back on brings the filter back.
    pub fn as_seen(&self, filter_available: bool) -> Cow<'_, WindowSidebar> {
        if filter_available || self.filter == Filter::default() {
            Cow::Borrowed(self)
        } else {
            Cow::Owned(WindowSidebar { filter: Filter::default(), ..self.clone() })
        }
    }

    /// Fold an open band or open a folded one.
    pub fn toggle_fold(&mut self, folder: Option<&Path>) {
        let key = folder_key(folder);
        if !self.folded.remove(&key) {
            self.folded.insert(key);
        }
    }
}

/// How a band is remembered: its folder's path, or nothing for "No folder".
pub fn folder_key(folder: Option<&Path>) -> String {
    folder.map(|folder| folder.to_string_lossy().into_owned()).unwrap_or_default()
}

/// What the listing is built from. `rows` and `pinned` are in the window's (manual) order.
pub struct ListInput<'a> {
    pub grouping: Grouping,
    /// Each session's folder (its repository, else its working directory); only read when the
    /// list is grouped by folder, so a flat list never has to look for them.
    pub folders: &'a [Option<PathBuf>],
    /// The folders the user pinned, in the order they were pinned: their bands lead the list and
    /// stay in it with no terminal inside (unless a filter is active). Only read when the list is
    /// grouped by folder.
    pub pinned_folders: &'a [PathBuf],
    pub rows: &'a [Searchable<'a>],
    pub pinned: &'a [bool],
    pub sidebar: &'a WindowSidebar,
    pub home: Option<&'a Path>,
    /// Who is in which split, and which terminal is selected: what makes a split's members sit
    /// together and show their marks. The default has none, so every terminal stands alone.
    pub splits: Splits<'a>,
    /// How the list keeps its order and what the ⌘ numbers count along (the two Settings rows).
    pub order: SidebarOrder,
    pub numbers: SidebarNumbers,
    /// The order the bands that aren't pinned folders keep (`order::settle_bands`), used under
    /// [`SidebarOrder::Stable`]; empty means the order of each band's first terminal.
    pub band_order: &'a [PathBuf],
}

/// One folder band.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Band {
    /// `None` is "No folder" (always last).
    pub folder: Option<PathBuf>,
    pub label: String,
    /// The folder is a pinned one: it stays in the list with no terminal in it.
    pub pinned: bool,
    /// Every terminal in it, pinned first: what a drag reorders. Empty for a pinned folder
    /// nobody is in.
    pub members: Vec<usize>,
    /// How many of them the filter keeps: what a folded band counts.
    pub matches: usize,
    pub folded: bool,
}

/// One line of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Band {
        band: usize,
        position: usize,
        of: usize,
    },
    /// A terminal: its place among the rows of its band (or of the flat list), from 1, its
    /// ⌘ number if it has one, and what it shows of its split (nothing, for a terminal in none).
    Row {
        session: usize,
        band: Option<usize>,
        position: usize,
        of: usize,
        number: Option<usize>,
        marks: SplitMarks,
    },
}

/// The list top to bottom.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    /// Empty when not grouped by folder.
    pub bands: Vec<Band>,
    pub entries: Vec<Entry>,
    /// Terminals the filter leaves out ("8 terminals hidden").
    pub hidden: usize,
    /// Idle terminals, filtered or not ("Hide idle terminals · 7 idle").
    pub idle: usize,
    /// Every terminal, as listed without folding or filter (pins applied).
    order: Vec<usize>,
    /// The terminals that carry a ⌘ number, in number order.
    numbered: Vec<usize>,
}

impl Listing {
    /// The list. A split's members sit together in their band, pinning one pins all of them, a
    /// filter that keeps one keeps all, and each row's marks say how it belongs.
    pub fn build(input: &ListInput) -> Self {
        let ListInput {
            grouping,
            folders,
            pinned_folders,
            rows,
            pinned,
            sidebar,
            home,
            splits,
            order: order_mode,
            numbers: numbers_mode,
            band_order,
        } = *input;
        // Classic orders the bands by their first terminal, as they stand now.
        let band_order = if order_mode == SidebarOrder::Stable { band_order } else { &[] };
        let filter = &sidebar.filter;
        let ids = split::normalised(splits.split_of, rows.len());
        let admitted =
            split::admitted_with_splits(&rows.iter().map(|row| filter.admits(row)).collect::<Vec<_>>(), &ids);
        let pinned = split::pinned_with_splits(pinned, &ids, rows.len());
        let is_pinned = |ix: usize| pinned.get(ix).copied().unwrap_or(false);
        let mut listing = Listing {
            hidden: admitted.iter().filter(|&&kept| !kept).count(),
            idle: rows.iter().filter(|row| row.state.is_idle()).count(),
            ..Default::default()
        };
        // The band of each place in `order`, for telling whether a split is together.
        let mut order_band: Vec<Option<usize>> = Vec::new();
        match grouping {
            Grouping::None => {
                let pinned_first = with_pins(&(0..rows.len()).collect::<Vec<_>>(), is_pinned);
                listing.order = split::gather_splits(&pinned_first, &ids);
                order_band = vec![None; listing.order.len()];
                let shown: Vec<usize> = listing.order.iter().copied().filter(|&ix| admitted[ix]).collect();
                listing.push_rows(&shown, None);
            }
            Grouping::Folder => {
                for group in group_by_folder_in(folders, pinned_folders, home, band_order) {
                    // Gathered within the band, so a terminal never leaves its folder for its split.
                    let members = split::gather_splits(&with_pins(&group.sessions, is_pinned), &ids);
                    order_band.extend(std::iter::repeat_n(Some(listing.bands.len()), members.len()));
                    let shown: Vec<usize> =
                        members.iter().copied().filter(|&ix| admitted.get(ix) == Some(&true)).collect();
                    listing.order.extend(&members);
                    let folded = sidebar.is_folded(group.folder.as_deref());
                    let matches = shown.len();
                    listing.bands.push(Band {
                        folder: group.folder,
                        label: group.label,
                        pinned: group.pinned,
                        members,
                        matches,
                        folded,
                    });
                    // A filter keeps the bands that hold a match, and only those; without one, a
                    // band with nothing in it is a pinned folder's, and stays.
                    if shown.is_empty() && filter.is_active() {
                        continue;
                    }
                    let band = listing.bands.len() - 1;
                    listing.entries.push(Entry::Band { band, position: 0, of: 0 });
                    if !folded {
                        listing.push_rows(&shown, Some(band));
                    }
                }
                let bands: Vec<usize> = listing.band_entries().collect();
                let of = bands.len();
                for (n, at) in bands.into_iter().enumerate() {
                    if let Entry::Band { position, of: total, .. } = &mut listing.entries[at] {
                        (*position, *total) = (n + 1, of);
                    }
                }
            }
        }
        listing.numbered = match numbers_mode {
            SidebarNumbers::List => listing.visible().take(NUMBERED).collect(),
            SidebarNumbers::Terminal => stay_numbered(rows.len(), NUMBERED, is_pinned),
        };
        let numbered = listing.numbered.clone();
        for entry in &mut listing.entries {
            if let Entry::Row { session, number, .. } = entry {
                *number = numbered.iter().position(|ix| ix == session).map(|at| at + 1);
            }
        }
        let mut shown = vec![false; rows.len()];
        for ix in listing.visible() {
            shown[ix] = true;
        }
        let marks = split::marks(&listing.order, &order_band, &shown, &ids, splits.selected);
        for entry in &mut listing.entries {
            if let Entry::Row { session, marks: own, .. } = entry {
                *own = marks.get(session).copied().unwrap_or_default();
            }
        }
        listing
    }

    fn push_rows(&mut self, sessions: &[usize], band: Option<usize>) {
        let of = sessions.len();
        for (n, &session) in sessions.iter().enumerate() {
            // The numbers are set once the whole list is known (`build`).
            let number = None;
            let marks = SplitMarks::default();
            self.entries.push(Entry::Row { session, band, position: n + 1, of, number, marks });
        }
    }

    fn band_entries(&self) -> impl Iterator<Item = usize> + '_ {
        self.entries.iter().enumerate().filter(|(_, e)| matches!(e, Entry::Band { .. })).map(|(at, _)| at)
    }

    /// The terminals you can see, top to bottom.
    pub fn visible(&self) -> impl Iterator<Item = usize> + '_ {
        self.entries.iter().filter_map(|entry| match entry {
            Entry::Row { session, .. } => Some(*session),
            Entry::Band { .. } => None,
        })
    }

    /// The terminals that carry a ⌘ number, in number order.
    pub fn numbered(&self) -> Vec<usize> {
        self.numbered.clone()
    }

    /// Every terminal top to bottom, folded or filtered out ones included (pins applied).
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// The terminal `delta` places from `active` (⌘⇧] / ⌘⇧[), wrapping around the terminals you
    /// can see; the whole list when `active` itself is folded away or filtered out.
    pub fn step(&self, active: usize, delta: isize) -> Option<usize> {
        let visible: Vec<usize> = self.visible().collect();
        let along = if visible.contains(&active) { visible } else { self.order.clone() };
        let place = along.iter().position(|&ix| ix == active)?;
        let next = (place as isize + delta).rem_euclid(along.len() as isize) as usize;
        along.get(next).copied()
    }

    /// What `session`'s row shows of its split (nothing, for a terminal in none or not on screen).
    pub fn marks(&self, session: usize) -> SplitMarks {
        self.entries
            .iter()
            .find_map(|entry| match entry {
                Entry::Row { session: own, marks, .. } if *own == session => Some(*marks),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// The band `session` is in.
    pub fn band_of(&self, session: usize) -> Option<usize> {
        self.bands.iter().position(|band| band.members.contains(&session))
    }
}

#[cfg(test)]
mod tests {
    use super::{Entry, ListInput, Listing, WindowSidebar, folder_key};
    use crate::sidebar::filter::{Filter, RowState, Searchable, Show};
    use crate::sidebar::{Grouping, NO_FOLDER, SidebarNumbers, SidebarOrder, SplitMarks, Splits};
    use std::path::{Path, PathBuf};

    const NAMES: [&str; 12] = ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9", "a10", "a11"];

    fn rows(states: &[RowState]) -> Vec<Searchable<'static>> {
        states
            .iter()
            .enumerate()
            .map(|(ix, &state)| Searchable {
                name: NAMES[ix],
                state,
                place: None,
                branch: None,
                command: None,
            })
            .collect()
    }

    fn dirs(paths: &[Option<&str>]) -> Vec<Option<PathBuf>> {
        paths.iter().map(|p| p.map(PathBuf::from)).collect()
    }

    fn build(
        grouping: Grouping,
        folders: &[Option<PathBuf>],
        rows: &[Searchable],
        pinned: &[bool],
        sidebar: &WindowSidebar,
    ) -> Listing {
        build_with_folders(grouping, folders, &[], rows, pinned, sidebar)
    }

    fn build_with_folders(
        grouping: Grouping,
        folders: &[Option<PathBuf>],
        pinned_folders: &[PathBuf],
        rows: &[Searchable],
        pinned: &[bool],
        sidebar: &WindowSidebar,
    ) -> Listing {
        Listing::build(&ListInput {
            grouping,
            folders,
            pinned_folders,
            rows,
            pinned,
            sidebar,
            home: None,
            splits: Splits::default(),
            order: SidebarOrder::Classic,
            numbers: SidebarNumbers::List,
            band_order: &[],
        })
    }

    /// The list as text: bands as `[label]`, rows as their session index and ⌘ number.
    fn shape(listing: &Listing) -> Vec<String> {
        listing
            .entries
            .iter()
            .map(|entry| match *entry {
                Entry::Band { band, .. } => format!("[{}]", listing.bands[band].label),
                Entry::Row { session, number: Some(n), .. } => format!("{session}:{n}"),
                Entry::Row { session, number: None, .. } => format!("{session}"),
            })
            .collect()
    }

    #[test]
    fn with_nothing_pinned_folded_or_filtered_the_order_is_the_grouping_order() {
        let folders = dirs(&[Some("/w/b"), Some("/w/a"), None, Some("/w/b")]);
        let rows = rows(&[RowState::Idle; 4]);
        let none = [false; 4];
        let sidebar = WindowSidebar::default();
        for grouping in Grouping::ALL {
            let listing = build(grouping, &folders, &rows, &none, &sidebar);
            assert_eq!(listing.order(), grouping.order(&folders, &[]), "{grouping:?}");
            assert_eq!(listing.visible().collect::<Vec<_>>(), grouping.order(&folders, &[]), "{grouping:?}");
        }
        let grouped = build(Grouping::Folder, &folders, &rows, &none, &sidebar);
        assert_eq!(shape(&grouped), ["[b]", "0:1", "3:2", "[a]", "1:3", &format!("[{NO_FOLDER}]"), "2:4"]);
    }

    #[test]
    fn a_flat_list_never_reads_folders() {
        let rows = rows(&[RowState::Idle; 3]);
        let listing = build(Grouping::None, &[], &rows, &[false; 3], &WindowSidebar::default());
        assert_eq!(shape(&listing), ["0:1", "1:2", "2:3"]);
        assert!(listing.bands.is_empty());
    }

    #[test]
    fn numbers_stop_at_nine_and_skip_folded_bands() {
        let folders: Vec<Option<PathBuf>> =
            (0..12).map(|ix| Some(PathBuf::from(if ix < 2 { "/w/a" } else { "/w/b" }))).collect();
        let rows = rows(&[RowState::Idle; 12]);
        let open = build(Grouping::Folder, &folders, &rows, &[false; 12], &WindowSidebar::default());
        assert_eq!(open.numbered(), [0, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(open.entries.iter().any(|e| matches!(e, Entry::Row { session: 9, number: None, .. })));

        let mut sidebar = WindowSidebar::default();
        sidebar.toggle_fold(Some(Path::new("/w/a")));
        let folded = build(Grouping::Folder, &folders, &rows, &[false; 12], &sidebar);
        assert_eq!(folded.numbered(), [2, 3, 4, 5, 6, 7, 8, 9, 10], "a folded band takes no number");
        assert_eq!(shape(&folded)[..3], ["[a]", "[b]", "2:1"]);
        // Its terminals are still in the order (⌘⇧] from one of them goes on from there).
        assert_eq!(folded.order().len(), 12);
    }

    #[test]
    fn pinned_terminals_lead_their_band_and_the_flat_list() {
        let folders = dirs(&[Some("/w/a"), Some("/w/b"), Some("/w/a"), Some("/w/a")]);
        let rows = rows(&[RowState::Idle; 4]);
        let pinned = [false, false, false, true];
        let sidebar = WindowSidebar::default();
        let grouped = build(Grouping::Folder, &folders, &rows, &pinned, &sidebar);
        assert_eq!(shape(&grouped), ["[a]", "3:1", "0:2", "2:3", "[b]", "1:4"]);
        assert_eq!(grouped.bands[0].members, [3, 0, 2]);
        let flat = build(Grouping::None, &folders, &rows, &pinned, &sidebar);
        assert_eq!(shape(&flat), ["3:1", "0:2", "1:3", "2:4"]);
    }

    fn paths(paths: &[&str]) -> Vec<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn pinned_folders_lead_in_pin_order_and_stay_with_no_terminal_in_them() {
        let folders = dirs(&[Some("/w/b"), Some("/w/a"), Some("/w/b")]);
        let rows = rows(&[RowState::Idle; 3]);
        let sidebar = WindowSidebar::default();
        let pins = paths(&["/w/z", "/w/b"]);
        let listing = build_with_folders(Grouping::Folder, &folders, &pins, &rows, &[false; 3], &sidebar);
        // The empty band has no rows and takes no ⌘ number; the pinned `b` leads `a`.
        assert_eq!(shape(&listing), ["[z]", "[b]", "0:1", "2:2", "[a]", "1:3"]);
        assert_eq!(
            listing
                .bands
                .iter()
                .map(|band| (band.pinned, band.members.len(), band.matches))
                .collect::<Vec<_>>(),
            [(true, 0, 0), (true, 2, 2), (false, 1, 1)]
        );
        assert_eq!(listing.bands[0].folder.as_deref(), Some(Path::new("/w/z")));
        assert_eq!(listing.entries[0], Entry::Band { band: 0, position: 1, of: 3 });
        assert_eq!(listing.entries[4], Entry::Band { band: 2, position: 3, of: 3 });
        // The order, the next terminal and the sessions' bands follow the bands' new places.
        assert_eq!(listing.order(), Grouping::Folder.order(&folders, &pins));
        assert_eq!(listing.order(), [0, 2, 1]);
        assert_eq!(listing.band_of(1), Some(2));
        assert_eq!(listing.step(1, 1), Some(0));
    }

    #[test]
    fn a_pinned_folder_that_is_empty_can_fold_and_counts_nothing() {
        let rows = rows(&[]);
        let mut sidebar = WindowSidebar::default();
        let pins = paths(&["/w/z"]);
        sidebar.toggle_fold(Some(Path::new("/w/z")));
        let listing = build_with_folders(Grouping::Folder, &[], &pins, &rows, &[], &sidebar);
        assert_eq!(shape(&listing), ["[z]"]);
        assert!(listing.bands[0].folded);
        assert_eq!((listing.hidden, listing.idle), (0, 0));
        assert!(listing.numbered().is_empty());
    }

    #[test]
    fn an_active_filter_hides_a_pinned_folder_without_a_match_like_any_band() {
        let folders = dirs(&[Some("/w/a"), Some("/w/b")]);
        let rows = rows(&[RowState::Idle, RowState::Working]);
        let pins = paths(&["/w/z", "/w/a"]);
        let mut sidebar = WindowSidebar::default();
        let listing = build_with_folders(Grouping::Folder, &folders, &pins, &rows, &[false; 2], &sidebar);
        assert_eq!(shape(&listing), ["[z]", "[a]", "0:1", "[b]", "1:2"]);

        sidebar.filter = Filter { show: Show::Working, ..Filter::default() };
        let listing = build_with_folders(Grouping::Folder, &folders, &pins, &rows, &[false; 2], &sidebar);
        assert_eq!(shape(&listing), ["[b]", "1:1"]);
        assert_eq!(listing.entries[0], Entry::Band { band: 2, position: 1, of: 1 });

        // A search that matches nothing leaves no band at all, pinned ones included.
        sidebar.filter = Filter { search: "zzz".into(), ..Filter::default() };
        let listing = build_with_folders(Grouping::Folder, &folders, &pins, &rows, &[false; 2], &sidebar);
        assert!(listing.entries.is_empty());
        assert_eq!(listing.hidden, 2);
    }

    #[test]
    fn a_pinned_folder_with_terminals_holds_them_and_two_of_a_name_are_spelled_out() {
        let folders = dirs(&[Some("/Users/me/b/app"), Some("/Users/me/b/app")]);
        let rows = rows(&[RowState::Idle; 2]);
        let pins = paths(&["/Users/me/a/app"]);
        let listing = Listing::build(&ListInput {
            grouping: Grouping::Folder,
            folders: &folders,
            pinned_folders: &pins,
            rows: &rows,
            pinned: &[false; 2],
            sidebar: &WindowSidebar::default(),
            home: Some(Path::new("/Users/me")),
            splits: Splits::default(),
            order: SidebarOrder::Classic,
            numbers: SidebarNumbers::List,
            band_order: &[],
        });
        assert_eq!(shape(&listing), ["[~/a/app]", "[~/b/app]", "0:1", "1:2"]);
        assert!(listing.bands[0].pinned && !listing.bands[1].pinned);
    }

    #[test]
    fn a_flat_list_ignores_pinned_folders() {
        let folders = dirs(&[Some("/w/a"), Some("/w/b")]);
        let rows = rows(&[RowState::Idle; 2]);
        let pins = paths(&["/w/z", "/w/b"]);
        let sidebar = WindowSidebar::default();
        let flat = build_with_folders(Grouping::None, &folders, &pins, &rows, &[false; 2], &sidebar);
        assert_eq!(shape(&flat), ["0:1", "1:2"]);
        assert!(flat.bands.is_empty());
    }

    #[test]
    fn a_filter_keeps_matching_rows_and_the_bands_that_hold_them() {
        let folders = dirs(&[Some("/w/a"), Some("/w/b"), Some("/w/a"), Some("/w/c")]);
        let rows = rows(&[RowState::Idle, RowState::NeedsYou, RowState::Working, RowState::Idle]);
        let mut sidebar = WindowSidebar {
            filter: Filter { show: Show::Working, ..Filter::default() },
            ..Default::default()
        };
        let listing = build(Grouping::Folder, &folders, &rows, &[false; 4], &sidebar);
        assert_eq!(shape(&listing), ["[a]", "2:1"]);
        assert_eq!((listing.hidden, listing.idle), (3, 2));
        assert_eq!(listing.bands[0].matches, 1, "a folded band would count only its match");

        // Hiding idle terminals under All.
        sidebar.filter = Filter { hide_idle: true, ..Filter::default() };
        let listing = build(Grouping::None, &folders, &rows, &[false; 4], &sidebar);
        assert_eq!(shape(&listing), ["1:1", "2:2"]);

        // Nothing matches: no bands, every terminal hidden.
        sidebar.filter = Filter { search: "zzz".into(), ..Filter::default() };
        let listing = build(Grouping::Folder, &folders, &rows, &[false; 4], &sidebar);
        assert!(listing.entries.is_empty());
        assert_eq!(listing.hidden, 4);
    }

    #[test]
    fn a_filter_nobody_can_see_does_not_hide_rows_and_comes_back_with_the_flag() {
        let folders = dirs(&[Some("/w/a"), Some("/w/b"), Some("/w/a")]);
        let rows = rows(&[RowState::Idle, RowState::NeedsYou, RowState::Working]);
        let saved = WindowSidebar {
            grouping: Grouping::Folder,
            filter: Filter { show: Show::Working, hide_idle: true, search: "a2".into() },
            ..Default::default()
        };
        let list = |view: &WindowSidebar| build(Grouping::Folder, &folders, &rows, &[false; 3], view);

        // The funnel is there: the filter applies.
        let on = saved.as_seen(true);
        assert!(on.filter.is_active());
        assert_eq!(list(&on).hidden, 2);

        // The funnel is gone: everything is listed, nothing is hidden, no chip, grouping kept.
        let off = saved.as_seen(false);
        assert!(!off.filter.is_active());
        assert!(off.filter.chip().is_none());
        assert_eq!(off.grouping, Grouping::Folder);
        let unfiltered = list(&off);
        assert_eq!(unfiltered.hidden, 0);
        assert_eq!(unfiltered.order().len(), 3);
        assert_eq!(unfiltered, list(&WindowSidebar { filter: Filter::default(), ..saved.clone() }));

        // The saved view is untouched, so the filter returns with the flag.
        assert_eq!(saved.filter.show, Show::Working);
        assert_eq!(list(&saved.as_seen(true)).hidden, 2);
    }

    #[test]
    fn rows_and_bands_know_their_place_in_their_set() {
        let folders = dirs(&[Some("/w/a"), Some("/w/b"), Some("/w/a")]);
        let rows = rows(&[RowState::Idle; 3]);
        let listing = build(Grouping::Folder, &folders, &rows, &[false; 3], &WindowSidebar::default());
        assert_eq!(listing.entries[0], Entry::Band { band: 0, position: 1, of: 2 });
        assert_eq!(
            listing.entries[2],
            Entry::Row {
                session: 2,
                band: Some(0),
                position: 2,
                of: 2,
                number: Some(2),
                marks: SplitMarks::default()
            }
        );
        assert_eq!(listing.entries[3], Entry::Band { band: 1, position: 2, of: 2 });
        assert_eq!(listing.band_of(2), Some(0));
    }

    #[test]
    fn stepping_wraps_around_what_you_can_see() {
        let folders = dirs(&[Some("/w/a"), Some("/w/b"), Some("/w/a")]);
        let rows = rows(&[RowState::Idle; 3]);
        let mut sidebar = WindowSidebar::default();
        let listing = build(Grouping::Folder, &folders, &rows, &[false; 3], &sidebar);
        assert_eq!(listing.step(0, 1), Some(2));
        assert_eq!(listing.step(1, 1), Some(0), "wraps");
        assert_eq!(listing.step(0, -1), Some(1));
        sidebar.toggle_fold(Some(Path::new("/w/b")));
        let listing = build(Grouping::Folder, &folders, &rows, &[false; 3], &sidebar);
        assert_eq!(listing.step(2, 1), Some(0), "the folded terminal is skipped");
        assert_eq!(listing.step(1, 1), Some(0), "from a folded one, along the whole list");
    }

    #[test]
    fn folding_is_remembered_per_folder_and_round_trips() {
        let mut sidebar = WindowSidebar::default();
        sidebar.toggle_fold(Some(Path::new("/w/a")));
        sidebar.toggle_fold(None);
        assert!(sidebar.is_folded(Some(Path::new("/w/a"))));
        assert!(sidebar.is_folded(None));
        assert!(!sidebar.is_folded(Some(Path::new("/w/b"))));
        assert_eq!(folder_key(None), "");
        let json = serde_json::to_string(&sidebar).unwrap_or_default();
        assert_eq!(serde_json::from_str::<WindowSidebar>(&json).ok(), Some(sidebar.clone()));
        sidebar.toggle_fold(Some(Path::new("/w/a")));
        assert!(!sidebar.is_folded(Some(Path::new("/w/a"))));
    }
}
