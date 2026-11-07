//! Changes & diffs' live preview: the Changes panel's list, the diff view's rows and a binary file's
//! diff, each drawn by the feature's own parts (`features::git`) from the settings as they are now.
//! The strip shows one of the three; the page follows the setting last changed to its scene, and the
//! tabs over it show another.

use gpui_kit::*;
use wormhole_core::config::{Settings, SizeUnits};
use wormhole_core::view::layout::PANEL_DEFAULT;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{Key, text};

use crate::features::git::changes::sample as changes_sample;
use crate::features::git::diff::preview as diff_preview;
use crate::settings::settings;
use crate::settings_window::{SettingsWindow, ui};
use crate::style;

/// What the strip shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::settings_window) enum GitScene {
    /// The Changes panel's list: File view, Group untracked files separately.
    Panel,
    /// A diff of a file: Default layout, Wrap long lines, Tab width.
    Diff,
    /// A binary file's diff, which says its size: File sizes.
    Sizes,
}

impl GitScene {
    const ALL: [Self; 3] = [Self::Panel, Self::Diff, Self::Sizes];

    /// The tab's name: the group of the settings it shows, and the File sizes row's own.
    fn label(self) -> Key {
        match self {
            Self::Panel => SETTINGS_GIT_GROUP_CHANGES_PANEL,
            Self::Diff => SETTINGS_GIT_GROUP_DIFF_VIEW,
            Self::Sizes => SETTINGS_GIT_SIZE_UNITS,
        }
    }
}

/// The settings of the page, as they were when the preview last looked.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Knobs {
    tree: bool,
    untracked: bool,
    split: bool,
    wrap: bool,
    tab_width: usize,
    units: SizeUnits,
}

impl Knobs {
    fn of(s: &Settings) -> Self {
        Self {
            tree: s.changes_tree,
            untracked: s.group_untracked,
            split: s.diff_split,
            wrap: s.diff_wrap,
            tab_width: s.tab_width,
            units: s.size_units,
        }
    }
}

/// Which scene the strip is on, and the settings it last saw.
pub(in crate::settings_window) struct GitPreview {
    scene: GitScene,
    seen: Knobs,
    /// How wide the page's column is, as the window last drew it: the diff scene lays itself out as the
    /// diff view does at that width (Split only where `diff_split_fits`), so a narrow window shows the
    /// unified diff, which is shorter, and the scene stays whole in the strip.
    pub(in crate::settings_window) body_width: f32,
}

impl GitPreview {
    /// On the first group's scene, as the page opens.
    pub(in crate::settings_window) fn new(s: &Settings) -> Self {
        Self { scene: GitScene::Panel, seen: Knobs::of(s), body_width: f32::INFINITY }
    }

    /// A setting changed (from the page, a reset arrow or `config.json`): the strip moves to the scene
    /// of the one that did. Of several at once (Reset All), the diff's before the list's.
    pub(in crate::settings_window) fn follow(&mut self, s: &Settings) {
        let now = Knobs::of(s);
        let before = self.seen;
        self.seen = now;
        if now.units != before.units {
            self.scene = GitScene::Sizes;
        } else if (now.split, now.wrap, now.tab_width) != (before.split, before.wrap, before.tab_width) {
            self.scene = GitScene::Diff;
        } else if (now.tree, now.untracked) != (before.tree, before.untracked) {
            self.scene = GitScene::Panel;
        }
    }
}

/// The strip's scene: the one in front of the page's three, on the stage every preview stands on.
pub(in crate::settings_window) fn draw_git_preview(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let s = settings(cx).clone();
    let split_fits = wormhole_core::view::limits::diff_split_fits(window.git_preview.body_width);
    match window.git_preview.scene {
        // The panel's own list at the panel's width, in the middle of the stage. Its cards keep the edge the
        // panel gives them (`card.edge`): that is the real component, not this preview's frame.
        GitScene::Panel => div().w_full().max_w(px(PANEL_DEFAULT)).mx_auto().child(changes_sample::list(
            s.changes_tree,
            s.group_untracked,
            ui,
            cx,
        )),
        GitScene::Diff => diff_preview::diff(ui, &s, split_fits, cx),
        GitScene::Sizes => diff_preview::binary(ui, s.diff_split && split_fits, s.size_units, cx),
    }
    .into_any_element()
}

/// The tabs for the scenes, at the right of the strip's head: the strip is one fixed height
/// (`preview_column::PREVIEW_H`), and a row of tabs over the preview would take a fifth of it.
pub(in crate::settings_window) fn git_preview_tabs(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let scene = window.git_preview.scene;
    style::segmented_form(ui)
        .children(GitScene::ALL.into_iter().map(|tab| {
            style::form_segment(("git-preview-tab", tab as usize), ui, None, text(tab.label()), tab == scene)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.git_preview.scene = tab;
                    cx.notify();
                }))
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use wormhole_core::config::{Settings, SizeUnits};

    use super::{GitPreview, GitScene};

    fn after(change: impl FnOnce(&mut Settings)) -> GitScene {
        let mut s = Settings::default();
        let mut preview = GitPreview::new(&s);
        assert_eq!(preview.scene, GitScene::Panel, "the page opens on the first group's scene");
        change(&mut s);
        preview.follow(&s);
        preview.scene
    }

    #[test]
    fn the_strip_follows_the_setting_last_changed() {
        assert_eq!(after(|s| s.changes_tree = !s.changes_tree), GitScene::Panel);
        assert_eq!(after(|s| s.group_untracked = !s.group_untracked), GitScene::Panel);
        assert_eq!(after(|s| s.diff_split = !s.diff_split), GitScene::Diff);
        assert_eq!(after(|s| s.diff_wrap = !s.diff_wrap), GitScene::Diff);
        assert_eq!(after(|s| s.tab_width += 1), GitScene::Diff);
        assert_eq!(after(|s| s.size_units = SizeUnits::Binary), GitScene::Sizes);
    }

    #[test]
    fn a_tab_stays_while_nothing_changes() {
        let s = Settings::default();
        let mut preview = GitPreview::new(&s);
        preview.scene = GitScene::Diff;
        preview.follow(&s);
        assert_eq!(preview.scene, GitScene::Diff);
    }
}
