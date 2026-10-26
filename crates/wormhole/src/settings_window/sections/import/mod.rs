//! Import: the terminals found on this Mac, what each would bring over (and what it can't), and how
//! each compares with Wormhole. The page is two blocks drawn from the scan the window keeps
//! (`found`, `review`, `compare`); the words are in `words`.
//!
//! The scan reads files under the real home, so it runs on the background executor and only when
//! the page is first shown or "Scan again" is pressed, never at launch. Nothing is written until
//! "Import selected".

mod compare;
mod found;
mod review;
mod words;

use std::collections::HashMap;

use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::import::{Found, Proposed, Source};

use crate::settings::import::{self as shared, Brought};
use crate::settings_window::SettingsWindow;
use crate::settings_window::registry::Setting;

pub(super) const SETTINGS: &[Setting] =
    &[Setting::block("import-found", found::draw), Setting::block("import-compare", compare::draw)];

/// What the page keeps of its scan: the terminals found, the one open, which rows are ticked.
#[derive(Default)]
pub(crate) struct ImportPage {
    /// `None` until the first scan finishes.
    found: Option<Vec<Found>>,
    /// The scan in progress; a page that was never shown has none and starts one when it is.
    task: Option<Task<()>>,
    started: bool,
    /// The terminal whose settings are listed under its card.
    open: Option<Source>,
    /// Per terminal, whether each row of its proposal is ticked.
    ticked: HashMap<Source, Vec<bool>>,
    /// The last import, with the terminal it was from.
    done: Option<(Source, Brought)>,
}

impl ImportPage {
    pub(crate) fn scanning(&self) -> bool {
        self.task.is_some()
    }

    fn open_found(&self) -> Option<&Found> {
        let open = self.open?;
        self.found.as_ref()?.iter().find(|found| found.source == open)
    }

    /// The rows of `found` that are ticked, as the page shows them.
    fn chosen(&self, found: &Found) -> Vec<Proposed> {
        let ticks = self.ticked.get(&found.source);
        found
            .proposal
            .values
            .iter()
            .enumerate()
            .filter(|(ix, _)| ticks.and_then(|t| t.get(*ix)).copied().unwrap_or(false))
            .map(|(_, proposed)| proposed.clone())
            .collect()
    }

    /// A scan's result: the ticks start at each row's default, except for a terminal whose proposal
    /// is the one already shown (a scan again that found nothing new), which keeps the person's; the
    /// newest terminal opens unless one that is still there already is.
    fn take(&mut self, found: Vec<Found>) {
        let before = self.found.take().unwrap_or_default();
        let mut ticked = HashMap::new();
        for scanned in &found {
            let kept = before
                .iter()
                .find(|old| old.source == scanned.source && old.proposal == scanned.proposal)
                .and_then(|_| self.ticked.remove(&scanned.source));
            let ticks = kept.unwrap_or_else(|| {
                scanned.proposal.values.iter().map(|proposed| proposed.default_on).collect()
            });
            ticked.insert(scanned.source, ticks);
        }
        self.ticked = ticked;
        if !self.open.is_some_and(|open| found.iter().any(|f| f.source == open)) {
            self.open = found.first().map(|f| f.source);
        }
        self.done = None;
        self.found = Some(found);
    }
}

impl SettingsWindow {
    /// Showing the page starts the scan the first time; later showings find it started.
    pub(in crate::settings_window) fn import_shown(&mut self, cx: &mut Context<Self>) {
        if !self.import.started {
            self.scan_import(cx);
        }
    }

    /// Look for terminals again, in the background; what is shown stays until the result comes.
    pub(in crate::settings_window) fn scan_import(&mut self, cx: &mut Context<Self>) {
        self.import.started = true;
        self.import.task = Some(cx.spawn(async move |this, cx| {
            let found = cx.background_executor().spawn(async { shared::scan_here() }).await;
            this.update(cx, |this, cx| {
                this.import.task = None;
                this.import.take(found);
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    /// A scan that was made over a folder of the caller's own (devctl, a frame test): shown as the page's.
    #[cfg(any(test, feature = "devctl"))]
    pub(in crate::settings_window) fn set_import(&mut self, found: Vec<Found>, cx: &mut Context<Self>) {
        self.import.started = true;
        self.import.task = None;
        self.import.take(found);
        self.show_section(Section::Import, cx);
    }

    /// Bring over the ticked rows of `source`, in one settings update.
    pub(in crate::settings_window) fn apply_import(&mut self, source: Source, cx: &mut Context<Self>) {
        let Some(found) = self.import.found.as_ref().and_then(|all| all.iter().find(|f| f.source == source))
        else {
            return;
        };
        let chosen = self.import.chosen(found);
        match shared::apply(cx, found, &chosen) {
            Ok(brought) => {
                self.notice = None;
                self.import.done = Some((source, brought));
            }
            Err(why) => {
                tracing::warn!("importing from {}: {why}", source.key());
                self.notice = Some((Section::Import, why.into()));
                self.import.done = None;
            }
        }
        cx.notify();
    }

    /// devctl `import-open`: the same as opening that terminal's card.
    #[cfg(feature = "devctl")]
    pub(in crate::settings_window) fn import_open(&mut self, source: Source, cx: &mut Context<Self>) {
        if self.import_has(source) {
            self.import.open = Some(source);
            cx.notify();
        }
    }

    /// devctl `import-apply`: the same as pressing the button.
    #[cfg(feature = "devctl")]
    pub(in crate::settings_window) fn import_has(&self, source: Source) -> bool {
        self.import.found.as_ref().is_some_and(|all| all.iter().any(|f| f.source == source))
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::import::{Found, Proposal, Proposed, Setting, SettingValue, Source};

    use super::ImportPage;

    fn found(source: Source, rows: usize) -> Found {
        let values = (0..rows)
            .map(|ix| Proposed {
                setting: Setting::FontSize,
                from: format!("row {ix}"),
                value: SettingValue::Number(12.0),
                default_on: true,
            })
            .collect();
        Found {
            source,
            files: Vec::new(),
            modified: None,
            proposal: Proposal { values, skipped: Vec::new() },
        }
    }

    #[test]
    fn a_scan_again_keeps_the_ticks_of_a_terminal_that_did_not_change() {
        let mut page = ImportPage::default();
        page.take(vec![found(Source::Ghostty, 3), found(Source::Alacritty, 2)]);
        assert_eq!(page.ticked[&Source::Ghostty], [true, true, true]);
        page.ticked.insert(Source::Ghostty, vec![true, false, true]);
        page.ticked.insert(Source::Alacritty, vec![false, false]);

        // Ghostty reads the same; Alacritty's file changed (one row more).
        page.take(vec![found(Source::Ghostty, 3), found(Source::Alacritty, 3)]);
        assert_eq!(page.ticked[&Source::Ghostty], [true, false, true]);
        assert_eq!(page.ticked[&Source::Alacritty], [true, true, true]);
    }

    #[test]
    fn a_terminal_that_is_gone_loses_its_ticks_and_the_open_card() {
        let mut page = ImportPage::default();
        page.take(vec![found(Source::Ghostty, 1), found(Source::Kitty, 1)]);
        assert_eq!(page.open, Some(Source::Ghostty));
        page.take(vec![found(Source::Kitty, 1)]);
        assert_eq!(page.open, Some(Source::Kitty));
        assert!(!page.ticked.contains_key(&Source::Ghostty));
    }
}
