//! A folder band: chevron, folder name, the pin of a pinned folder and the
//! worktree mark, the count while folded and the + under the pointer; and under the name a second
//! line, the folder's path, the ±N of its changes, its branch chip and the branch's pull request
//! (the "No folder" band, and every band in the compact density, stay one line with the chip, ±N
//! and pull request after the name). The band
//! of the folder the list is scrolled into sticks under the header, pushed up 1:1 by the next one.
use wormhole_core::git::pr::PullRequest;
use wormhole_core::sidebar::spoken;
use wormhole_core::sidebar::text::count;
use wormhole_core::sidebar::text::{band_line_widths, band_widths};
use wormhole_core::sidebar::{Band, GroupGit, display_path, group_git};

use super::super::app::app;
use super::super::focus::SidebarStop;
use super::super::title_chips::pr_look;
use super::rows::Frame;
use super::*;
use crate::style::{BandKind, BandParts, FolderLook, FolderMeta, Section};

/// What a band's right side takes (count or +, and its padding), and the room the chevron
/// and its gap take before the name, for knowing when the name or the branch is cut off.
const BAND_RIGHT_W: f32 = 48.0;
const BAND_LEAD_W: f32 = style::SIDEBAR_INSET + style::ITEM_LEAD_W + 8.0;
/// The gap between the name and the chip, and the chip's glyph and padding around its text.
const CHIP_GAP: f32 = 8.0;
const CHIP_CHROME: f32 = 28.0;
/// A mark after the name (the pin, the worktree), its gap included.
const MARK_W: f32 = 22.0;
/// A folder's path on the second line that would be left narrower than this is left out instead
/// of showing as a sliver of text.
const PATH_LEAST: f32 = 24.0;

/// How a band is drawn this frame: whether it has a second line, and what the cut-off math found.
struct BandLook {
    meta: Option<FolderMeta>,
    /// Something is cut off (the name, the path or the branch): the tooltip tells it whole.
    cut: bool,
}

impl Workspace {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_band(
        &self,
        listing: &Listing,
        band_ix: usize,
        (position, of): (usize, usize),
        git: &[Option<SessionGit>],
        frame: &Frame,
        sticky: bool,
        joined: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = frame.ui;
        let band = &listing.bands[band_ix];
        let git = group_git(&band.members, git, self.active);
        let changes = git.and_then(|g| g.changes).filter(|&n| frame.show_changes && n > 0);
        let worktree =
            band.folder.as_deref().filter(|_| git.is_some()).and_then(|root| self.worktree_of(root));

        let two_lines = band_has_second_line(band, frame);
        let pull_request = git.and(self.band_pull_request(band, cx));
        let pr = pull_request.as_ref().map(|pr| {
            let look = pr_look(pr, ui);
            let width = look.mark_w(frame.measure(&look.label, CAPTION_REM, FontWeight::MEDIUM, window));
            (band_pr_mark(pr, look, ui), width)
        });
        let pr_w = pr.as_ref().map(|(_, w)| CHIP_GAP + w);
        let mut pr = pr.map(|(mark, _)| mark);
        let detail = self.band_detail(
            band,
            git,
            changes,
            worktree.as_deref(),
            two_lines,
            pr.take_if(|_| !two_lines),
            frame,
        );
        let folded = band.folded;
        let this = cx.weak_entity();
        let (on_toggle, on_add) = (this.clone(), this);
        let (fold_folder, add_folder) = (band.folder.clone(), band.folder.clone());
        let parts = BandParts {
            kind: BandKind::Folder,
            section: if folded { Section::Collapsed } else { Section::Open },
            title: band.label.clone().into(),
            detail,
            count: Some(style::count_label(band.matches).into()),
            add: Some((
                "New terminal here",
                Box::new(move |_, window, cx| {
                    on_add.update(cx, |this, cx| this.new_session_in(add_folder.clone(), window, cx)).ok();
                }),
            )),
            on_toggle: Some(Box::new(move |_, _, cx| {
                on_toggle.update(cx, |this, cx| this.toggle_band(fold_folder.clone(), cx)).ok();
            })),
            drop_target: false,
            sticky,
            joined,
        };
        // The folder's colour (the project's, else the person's): a tint from the left, and a word
        // for VoiceOver.
        let colored = band.folder.as_deref().and_then(|folder| self.band_tint(folder, cx));
        let look = FolderLook {
            cmd_held: frame.hints,
            unknown: band.folder.is_none(),
            tint: colored.map(|(_, tint)| tint),
        };
        let id: ElementId = if sticky {
            "sticky-band".into()
        } else {
            ElementId::Name(SharedString::from(format!(
                "band-{}",
                wormhole_core::sidebar::folder_key(band.folder.as_deref())
            )))
        };
        let label = spoken::band_label(
            &band.label,
            band.pinned,
            colored.map(|(name, _)| name),
            git.map(|g| GroupGit { branch: g.branch, changes }),
            band.members.len(),
            !folded,
            position,
            of,
        );

        // A tooltip only when the name, the path or the branch is cut off: the whole path,
        // then the branch, the changes and how many terminals.
        let BandLook { meta, cut } =
            self.band_look(band, git, changes, worktree.is_some(), two_lines, (pr, pr_w), frame, window);
        let tooltip = cut.then(|| {
            let path =
                band.folder.as_deref().map_or_else(|| band.label.clone(), |f| display_path(f, home_dir()));
            let mut second = Vec::new();
            if let Some(branch) = git.and_then(|g| g.branch).filter(|_| frame.show_branch) {
                second.push(format!("⎇ {branch}"));
            }
            if let Some(n) = changes {
                second.push(count(n, "changed file", "changed files"));
            }
            second.push(count(band.members.len(), "terminal", "terminals"));
            (SharedString::from(path), SharedString::from(second.join(" · ")))
        });

        // The keys' place. Its stuck copy, which covers the band once the list scrolls,
        // wears the ring too; VoiceOver's focus stays on the band in the list.
        let (keyed, ring) = self.sidebar_keyed(&SidebarStop::Band(band.folder.clone()), window);
        let keyed = keyed && !sticky;
        let menu_open = band.folder.is_some() && self.sidebar.menu_band == band.folder;
        let element = style::folder_band(id, ui, parts, look, meta)
            .relative()
            .role(Role::TreeItem)
            .aria_label(label)
            .aria_expanded(!folded)
            .aria_level(1)
            .aria_position_in_set(position)
            .aria_size_of_set(of)
            .when(keyed, |d| d.aria_active_descendant())
            .when(ring || menu_open, |d| {
                d.child(style::focus_ring(ui, style::RingPlacement::Inside, px(0.0)))
            })
            .when_some(tooltip, |d, (first, second)| {
                d.tooltip(move |window, cx| two_line_tooltip(first.clone(), second.clone(), ui, window, cx))
            });
        // Last: the menu wraps the band.
        self.with_band_menu(element, band.folder.as_deref(), cx)
    }

    /// The second line of `band` (its path, ±N and branch) with how much of it fits, and whether
    /// anything on either line is cut off. A band without a second line only has the name and,
    /// after it, the chip and ±N to check.
    #[allow(clippy::too_many_arguments)]
    fn band_look(
        &self,
        band: &Band,
        git: Option<GroupGit>,
        changes: Option<usize>,
        has_worktree: bool,
        two_lines: bool,
        (pull_request, pr_w): (Option<AnyElement>, Option<f32>),
        frame: &Frame,
        window: &Window,
    ) -> BandLook {
        let ui = frame.ui;
        // The pull request's mark takes its room before the path and the chip share the rest.
        let pr_w = pr_w.unwrap_or_default();
        let marks = usize::from(band.pinned) + usize::from(has_worktree);
        let folder_w = frame.measure(&band.label, LABEL_REM, FontWeight::SEMIBOLD, window);
        let shown = git.filter(|_| frame.show_branch);
        // The chip's name and the glyph and padding round it, before the most a chip is drawn at
        // (its name is cut there) or the least.
        let chip_text = |git: GroupGit| {
            frame.measure(git.branch.unwrap_or("detached"), CAPTION_REM, FontWeight::MEDIUM, window)
                + CHIP_CHROME
        };

        let Some(folder) = band.folder.as_deref().filter(|_| two_lines) else {
            // One line: the chip and ±N follow the name, and the chip gives way before it.
            let avail = frame.width - BAND_LEAD_W - BAND_RIGHT_W - marks as f32 * MARK_W - pr_w;
            let chip_w = shown.filter(|git| git.branch.is_some()).map(chip_text);
            let fits = band_widths(
                avail,
                folder_w,
                chip_w,
                CHIP_GAP,
                style::FOLDER_NAME_MAX,
                style::BRANCH_CHIP_MIN,
            );
            let cut = folder_w > fits.folder + 0.5
                || chip_w
                    .zip(fits.chip)
                    .is_some_and(|(natural, drawn)| natural.min(style::BRANCH_CHIP_MAX) > drawn + 0.5)
                || chip_w.is_some_and(|w| w > style::BRANCH_CHIP_MAX);
            return BandLook { meta: None, cut };
        };

        // Two lines: the name has what the marks leave on the first, and on the second the path
        // gives way first, then the chip; ±N never does.
        let name_cut = folder_w > frame.width - BAND_LEAD_W - BAND_RIGHT_W - marks as f32 * MARK_W + 0.5;
        let path = SharedString::from(display_path(folder, home_dir()));
        let path_natural = frame.measure(&path, CAPTION_REM, FontWeight::NORMAL, window);
        let changes_w = changes
            .map(|n| frame.measure(&format!("±{n}"), CAPTION_REM, FontWeight::NORMAL, window))
            .unwrap_or_default();
        let chip_natural = shown.map(|git| chip_text(git).max(style::BRANCH_CHIP_MIN));
        let line = band_line_widths(
            frame.width - BAND_LEAD_W - style::SIDEBAR_INSET - pr_w,
            path_natural,
            changes_w,
            chip_natural.map(|w| w.min(style::BRANCH_CHIP_MAX)),
            CHIP_GAP,
            PATH_LEAST,
            style::BRANCH_CHIP_MIN,
        );
        let path_cut = line.path + 0.5 < path_natural;
        let chip_cut = chip_natural.zip(line.chip).is_some_and(|(natural, drawn)| {
            natural > style::BRANCH_CHIP_MAX || natural.min(style::BRANCH_CHIP_MAX) > drawn + 0.5
        });
        let branch = shown.map(|git| self.band_chip(band, git, chip_cut.then_some(line.chip).flatten(), ui));
        let meta = FolderMeta { path, path_w: path_cut.then_some(line.path), changes, branch, pull_request };
        BandLook { meta: Some(meta), cut: name_cut || path_cut || chip_cut }
    }

    /// The branch chip on a band's second line, with the tooltip a detached HEAD has: the commit it
    /// stands at. `max_w` is the width it is cut to on a narrow line.
    fn band_chip(&self, band: &Band, git: GroupGit, max_w: Option<f32>, ui: UiColors) -> AnyElement {
        let detached = git.branch.is_none();
        let head = band
            .folder
            .as_deref()
            .and_then(|root| self.sidebar.heads.get(root))
            .and_then(|(_, head)| head.clone());
        div()
            .id("branch-chip")
            .min_w_0()
            .flex()
            .when_some(max_w, |d, w| d.max_w(px(w)))
            .child(style::branch_tag(ui, git.branch))
            .when(detached, |d| {
                let first = SharedString::from(match &head {
                    Some(head) => format!("HEAD detached at {}", head.short),
                    None => "HEAD detached".to_string(),
                });
                let second = SharedString::from(head.map(|h| h.subject).unwrap_or_default());
                d.tooltip(move |window, cx| two_line_tooltip(first.clone(), second.clone(), ui, window, cx))
            })
            .into_any_element()
    }

    /// The pull request of a band's branch, as far as it is known: its terminals share the folder,
    /// so the first one whose branch was asked about answers for all.
    fn band_pull_request(&self, band: &Band, cx: &App) -> Option<PullRequest> {
        let cache = &app(cx).pr;
        band.members
            .iter()
            .find_map(|&ix| self.session_pr_branch(ix, cx).and_then(|branch| cache.get(&branch).cloned()))
    }

    /// After a band's name: the pin of a pinned folder, the worktree mark and, on a band of one
    /// line, the branch chip (a detached HEAD's names its commit), ±N and the pull request.
    #[allow(clippy::too_many_arguments)]
    fn band_detail(
        &self,
        band: &Band,
        git: Option<GroupGit>,
        changes: Option<usize>,
        worktree: Option<&Path>,
        two_lines: bool,
        pull_request: Option<AnyElement>,
        frame: &Frame,
    ) -> Option<AnyElement> {
        let ui = frame.ui;
        let pin = band.pinned.then(|| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .text_color(ui.faint)
                .child(Icon::new(IconName::Pin).xsmall())
        });
        let mark = worktree.map(|main| {
            let name = main
                .file_name()
                .map_or_else(|| main.display().to_string(), |n| n.to_string_lossy().into_owned());
            let first = SharedString::from(format!("{} · worktree of {name}", band.label));
            let second = SharedString::from(
                band.folder.as_deref().map(|f| display_path(f, home_dir())).unwrap_or_default(),
            );
            div()
                .id("worktree-mark")
                .flex_none()
                .flex()
                .items_center()
                .text_color(ui.faint)
                .child(style::icon(IconName::GitFork))
                .tooltip(move |window, cx| two_line_tooltip(first.clone(), second.clone(), ui, window, cx))
        });
        // The chip and ±N are on the second line of a band that has one.
        let chip = git.filter(|_| frame.show_branch && !two_lines).map(|git| {
            let detached = git.branch.is_none();
            let head = band
                .folder
                .as_deref()
                .and_then(|root| self.sidebar.heads.get(root))
                .and_then(|(_, head)| head.clone());
            div().id("branch-chip").min_w_0().flex().child(style::branch_chip(ui, git.branch, changes)).when(
                detached,
                |d| {
                    let first = SharedString::from(match &head {
                        Some(head) => format!("HEAD detached at {}", head.short),
                        None => "HEAD detached".to_string(),
                    });
                    let second = SharedString::from(head.map(|h| h.subject).unwrap_or_default());
                    d.tooltip(move |window, cx| {
                        two_line_tooltip(first.clone(), second.clone(), ui, window, cx)
                    })
                },
            )
        });
        // The changes go with the chip; without it they stand alone.
        let lone_changes = changes.filter(|_| chip.is_none() && !two_lines).map(|n| {
            div().flex_none().text_caption().tabular_nums().text_color(ui.muted).child(format!("±{n}"))
        });
        if pin.is_none()
            && mark.is_none()
            && chip.is_none()
            && lone_changes.is_none()
            && pull_request.is_none()
        {
            return None;
        }
        Some(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .children(pin)
                .children(mark)
                .children(chip)
                .children(lone_changes)
                .children(pull_request)
                .into_any_element(),
        )
    }

    /// The band of the folder the list is scrolled into, stuck under the header: one at a
    /// time, on the opaque `band.solid`, pushed up 1:1 by the next band.
    pub(super) fn sticky_band(
        &self,
        listing: &Listing,
        git: &[Option<SessionGit>],
        frame: &Frame,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let scroll = &self.sidebar.scroll;
        // A wheel event moves the offset past either end and the list only clamps it when it is
        // laid out, after this: unclamped, a list too short to scroll grows a stuck band, 4 pt
        // above its own, for every frame of a trackpad scroll.
        let offset = scroll.offset().y.clamp(-scroll.max_offset().y, px(0.0));
        let top = scroll.bounds().top();
        if offset >= px(0.0) {
            return None;
        }
        // Where an entry's top is in the list's viewport (bounds are from the last frame).
        let at = |ix: usize| scroll.bounds_for_item(ix).map(|b| b.top() + offset - top);
        let mut stuck = None;
        let mut next = None;
        for (ix, entry) in listing.entries.iter().enumerate() {
            let Entry::Band { band, position, of } = *entry else { continue };
            let y = at(ix)?;
            if y < px(0.0) {
                stuck = Some((band, position, of));
            } else {
                next = Some(y);
                break;
            }
        }
        let (band, position, of) = stuck?;
        // The next band pushes this one up by as much as the stuck band is tall.
        let height = band_height(&listing.bands[band], frame);
        let push = next.map_or(px(0.0), |next| (next - px(height)).min(px(0.0)));
        Some(
            div()
                .absolute()
                .top(push)
                .left_0()
                .right_0()
                .child(self.render_band(listing, band, (position, of), git, frame, true, false, window, cx))
                .into_any_element(),
        )
    }
}

/// Whether `band` has a second line (its path, ±N and branch): every band of a folder, except in
/// the compact density, which keeps one line per terminal and per band.
fn band_has_second_line(band: &Band, frame: &Frame) -> bool {
    band.folder.is_some() && !frame.compact
}

/// How tall `band` is drawn.
fn band_height(band: &Band, frame: &Frame) -> f32 {
    if band_has_second_line(band, frame) { style::FOLDER_BAND_H } else { style::BAND_H }
}

/// A band's pull request: its mark, named and explained for VoiceOver and the tooltip. A click
/// opens it and leaves the band folded or open as it was.
fn band_pr_mark(pr: &PullRequest, look: style::PrLook, ui: UiColors) -> AnyElement {
    let (url, words) = (pr.url.clone(), SharedString::from(pr.words()));
    let tip = words.clone();
    div()
        .id("band-pull-request")
        .flex_none()
        .role(Role::Link)
        .aria_label(words)
        .cursor_pointer()
        .child(style::pull_request_mark(ui, look))
        .tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(tip.clone()).build(window, cx))
        .on_click(move |_, _, cx| {
            cx.stop_propagation();
            cx.open_url(&url);
        })
        .into_any_element()
}
