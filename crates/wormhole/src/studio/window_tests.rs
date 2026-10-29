//! The Studio window itself, in a headless GPUI (needs `devctl`'s test support). Drafts
//! resolve against the built-in themes only, so no test reads the user's themes folder. The
//! sheets are the platform's: the test platform queues them, and `simulate_prompt_answer`
//! presses a button by its label.

use gpui_kit::{
    AnyWindowHandle, AppContext as _, Context, Entity, TestAppContext, Window, WindowOptions, hsla,
};
use wormhole_core::theme::tokens::{ThemeFile, ThemeTokens, contrast_issues, resolve};

use super::document::{Document, Subject};
use super::opening::{Inputs, open_document};
use super::save::Notice;
use super::themes::{Found, SaveError, Saved, Themes};
use super::{Edit, StudioWindow};
use crate::settings::SettingsStore;
use crate::theme_bridge;

fn builtins_only(file: &ThemeFile) -> Result<ThemeTokens, String> {
    resolve(file, &|_| None)
}

/// A themes folder that is not the user's: no file is read, and "saving" fails with a
/// word the tests recognise.
pub(super) fn themes() -> Themes {
    Themes {
        resolve: builtins_only,
        saved_path: |_| None,
        catalog: Vec::new,
        locate: |setting| Found::Builtin(setting.to_string()),
        read: |_| Err("stub".into()),
        save_new: |_| Err("stub".into()),
        overwrite: |_, _, _| Err(SaveError::Other("stub".into())),
        new_id: |name, _| wormhole_core::theme::tokens::slug::derive_id(name),
        convert: |file, id| {
            wormhole_core::theme::tokens::copy::convert_to_v1(
                file,
                &|_| None,
                &|n| wormhole_core::theme::tokens::slug::derive_id(n),
                id,
                "2026-10-29",
            )
        },
        selects: |_, _| false,
    }
}

/// The Studio opens on the built-in in use, as "Untitled (from Wormhole Dark)".
pub(super) fn opened(themes: &Themes, cx: &gpui_kit::App) -> Document {
    let in_use = cx.global::<SettingsStore>().base_tokens().clone();
    open_document(&Inputs { shown: in_use.name.clone(), project: None, in_use }, themes)
}

fn open(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<StudioWindow>) {
    open_with(cx, themes())
}

fn open_with(cx: &mut TestAppContext, themes: Themes) -> (AnyWindowHandle, Entity<StudioWindow>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_global(SettingsStore::for_test());
        let doc = opened(&themes, cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| StudioWindow::with_themes(themes, doc, window, cx))
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
    assert!(studio.dirty());
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
        studio.doc.file.name.clone().expect("a name")
    });
    // The question is a sheet on the window, worded as macOS words it.
    let (message, detail) = cx.pending_prompt().expect("the question");
    assert_eq!(message, format!("Do you want to save the changes to \u{201c}{name}\u{201d}?"));
    assert_eq!(detail, "1 change. Your changes are lost if you don\u{2019}t save them.");
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
        assert!(studio.dirty(), "the edits are kept");
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
    // A copy with no file yet is named first; the window waits for the name.
    with(cx, handle, &studio, |studio, window, cx| {
        assert_eq!(studio.naming, Some(true));
        assert!(studio.renaming && !studio.asking);
        studio.commit_name(window, cx);
        assert_eq!(studio.notice, failed("Could not save: stub"), "the save was attempted");
        assert!(studio.dirty(), "and failed, so the window stayed");
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
        assert!(studio.dirty());
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
        assert!(!studio.doc.file.ui.contains_key("state.highlight"), "the draft keeps the last good value");
        assert!(!studio.dirty());

        studio.take_suggestion("primary / 20%".into(), window, cx);
        assert_eq!(studio.doc.file.ui.get("state.highlight").map(String::as_str), Some("primary / 20%"));
        assert_eq!(studio.field_error, None);
        assert!(studio.dirty());
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
        assert_eq!(studio.doc.file.ui.get("state.highlight").map(String::as_str), Some("primary"));

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
        assert!(studio.doc.file.ui.is_empty());
    });
}

/// A dark primary falls short, and the suggested fix clears every pair it causes.
#[gpui_kit::test]
fn a_contrast_fix_clears_the_dark_primary(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.select_token("primary", window, cx);
        studio.set_selected_value(Some("#3d59a1".into()), Edit::Field, window, cx);
        let tokens = builtins_only(&studio.doc.file).expect("resolves");
        let issues = contrast_issues(&tokens, &studio.doc.file.ui);
        let fix = issues.iter().find_map(|i| i.fix.clone()).expect("a fix");
        assert_eq!(fix.token, "primary");

        studio.set_value(&fix.token, Some(fix.expression), Edit::Field, window, cx);
        let tokens = builtins_only(&studio.doc.file).expect("resolves");
        let left: Vec<_> = contrast_issues(&tokens, &studio.doc.file.ui)
            .into_iter()
            .filter(|i| !i.check.fg.starts_with("terminal."))
            .collect();
        assert!(left.is_empty(), "{left:?}");
    });
}

/// The first save of a copy asks for its name in the title row: Return gives it and saves.
fn name_it(studio: &mut StudioWindow, name: &str, window: &mut Window, cx: &mut Context<StudioWindow>) {
    studio.request_save(window, cx);
    assert!(studio.renaming && studio.naming.is_some(), "the name is asked for");
    studio.name.update(cx, |input, cx| input.set_value(name, window, cx));
    studio.commit_name(window, cx);
}

/// A name the themes folder has is asked about in a sheet. Cancel keeps the copy as it is, to be
/// named again; Replace writes it after all.
#[gpui_kit::test]
fn a_name_that_is_taken_asks_before_replacing(cx: &mut TestAppContext) {
    let (handle, studio) =
        open_with(cx, Themes { saved_path: |_| Some("/themes/mine.json".into()), ..themes() });
    with(cx, handle, &studio, |studio, window, cx| {
        name_it(studio, "Dusk", window, cx);
        assert!(studio.asking);
        assert_eq!(studio.notice, None, "nothing was written yet");
    });
    let (message, _) = cx.pending_prompt().expect("the question");
    assert_eq!(message, "\u{201c}Dusk\u{201d} already exists. Do you want to replace it?");

    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, window, cx| {
        assert_eq!(studio.notice, None);
        assert!(!studio.asking);
        assert_eq!(studio.doc.name(), "Dusk");
        name_it(studio, "Dusk 2", window, cx);
        assert!(studio.asking, "the name is taken as well");
    });
    cx.simulate_prompt_answer("Replace");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.notice, failed("Could not save: stub"), "the write was attempted");
        assert!(!studio.asking);
    });
}

#[gpui_kit::test]
fn the_first_save_of_an_untitled_theme_asks_for_a_name_then_writes_id_and_base(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, saving());
    with(cx, handle, &studio, |studio, window, cx| {
        assert_eq!(studio.doc.name(), "Untitled (from Wormhole Dark)");
        assert!(!studio.dirty(), "not dirty until the first edit");
        assert!(studio.should_close(window, cx), "nothing to lose");
        studio.request_save(window, cx);
        assert_eq!(studio.notice, None, "nothing is written before the name");
        assert!(studio.renaming && studio.naming == Some(false));
        studio.name.update(cx, |input, cx| input.set_value("Dusk", window, cx));
        studio.commit_name(window, cx);
        assert_eq!(studio.doc.name(), "Dusk");
        assert_eq!(studio.doc.file.id.as_deref(), Some("dusk"));
        assert!(studio.doc.file.base.is_some());
        assert_eq!(studio.doc.subject, Subject::Own { path: "/themes/mine.json".into() });
        assert!(!studio.dirty());
        assert!(matches!(studio.notice, Some(Notice::Saved { .. })), "{:?}", studio.notice);
    });
    assert!(!cx.has_pending_prompt());
}

/// Esc in the name field leaves the copy unsaved.
#[gpui_kit::test]
fn cancelling_the_name_saves_nothing(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, saving());
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.request_save(window, cx);
        studio.cancel_rename(window, cx);
        assert_eq!((studio.naming, studio.renaming, studio.notice.clone()), (None, false, None));
        assert!(studio.dirty());
    });
}

/// A theme with a file of its own is saved over it with no question, under the same file name.
#[gpui_kit::test]
fn saving_your_own_theme_writes_over_its_file_without_asking(cx: &mut TestAppContext) {
    let own = Themes {
        locate: |_| Found::File("/themes/dusk.json".into()),
        read: |path| {
            let file =
                wormhole_core::theme::tokens::parse_theme_file(r##"{"name": "Dusk", "extends": "Nord"}"##)?;
            assert_eq!(path, std::path::Path::new("/themes/dusk.json"));
            Ok(super::themes::Loaded { file, stamp: None })
        },
        overwrite: |path, file, _| {
            assert_eq!(file.name.as_deref(), Some("Dusk 2"), "a rename changes the name only");
            Ok(Saved { path: path.to_path_buf(), stamp: None, lost_comments: false })
        },
        ..themes()
    };
    let (handle, studio) = open_with(cx, own);
    with(cx, handle, &studio, |studio, window, cx| {
        assert_eq!(studio.doc.subject, Subject::Own { path: "/themes/dusk.json".into() });
        assert!(!studio.dirty());
        edit(studio, window, cx);
        studio.start_rename(window, cx);
        studio.name.update(cx, |input, cx| input.set_value("Dusk 2", window, cx));
        studio.finish_rename(window, cx);
        studio.request_save(window, cx);
        assert!(!studio.asking && !studio.renaming);
        assert!(!studio.dirty());
        let Some(Notice::Saved { path, .. }) = studio.notice.clone() else { panic!("{:?}", studio.notice) };
        assert_eq!(path, std::path::PathBuf::from("/themes/dusk.json"), "the file name stays");
    });
    assert!(!cx.has_pending_prompt());
}

/// A file another program wrote since it was read is not overwritten without a question.
#[gpui_kit::test]
fn a_file_changed_on_disk_asks_before_it_is_overwritten(cx: &mut TestAppContext) {
    use std::cell::Cell;
    thread_local!(static WRITES: Cell<usize> = const { Cell::new(0) });
    let own = Themes {
        locate: |_| Found::File("/themes/dusk.json".into()),
        read: |_| {
            let file = wormhole_core::theme::tokens::parse_theme_file(r##"{"name": "Dusk"}"##)?;
            Ok(super::themes::Loaded { file, stamp: None })
        },
        overwrite: |path, _, expected| {
            WRITES.with(|w| w.set(w.get() + 1));
            // Changed once; the answer Overwrite writes without the check.
            if WRITES.with(Cell::get) == 1 && expected.is_none() {
                return Err(SaveError::ChangedOnDisk);
            }
            Ok(Saved { path: path.to_path_buf(), stamp: None, lost_comments: false })
        },
        ..themes()
    };
    let (handle, studio) = open_with(cx, own);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.request_save(window, cx);
        assert!(studio.asking && studio.dirty());
    });
    let (message, _) = cx.pending_prompt().expect("the question");
    assert_eq!(message, "\u{201c}Dusk\u{201d} was changed by another application.");
    cx.simulate_prompt_answer("Overwrite");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, _, _| assert!(!studio.dirty() && !studio.asking));
}

/// Duplicate writes the theme, edits and all, as "… copy" and goes on editing that; the picks
/// stay, and the copy has nothing left to lose.
#[gpui_kit::test]
fn duplicate_goes_on_editing_the_copy_and_changes_no_pick(cx: &mut TestAppContext) {
    let saving = Themes {
        save_new: |file| {
            Ok(Saved {
                path: format!("/themes/{}.json", file.name.clone().unwrap_or_default()).into(),
                stamp: None,
                lost_comments: false,
            })
        },
        ..themes()
    };
    let (handle, studio) = open_with(cx, saving);
    let before = cx.update(|cx| cx.global::<SettingsStore>().settings.clone());
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        let name = studio.doc.name().to_string();
        studio.duplicate(window, cx);
        assert_eq!(studio.doc.name(), format!("{name} copy"));
        assert_eq!(
            studio.doc.file.ui.get("primary").map(String::as_str),
            Some("#3d59a1"),
            "the edits travel"
        );
        assert!(!studio.dirty());
        assert!(matches!(studio.doc.subject, Subject::Own { .. }));
        assert_eq!(studio.history.undo_len(), 0, "the copy has a history of its own");
        let s = &cx.global::<SettingsStore>().settings;
        assert_eq!((&s.theme, &s.theme_light), (&before.theme, &before.theme_light));
        assert!(studio.should_close(window, cx), "nothing left to lose");
    });
}

fn own_themes() -> Themes {
    Themes {
        catalog: || wormhole_core::theme::catalog::catalog_in(std::path::Path::new("/no/such/themes/folder")),
        read: |path| {
            let file =
                wormhole_core::theme::tokens::parse_theme_file(r##"{"name": "Dusk", "extends": "Nord"}"##)?;
            assert_eq!(path, std::path::Path::new("/themes/dusk.json"));
            Ok(super::themes::Loaded { file, stamp: None })
        },
        ..themes()
    }
}

/// Choose a theme of the open theme menu, as a click on it would.
fn pick_from_menu(cx: &mut TestAppContext, handle: AnyWindowHandle, title: &str) {
    let (_, callbacks) = crate::platform::record::pick(title).expect("the item is there");
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                for callback in &callbacks {
                    callback(window, cx);
                }
            })
            .expect("the window is open");
    });
}

#[gpui_kit::test]
fn the_name_opens_the_theme_menu_and_a_choice_opens_that_theme(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, own_themes());
    with(cx, handle, &studio, |studio, window, cx| studio.open_theme_menu(window, cx));
    let menu = crate::platform::record::open().expect("the menu opened");
    let outline = menu.outline();
    assert!(
        outline.contains(&"# Built-in".to_string()) && outline.contains(&"- New Theme\u{2026}".to_string())
    );
    assert!(outline.contains(&"- Revert to Saved [off]".to_string()), "no edits: {outline:?}");
    pick_from_menu(cx, handle, "Nord");
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.doc.subject, Subject::Untitled { from: "Nord".into() });
        assert_eq!(studio.doc.name(), "Untitled (from Nord)");
        assert!(!studio.dirty());
    });
    assert!(!cx.has_pending_prompt(), "nothing to lose: no question");
}

/// With edits, choosing another theme asks Save, Cancel or Don't Save.
#[gpui_kit::test]
fn switching_with_edits_asks_save_dont_save_cancel(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, own_themes());
    let before = cx.update(|cx| cx.global::<SettingsStore>().settings.clone());
    let choose_nord = |cx: &mut TestAppContext| {
        with(cx, handle, &studio, |studio, window, cx| {
            if !studio.dirty() {
                edit(studio, window, cx);
            }
            studio.open_theme_menu(window, cx);
        });
        pick_from_menu(cx, handle, "Nord");
    };
    // Cancel: the window stays on its theme with its edits.
    choose_nord(cx);
    let (message, detail) = cx.pending_prompt().expect("the question");
    assert_eq!(
        message,
        "Do you want to save the changes to \u{201c}Untitled (from Wormhole Dark)\u{201d} before opening \u{201c}Nord\u{201d}?"
    );
    assert_eq!(detail, "1 change. Your changes are lost if you don\u{2019}t save them.");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, _, _| {
        assert!(studio.dirty() && !studio.asking);
        assert_eq!(studio.doc.name(), "Untitled (from Wormhole Dark)");
    });
    // Don't Save: Nord opens, the edits are gone, the history is fresh.
    choose_nord(cx);
    cx.simulate_prompt_answer("Don\u{2019}t Save");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, _, _| {
        assert_eq!(studio.doc.name(), "Untitled (from Nord)");
        assert!(!studio.dirty());
        assert_eq!(studio.history.undo_len(), 0);
    });
    // Save: a copy with no file yet is named first, and Nord waits for that to go through.
    choose_nord(cx);
    cx.simulate_prompt_answer("Save");
    cx.run_until_parked();
    with(cx, handle, &studio, |studio, _, _| {
        assert!(studio.renaming && studio.dirty(), "the name is asked for; the switch waits");
    });
    let after = cx.update(|cx| cx.global::<SettingsStore>().settings.clone());
    assert_eq!((&after.theme, &after.theme_light), (&before.theme, &before.theme_light), "no pick changed");
}

#[gpui_kit::test]
fn switching_without_edits_does_not_ask(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, own_themes());
    with(cx, handle, &studio, |studio, window, cx| {
        studio.open_target(&super::opening::Target::Own("/themes/dusk.json".into()), window, cx);
        assert_eq!(studio.doc.subject, Subject::Own { path: "/themes/dusk.json".into() });
        assert!(!studio.asking);
    });
    assert!(!cx.has_pending_prompt());
}

/// Rename changes the display name only: the file stays, and a pick that named the theme by its
/// old name names its file instead of losing it.
#[gpui_kit::test]
fn renaming_keeps_the_file_and_the_pick(cx: &mut TestAppContext) {
    let renamed = Themes {
        overwrite: |path, file, _| {
            assert_eq!(file.name.as_deref(), Some("Dusk 2"));
            Ok(Saved { path: path.to_path_buf(), stamp: None, lost_comments: false })
        },
        ..own_themes()
    };
    let (handle, studio) = open_with(cx, renamed);
    cx.update(|cx| SettingsStore::update(cx, |s| s.theme = "dusk".into()));
    with(cx, handle, &studio, |studio, window, cx| {
        studio.open_target(&super::opening::Target::Own("/themes/dusk.json".into()), window, cx);
        studio.start_rename(window, cx);
        studio.name.update(cx, |input, cx| input.set_value("Dusk 2", window, cx));
        studio.finish_rename(window, cx);
        assert_eq!(studio.history.undo_label(), Some(&super::document::history::StepLabel::Rename));
        studio.request_save(window, cx);
        assert_eq!(studio.doc.subject, Subject::Own { path: "/themes/dusk.json".into() });
        assert_eq!(cx.global::<SettingsStore>().settings.theme, "/themes/dusk.json");
    });
}

/// Revert to Saved is one step to undo.
#[gpui_kit::test]
fn revert_to_saved_is_one_undoable_step(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.discard_changes(window, cx);
        assert!(!studio.dirty());
        studio.undo(window, cx);
        assert!(studio.dirty(), "the edit is back");
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
    Themes {
        save_new: |_| Ok(Saved { path: "/themes/mine.json".into(), stamp: None, lost_comments: false }),
        ..themes()
    }
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
        name_it(studio, "Mine", window, cx);
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
        name_it(studio, "Mine", window, cx);
        let s = &cx.global::<SettingsStore>().settings;
        assert_eq!((&s.theme, &s.theme_light), (&before.theme, &before.theme_light));
    });
}

/// Undo puts the file back and redo returns it; back at the saved file nothing is unsaved.
#[gpui_kit::test]
fn undo_and_redo_walk_the_edits_and_clear_unsaved(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.set_selected_value(Some("#aa0000".into()), Edit::Field, window, cx);
        assert_eq!(studio.history.undo_len(), 2);
        studio.undo(window, cx);
        assert_eq!(studio.doc.file.ui.get("primary").map(String::as_str), Some("#3d59a1"));
        studio.undo(window, cx);
        assert!(!studio.dirty(), "undone to the saved file");
        studio.redo(window, cx);
        assert_eq!(studio.doc.file.ui.get("primary").map(String::as_str), Some("#3d59a1"));
        assert!(studio.dirty());
        // A new edit drops what could be redone.
        studio.set_selected_value(Some("#00aa00".into()), Edit::Field, window, cx);
        assert_eq!(studio.history.redo_len(), 0);
    });
}

/// The picker and the slider report every value on the way; a drag is one step.
#[gpui_kit::test]
fn a_picker_drag_is_one_undo_step(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        for hex in ["#111111", "#222222", "#333333"] {
            studio.set_selected_value(Some(hex.into()), Edit::Picker, window, cx);
        }
        assert_eq!(studio.history.undo_len(), 1);
        studio.end_gesture(cx);
        studio.set_selected_value(Some("#444444".into()), Edit::Picker, window, cx);
        assert_eq!(studio.history.undo_len(), 2);
        studio.undo(window, cx);
        assert_eq!(studio.doc.file.ui.get("primary").map(String::as_str), Some("#333333"));
        studio.undo(window, cx);
        assert!(!studio.dirty());
    });
}

#[gpui_kit::test]
fn undo_survives_save(cx: &mut TestAppContext) {
    let (handle, studio) = open_with(cx, saving());
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        name_it(studio, "Mine", window, cx);
        assert!(!studio.dirty());
        assert_eq!(studio.history.undo_len(), 2, "saving keeps the history: the edit and the name");
        studio.undo(window, cx);
        assert!(studio.dirty(), "the saved file now differs from the undone one");
    });
}

/// A field with typing in it keeps ⌘Z for itself; otherwise the file is undone.
#[gpui_kit::test]
fn undo_in_a_field_undoes_typing_then_the_step(cx: &mut TestAppContext) {
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        edit(studio, window, cx);
        studio.expression.update(cx, |e, cx| e.focus(window, cx));
        studio.typing.expression = true;
        studio.route_undo(false, window, cx);
        assert_eq!(studio.history.undo_len(), 1, "the field takes it");
        studio.typing.expression = false;
        studio.route_undo(false, window, cx);
        assert_eq!(studio.history.undo_len(), 0, "the step is undone");
        assert!(!studio.dirty());
    });
}

/// The Edit menu says what Undo would undo, while this window is the one in front.
#[gpui_kit::test]
fn the_edit_menu_titles_follow_the_history(cx: &mut TestAppContext) {
    use crate::window::menu_state::EditTitles;
    let (handle, studio) = open(cx);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.active = true;
        edit(studio, window, cx);
    });
    cx.run_until_parked();
    let titles = cx.update(|cx| cx.try_global::<EditTitles>().cloned()).expect("titles were set");
    assert_eq!(titles.undo.as_deref(), Some("Change \u{201c}primary\u{201d}"));
    assert_eq!(titles.redo, None);
    with(cx, handle, &studio, |studio, window, cx| studio.undo(window, cx));
    cx.run_until_parked();
    let titles = cx.update(|cx| cx.global::<EditTitles>().clone());
    assert_eq!((titles.undo, titles.redo.as_deref()), (None, Some("Change \u{201c}primary\u{201d}")));
}

/// Saving a theme in the older format writes the new one: its base copied from what it extends, its
/// own values on top, an id; the picks that named it by name name its id; the notice says what changed.
#[gpui_kit::test]
fn the_first_save_of_an_older_theme_writes_the_new_format(cx: &mut TestAppContext) {
    use std::cell::RefCell;
    thread_local!(static WRITTEN: RefCell<Option<ThemeFile>> = const { RefCell::new(None) });
    let own = Themes {
        overwrite: |path, file, _| {
            WRITTEN.with(|w| *w.borrow_mut() = Some(file.clone()));
            Ok(Saved { path: path.to_path_buf(), stamp: None, lost_comments: true })
        },
        ..own_themes()
    };
    let (handle, studio) = open_with(cx, own);
    cx.update(|cx| SettingsStore::update(cx, |s| s.theme_light = "Dusk".into()));
    with(cx, handle, &studio, |studio, window, cx| {
        studio.open_target(&super::opening::Target::Own("/themes/dusk.json".into()), window, cx);
        assert_eq!(studio.doc.file.version, None, "read as the older format");
        let before = builtins_only(&studio.doc.file).expect("resolves");
        studio.set_value("primary", Some("#aa0000".into()), Edit::Field, window, cx);
        let edited = builtins_only(&studio.doc.file).expect("resolves");
        studio.request_save(window, cx);
        let written = WRITTEN.with(|w| w.borrow().clone()).expect("written");
        assert_eq!((written.version, written.id.as_deref()), (Some(1), Some("dusk")));
        assert_eq!(written.base.as_ref().map(|b| b.from.as_str()), Some("nord"));
        assert_eq!(
            written.ui.get("primary").map(String::as_str),
            Some("#aa0000"),
            "the person's own value stays on top"
        );
        assert_eq!(builtins_only(&written).expect("resolves"), edited, "nothing on screen changes");
        assert_ne!(before, edited);
        assert_eq!(studio.doc.file, written);
        assert!(!studio.dirty());
        assert_eq!(cx.global::<SettingsStore>().settings.theme_light, "dusk", "the pick follows to the id");
        let Some(Notice::Saved { text, .. }) = studio.notice.clone() else { panic!("{:?}", studio.notice) };
        assert!(text.contains("older releases of Wormhole can\u{2019}t open this file"), "{text}");
        assert!(text.contains("Comments in the file were not kept."), "{text}");
    });
}

/// A conversion that would change how the theme looks is not written.
#[gpui_kit::test]
fn a_conversion_that_changes_the_look_is_not_written(cx: &mut TestAppContext) {
    let own = Themes {
        convert: |file, id| {
            let mut converted = wormhole_core::theme::tokens::copy::convert_to_v1(
                file,
                &|_| None,
                &|n| wormhole_core::theme::tokens::slug::derive_id(n),
                id,
                "2026-10-29",
            )?;
            converted.ui.insert("primary".into(), "#010203".into());
            Ok(converted)
        },
        overwrite: |_, _, _| panic!("nothing may be written"),
        ..own_themes()
    };
    let (handle, studio) = open_with(cx, own);
    with(cx, handle, &studio, |studio, window, cx| {
        studio.open_target(&super::opening::Target::Own("/themes/dusk.json".into()), window, cx);
        studio.set_value("border", Some("#aa0000".into()), Edit::Field, window, cx);
        studio.request_save(window, cx);
        assert_eq!(studio.notice, failed("Could not convert this theme safely, so it was not changed."),);
        assert!(studio.dirty() && studio.doc.file.version.is_none());
    });
}
