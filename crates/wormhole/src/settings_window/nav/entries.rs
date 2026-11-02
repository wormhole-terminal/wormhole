//! The rows the nav's component is given: for each visible section its entry, the sub-rows of the open one
//! under it, and the clear steps and headings between the runs. What the component draws from them is
//! `ui/settings_nav.dc.html`; here are the facts (which section is current, what a search found, what needs
//! attention) and the clicks.

use std::collections::HashMap;
use std::rc::Rc;

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::whole;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::settings_index::NavGroup;
use wormhole_core::config::settings_index::SectionId as Section;

use super::super::SettingsWindow;
use super::{section_icon, tile_token};
use crate::settings::{SettingsStore, UiColors, settings};
use crate::style;
use crate::theme_bridge::token;
use crate::ui::{Handler, NavEntry};

/// What a section's row says beside its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NavMark {
    None,
    /// Something on the page wants attention (an agent's hooks are incomplete): an amber dot.
    Attention,
    /// A search found this many settings on the page.
    Hits(usize),
    /// This many settings differ from their defaults: the Changed page's count.
    Changed(usize),
    /// A search found nothing on the page: the row dims.
    NoHits,
    /// A feature-flagged section still being built: a dot in the info colour, since the nav's Preview
    /// heading says what it is.
    Preview,
}

/// What VoiceOver reads for a row: its name, and what the row marks. A name with no mark is the
/// catalog's own text, which costs no allocation.
pub(super) fn spoken(name: &'static str, mark: NavMark) -> SharedString {
    match mark {
        NavMark::None => SharedString::new_static(name),
        NavMark::Attention => settings_nav_spoken_attention(name).into(),
        NavMark::Preview => settings_nav_spoken_preview(name).into(),
        NavMark::NoHits => settings_nav_spoken_no_matches(name).into(),
        NavMark::Hits(n) => settings_nav_spoken_matches(name, whole(n)).into(),
        NavMark::Changed(n) => settings_nav_spoken_changed(name, whole(n)).into(),
    }
}

impl SettingsWindow {
    /// The entries of the nav, in order. `hits` is how many settings a search found on each section.
    pub(super) fn nav_entries(
        &self,
        ui: UiColors,
        hits: &HashMap<Section, usize>,
        searching: bool,
        changed: usize,
        cx: &mut Context<Self>,
    ) -> Vec<NavEntry> {
        let attention = self.agents.as_ref().is_some_and(|agents| agents.needs_repair() > 0);
        // Only the sections shown for this build: the Preview group (feature flags, and a feature's own
        // section while its flag is on) is there only in a dev build, so a release never lists it. The
        // profile is the card above, with an avatar of its own.
        let shown: Vec<(Section, &'static str)> = Section::visible(settings(cx))
            .filter_map(|section| Some((section, tile_token(section)?)))
            // The Changed page is a run of its own after the profile card, listed only while something
            // differs (and kept while it is open, so the page that says "every setting is at its default"
            // has its row).
            .filter(|(section, _)| {
                *section != Section::Changed || changed > 0 || self.section == Section::Changed
            })
            .collect();
        let mut rows = Vec::new();
        let mut run: Option<NavGroup> = None;
        for (at, (section, tile)) in shown.iter().copied().enumerate() {
            let nav_group = section.info().nav;
            let starts_run = run != Some(nav_group);
            if starts_run {
                // The first run follows the profile card, which the window spaces itself.
                if run.is_some() {
                    rows.push(NavEntry { is_gap: true, ..NavEntry::default() });
                }
                if !nav_group.title().is_empty() {
                    rows.push(NavEntry {
                        is_label: true,
                        name: nav_group.title().into(),
                        ..NavEntry::default()
                    });
                }
                run = Some(nav_group);
            }
            let mark = match hits.get(&section) {
                _ if !searching && section == Section::Changed => NavMark::Changed(changed),
                _ if !searching && section == Section::Agents && attention => NavMark::Attention,
                // A preview section wears its tag when not searching; search marks override it.
                _ if !searching && section.is_preview() => NavMark::Preview,
                _ if !searching => NavMark::None,
                Some(n) => NavMark::Hits(*n),
                None => NavMark::NoHits,
            };
            let selected = section == self.section && !self.show_results;
            // A search shows what it found, not the page's groups.
            let subs = selected && !searching;
            let has_groups = self.nav_groups(section, settings(cx)).next().is_some();
            let tiles = &cx.global::<SettingsStore>().tokens;
            let pick: Handler = Rc::new(cx.listener(move |this, _, _, cx| this.show_section(section, cx)));
            rows.push(NavEntry {
                key: section as usize,
                is_item: true,
                name: section.title().into(),
                spoken: spoken(section.title(), mark),
                icon: Some(section_icon(section)),
                tile: style::nav_tile_fill(ui, token(tiles, tile), selected),
                selected,
                dim: mark == NavMark::NoHits,
                follows: !starts_run,
                opened: subs && has_groups,
                count: match mark {
                    NavMark::Hits(n) | NavMark::Changed(n) => Some(style::count_label(n).into()),
                    _ => None,
                },
                has_dot: matches!(mark, NavMark::Attention),
                preview: mark == NavMark::Preview,
                pick: Some(pick),
                ..NavEntry::default()
            });
            if subs {
                self.push_sub_rows(section, &mut rows, cx);
                // A step below the sub-rows, unless the run ends here.
                let ends_run = shown.get(at + 1).is_none_or(|(next, _)| next.info().nav != nav_group);
                if has_groups && !ends_run {
                    rows.push(NavEntry { is_sub_end: true, ..NavEntry::default() });
                }
            }
        }
        rows
    }

    /// A sub-row for each group of the open page.
    fn push_sub_rows(&self, section: Section, rows: &mut Vec<NavEntry>, cx: &mut Context<Self>) {
        let first = self.nav_groups(section, settings(cx)).next().map(|(at, _)| at);
        let current = self.current_group(first);
        let groups: Vec<_> = self.nav_groups(section, settings(cx)).collect();
        for (place, (at, group)) in groups.into_iter().enumerate() {
            let pick: Handler = Rc::new(cx.listener(move |this, _, _, cx| this.pick_group(at, cx)));
            rows.push(NavEntry {
                key: at,
                is_sub: true,
                name: group.title().into(),
                spoken: group.title().into(),
                current: current == Some(at),
                first: place == 0,
                follows: place > 0,
                pick: Some(pick),
                ..NavEntry::default()
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NavMark, spoken};

    #[test]
    fn a_name_with_no_mark_is_read_as_the_catalog_has_it() {
        assert_eq!(spoken("Fonts", NavMark::None), "Fonts");
    }
}
