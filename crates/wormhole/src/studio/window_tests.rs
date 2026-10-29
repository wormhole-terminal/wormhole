//! The Studio window itself, in a headless GPUI (needs `devctl`'s test support). Drafts
//! resolve against the built-in themes only, so no test reads the user's themes folder. The
//! sheets are the platform's: the test platform queues them, and `simulate_prompt_answer`
//! presses a button by its label.

use gpui_kit::{
    AnyWindowHandle, AppContext as _, Context, Entity, TestAppContext, Window, WindowOptions, hsla,
};
use wormhole_core::theme::tokens::{ThemeFile, ThemeTokens, contrast_issues, resolve};

use super::draft::Themes;
use super::save::Notice;
use super::{Edit, StudioWindow};
use crate::settings::SettingsStore;
use crate::theme_bridge;

fn builtins_only(file: &ThemeFile) -> Result<ThemeTokens, String> {
    resolve(file, &|_| None)
}

/// A themes folder that is not the user's: no file is read, and "saving" fails with a
/// word the tests recognise.
pub(super) fn themes() -> Themes {
    Themes { resolve: builtins_only, saved_path: |_| None, save: |_| Err("stub".into()), selects: |_, _| false }
}

fn open(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<StudioWindow>) {
    open_with(cx, themes())
}

fn open_with(cx: &mut TestAppContext, themes: Themes) -> (AnyWindowHandle, Entity<StudioWindow>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_global(SettingsStore::for_test());
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| StudioWindow::with_themes(themes, window, cx))
        })
        .expect("opens the window")
    })
}

/// Run `f` on the Studio inside its window.
fn with<R>(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    studio: &Entity<StudioWindow>,
    f: impl FnOnce(&mut StudioWindow, &mut Window, &mut Context<StudioWindow>) -> R,
) -> R {
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| studio.update(cx, |s, cx| f(s, window, cx)))
            .expect("the window is open")
    })
}

fn window_open(cx: &mut TestAppContext, handle: AnyWindowHandle) -> bool {
    cx.update(|cx| handle.update(cx, |_, _, _| ()).is_ok())
}

fn failed(text: &str) -> Option<Notice> {
    Some(Notice::Failed(text.to_string().into()))
}

/// One edit, so the draft has something to lose.
fn edit(studio: &mut StudioWindow, window: &mut Window, cx: &mut Context<StudioWindow>) {
    studio.set_selected_value(Some("#3d59a1".into()), Edit::Field, window, cx);
    assert!(studio.dirty);
}

#[gpui_kit::test]
fn closing_a_dirty_studio_asks_first(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        assert!(studio.should_close(window, cx), "nothing to lose: it closes");
    });
    assert!(!cx.has_pending_prompt());

    let name = with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        assert!(!studio.should_close(window, cx), "unsaved edits: it asks instead");
        assert!(studio.asking);
        // Asking again while the question is up does not stack another.
        assert!(!studio.should_close(window, cx));
        studio.draft.name.clone().expect("a name")
    });
    // The question is a sheet on the window, worded as macOS words it.
    let (message, detail) = cx.pending_prompt().expect("the question");
    assert_eq!(message, format!("Do you want to save the changes to \u{201c}{name}\u{201d}?"));
    assert!(detail.starts_with("1 change. If you don\u{2019}t save, the app goes back to "), "{detail}");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
}

#[gpui_kit::test]
fn cancelling_the_question_keeps_the_edits(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.request_close(window, cx);
        assert!(studio.asking);
    });
    // Esc presses Cancel.
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, window, cx| {
        assert!(studio.dirty, "the edits are kept");
        assert!(!studio.asking, "and the next close asks again");
        assert!(!studio.should_close(window, cx));
    });
    assert!(cx.has_pending_prompt());
    assert!(window_open(cx, handle));
}

/// Save (Return) saves first. A save that fails leaves the window open, saying why.
#[gpui_kit::test]
fn save_before_closing_keeps_the_window_when_saving_fails(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.request_close(window, cx);
    });
    cx.simulate_prompt_answer("Save");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.notice, failed("Could not save: stub"), "the save was attempted");
        assert!(studio.dirty, "and failed, so the window stayed");
        assert!(!studio.asking);
    });
    assert!(window_open(cx, handle));
}

/// Don't Save closes the window, the edits with it.
#[gpui_kit::test]
fn dont_save_closes_the_window(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.request_close(window, cx);
    });
    cx.simulate_prompt_answer("Don\u{2019}t Save");
    cx.run_until_parked();
    assert!(!window_open(cx, handle), "the window closed");
}

#[gpui_kit::test]
fn an_edit_from_the_picker_leaves_the_picker_alone(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.select_token("primary", window, cx);
        // A color mid-drag: not one a hex value can say exactly.
        let dragged = hsla(0.612_34, 0.555_1, 0.442_3, 1.0);
        studio.picker.update(cx, |picker, cx| picker.set_value(dragged, window, cx));
        let hex = theme_bridge::color(dragged).to_hex();

        studio.set_selected_value(Some(hex.clone()), Edit::Picker, window, cx);
        assert!(studio.dirty);
        assert_eq!(studio.picker.read(cx).value(), Some(dragged), "the sliders were rewritten");
        assert_eq!(studio.expression.read(cx).value().as_ref(), hex.as_str());

        // From the field, the picker is pointed at the (hex) value.
        studio.set_selected_value(Some(hex.clone()), Edit::Field, window, cx);
        assert_ne!(studio.picker.read(cx).value(), Some(dragged));
    });
}

/// An unknown name is caught as it is typed, says where, and offers the name it
/// probably is; Return leaves the draft at the last good value.
#[gpui_kit::test]
fn an_unknown_token_keeps_the_last_good_value(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.select_token("state.highlight", window, cx);
        studio.check_field("primery / 20%", cx);
        let err = studio.field_error.clone().expect("an error");
        assert_eq!(err.message.as_ref(), "Unknown token \u{201c}primery\u{201d} at 1:1.");
        assert_eq!(err.suggestion, Some(("primary".to_string(), "primary / 20%".to_string())));

        studio.apply_field("primery / 20%".into(), window, cx);
        assert!(!studio.draft.ui.contains_key("state.highlight"), "the draft keeps the last good value");
        assert!(!studio.dirty);

        studio.take_suggestion("primary / 20%".into(), window, cx);
        assert_eq!(studio.draft.ui.get("state.highlight").map(String::as_str), Some("primary / 20%"));
        assert_eq!(studio.field_error, None);
        assert!(studio.dirty);
    });
}

/// A name being typed offers the names that fit it; Return or Tab takes the highlighted
/// one, which goes on to what follows it, and a finished name offers nothing more, so Return
/// applies it.
#[gpui_kit::test]
fn a_name_being_typed_is_completed(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    let labels = |studio: &StudioWindow| -> Vec<String> {
        studio
            .completion
            .as_ref()
            .map(|c| c.items.iter().map(|i| i.label.clone()).collect())
            .unwrap_or_default()
    };
    with(cx, handle, &studio, |studio, window, cx| {
        studio.select_token("state.highlight", window, cx);
        studio.expression.update(cx, |e, cx| e.set_value("prim", window, cx));
        studio.refresh_completions(cx);
        assert_eq!(labels(studio)[..2], ["primary", "primary.hover"]);
        // ↓ and ↑ move the highlight, round the ends.
        studio.step_completion(-1, cx);
        assert_eq!(studio.completion.as_ref().map(|c| c.highlighted), Some(labels(studio).len() - 1));
        studio.step_completion(1, cx);
        assert_eq!(studio.completion.as_ref().map(|c| c.highlighted), Some(0));

        studio.take_completion(None, window, cx);
        assert_eq!(studio.expression.read(cx).value().as_ref(), "primary");
        assert!(studio.completion.is_none());
    });
    // Taking a name is a change the field reports: checked, and with nothing more to offer.
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, window, cx| {
        assert_eq!(studio.field_error, None);
        assert!(studio.completion.is_none(), "a finished name offers nothing more");
        studio.apply_field("primary".into(), window, cx);
        assert_eq!(studio.draft.ui.get("state.highlight").map(String::as_str), Some("primary"));

        // Going on from a name inside an expression: only the name at the caret is replaced.
        studio.expression.update(cx, |e, cx| e.set_value("mix(primary.", window, cx));
        studio.refresh_completions(cx);
        assert_eq!(labels(studio), ["primary.hover"]);
        studio.take_completion(None, window, cx);
        assert_eq!(studio.expression.read(cx).value().as_ref(), "mix(primary.hover");

        // Esc closes the list and leaves the text; a metric never gets one.
        studio.expression.update(cx, |e, cx| e.set_value("bord", window, cx));
        studio.refresh_completions(cx);
        assert!(studio.completion.is_some());
        studio.close_completions(cx);
        assert!(studio.completion.is_none());
        assert_eq!(studio.expression.read(cx).value().as_ref(), "bord");
        studio.select_token("metrics.radius", window, cx);
        studio.expression.update(cx, |e, cx| e.set_value("prim", window, cx));
        studio.refresh_completions(cx);
        assert!(studio.completion.is_none());
    });
}

/// A value that parses but closes a loop is reported as the loop, and not applied.
#[gpui_kit::test]
fn a_reference_cycle_is_not_applied(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.select_token("primary", window, cx);
        studio.check_field("border.focus", cx);
        assert_eq!(studio.field_error, None, "each name exists");
        studio.apply_field("border.focus".into(), window, cx);
        let err = studio.field_error.clone().expect("the cycle");
        assert!(err.message.contains("primary \u{2192} border.focus \u{2192} primary"), "{}", err.message);
        assert!(studio.draft.ui.is_empty());
    });
}

/// A dark primary falls short, and the suggested fix clears every pair it causes.
#[gpui_kit::test]
fn a_contrast_fix_clears_the_dark_primary(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.select_token("primary", window, cx);
        studio.set_selected_value(Some("#3d59a1".into()), Edit::Field, window, cx);
        let tokens = builtins_only(&studio.draft).expect("resolves");
        let issues = contrast_issues(&tokens, &studio.draft.ui);
        let fix = issues.iter().find_map(|i| i.fix.clone()).expect("a fix");
        assert_eq!(fix.token, "primary");

        studio.set_value(&fix.token, Some(fix.expression), Edit::Field, window, cx);
        let tokens = builtins_only(&studio.draft).expect("resolves");
        let left: Vec<_> = contrast_issues(&tokens, &studio.draft.ui)
            .into_iter()
            .filter(|i| !i.check.fg.starts_with("terminal."))
            .collect();
        assert!(left.is_empty(), "{left:?}");
    });
}

/// A name the themes folder has is asked about in a sheet. Cancel keeps the draft as it
/// is, to be renamed in the title row; Replace writes it after all.
#[gpui_kit::test]
fn a_name_that_is_taken_asks_before_replacing(cx: &mut TestAppContext) {
    let (handle, studio) =
        open_with(cx, Themes { saved_path: |_| Some("/themes/mine.json".into()), ..themes() });
    let name = with(cx, handle, &studio, |studio, window, cx| {
        studio.request_save(window, cx);
        assert!(studio.asking);
        assert_eq!(studio.notice, None, "nothing was written yet");
        studio.draft.name.clone().expect("a name")
    });
    let (message, _) = cx.pending_prompt().expect("the question");
    assert_eq!(message, format!("\u{201c}{name}\u{201d} already exists. Do you want to replace it?"));

    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, window, cx| {
        assert_eq!(studio.notice, None);
        assert!(!studio.asking);
        // Rename edits the name in place.
        studio.start_rename(window, cx);
        assert!(studio.renaming);
        studio.name.update(cx, |input, cx| input.set_value("Dusk", window, cx));
        studio.finish_rename(window, cx);
        assert_eq!(studio.draft.name.as_deref(), Some("Dusk"));
        assert!(!studio.renaming);
        studio.request_save(window, cx);
    });
    cx.simulate_prompt_answer("Replace");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.notice, failed("Could not save: stub"), "the write was attempted");
        assert!(!studio.asking);
    });
}

#[gpui_kit::test]
fn a_new_name_is_saved_without_asking(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.request_save(window, cx);
        // The stub folder refuses, which shows the write was attempted.
        assert_eq!(studio.notice, failed("Could not save: stub"));
    });
    assert!(!cx.has_pending_prompt());
}

/// Save Copy leaves the draft and the theme in use alone; Save Copy and Switch makes the
/// copy the theme in use and the one the window edits, with nothing left to lose.
#[gpui_kit::test]
fn save_copy_and_switch_goes_on_editing_the_copy(cx: &mut TestAppContext) {
    let saving = Themes {
        save: |file| Ok(format!("/themes/{}.json", file.name.clone().unwrap_or_default()).into()),
        ..themes()
    };
    let (handle, studio) = open_with(cx, saving);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        let name = studio.draft.name.clone().expect("a name");
        studio.save_copy(false, cx);
        assert_eq!(studio.draft.name.as_deref(), Some(name.as_str()), "the draft keeps its name");
        assert!(studio.dirty, "and its edits are still unsaved");
        assert_ne!(cx.global::<SettingsStore>().settings.theme, format!("{name} copy"));

        studio.save_copy(true, cx);
        assert_eq!(studio.draft.name.as_deref(), Some(format!("{name} copy").as_str()));
        assert!(!studio.dirty);
        assert_eq!(cx.global::<SettingsStore>().settings.theme, format!("/themes/{name} copy.json"));
        assert!(studio.should_close(window, cx), "nothing left to lose");
    });
}

/// Without a terminal in a trusted project there is nowhere to put the theme: nothing is
/// written, and the notice says what is missing.
#[gpui_kit::test]
fn use_in_this_project_needs_a_project(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, _, cx| {
        assert!(studio.project_for_theme(cx).is_err());
        studio.use_in_project(cx);
        assert_eq!(studio.notice, failed("Open a terminal in a project first."));
    });
}

fn saving() -> Themes {
    Themes { save: |_| Ok("/themes/mine.json".into()), ..themes() }
}

/// Dark default, Light mode: a saved light theme is the light pick, the dark one stays.
#[gpui_kit::test]
fn save_in_light_mode_sets_the_light_pick(cx: &mut TestAppContext) {
    use wormhole_core::config::ThemeMode;
    let (handle, studio) = open_with(cx, saving());
    cx.update(|cx| {
        SettingsStore::update(cx, |s| s.theme_mode = ThemeMode::Light);
    });
    let dark_pick = cx.update(|cx| cx.global::<SettingsStore>().settings.theme.clone());
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.request_save(window, cx);
        let s = &cx.global::<SettingsStore>().settings;
        assert_eq!(s.theme_light, "/themes/mine.json");
        assert_eq!(s.theme, dark_pick, "the dark pick is not touched");
    });
}

#[gpui_kit::test]
fn saving_a_theme_that_is_a_pick_changes_no_pick(cx: &mut TestAppContext) {
    let (handle, studio) =
        open_with(cx, Themes { selects: |setting, _| setting == "Wormhole Light", ..saving() });
    let before = cx.update(|cx| {
        SettingsStore::update(cx, |s| s.theme_light = "Wormhole Light".into());
        cx.global::<SettingsStore>().settings.clone()
    });
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.request_save(window, cx);
        let s = &cx.global::<SettingsStore>().settings;
        assert_eq!((&s.theme, &s.theme_light), (&before.theme, &before.theme_light));
    });
}
