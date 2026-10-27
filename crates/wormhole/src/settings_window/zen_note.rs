//! The note a settings page carries while Zen mode is on. Zen mode draws the window with less than the
//! settings say (`settings::zen`) and saves nothing, so the rows keep showing the person's own values;
//! the note says why the window doesn't match them, and leaves Zen mode from the page.
//!
//! Only the pages with a row Zen mode overrides carry it: the title row, the status bar, the minimap
//! and the lines between the columns.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_text::keys::{ZEN_SETTINGS_LEAVE, ZEN_SETTINGS_NOTE};
use wormhole_text::text;

use crate::settings::{set_zen_mode, zen};
use crate::settings_window::{SettingsWindow, ui};
use crate::style;

/// The rows (by their id in the settings table) Zen mode overrides.
const OVERRIDDEN: [&str; 4] = ["title-row", "status-bar", "minimap", "column-dividers"];

/// Whether a row of `section` is one Zen mode overrides.
fn affects(section: Section) -> bool {
    section.groups().iter().any(|group| group.entries.iter().any(|entry| OVERRIDDEN.contains(&entry.id)))
}

/// The note for `section`'s page, while Zen mode is on and the page has a row it overrides.
pub(super) fn draw(section: Section, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    if !zen(cx) || !affects(section) {
        return None;
    }
    Some(
        style::banner(ui(cx), style::BannerKind::Info, false, text(ZEN_SETTINGS_NOTE))
            .debug_selector(|| "zen-note".into())
            .child(
                Button::new("leave-zen")
                    .ghost()
                    .small()
                    .label(text(ZEN_SETTINGS_LEAVE))
                    .on_click(|_, _, cx| set_zen_mode(cx, false)),
            )
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{TestAppContext, VisualTestContext};
    use wormhole_core::config::settings_index::SectionId as Section;

    use super::affects;
    use crate::settings::set_zen_mode;
    use crate::settings_window::{current_window, open_settings};
    use crate::window::test_harness::{isolate_in, start};

    #[test]
    fn the_pages_with_a_row_zen_mode_overrides_carry_the_note() {
        // The title row, the status bar and the dividers are Appearance's, the minimap the Terminal's.
        assert!(affects(Section::Appearance));
        assert!(affects(Section::Terminal));
        assert!(!affects(Section::Fonts));
        assert!(!affects(Section::Notifications));
    }

    #[gpui_kit::test]
    fn the_note_shows_while_zen_mode_is_on_and_the_page_has_a_row_it_overrides(cx: &mut TestAppContext) {
        if isolate_in(
            module_path!(),
            "the_note_shows_while_zen_mode_is_on_and_the_page_has_a_row_it_overrides",
        ) {
            return;
        }
        start(cx, 1);
        cx.update(|cx| open_settings(Some(Section::Appearance), cx));
        cx.run_until_parked();
        let (handle, page) = cx.update(|cx| current_window(cx)).expect("the settings window opened");
        let note = |cx: &mut TestAppContext| {
            cx.update(|cx| handle.update(cx, |_, window, cx| window.render_frame(cx))).expect("open");
            VisualTestContext::from_window(handle, cx).debug_bounds("zen-note").is_some()
        };
        assert!(!note(cx), "no note while Zen mode is off");

        cx.update(|cx| set_zen_mode(cx, true));
        cx.run_until_parked();
        assert!(note(cx), "Appearance says the window shows less");
        cx.update(|cx| page.update(cx, |page, cx| page.show_section(Section::Terminal, cx)));
        cx.run_until_parked();
        assert!(note(cx), "so does Terminal, where the minimap is");
        cx.update(|cx| page.update(cx, |page, cx| page.show_section(Section::Fonts, cx)));
        cx.run_until_parked();
        assert!(!note(cx), "a page with nothing Zen mode overrides has none");

        cx.update(|cx| page.update(cx, |page, cx| page.show_section(Section::Appearance, cx)));
        cx.update(|cx| set_zen_mode(cx, false));
        cx.run_until_parked();
        assert!(!note(cx), "and it goes with Zen mode");
    }
}
