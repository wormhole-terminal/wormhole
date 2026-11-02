//! The colour control of the primary and branch rows (`select::color_pick`) on Appearance: a pop-up button
//! that says its colour to VoiceOver, ⇥ reaches it, ↩ and Space open it, the arrows walk the choices (the
//! theme's colour, the presets, Custom…), ↩ chooses, Esc closes and leaves the keyboard on it; Custom…
//! opens the editor, whose hex field applies on ↩ and refuses what isn't a colour. A colour written by hand
//! shows as it is and is never changed by being looked at.
//!
//! The window runs in a child process with a scratch data dir (`isolate_in`), like the whole-window tests.

use gpui_kit::accesskit::Role;
use gpui_kit::{AnyWindowHandle, Modifiers, TestAppContext, VisualTestContext};
use wormhole_core::config::BranchColor;
use wormhole_core::theme::folder_color::FolderColor;

use super::info_tests::draw;
use super::menu_tests::{activate, click, named, open_at, options};
use super::{Section, current_window};
use crate::settings::{SettingsStore, settings, update_settings};
use crate::window::test_harness::{isolate_in, start};

const PRIMARY: &str = "Primary colour";
const BRANCH: &str = "Branch colour";

fn open(cx: &mut TestAppContext) -> AnyWindowHandle {
    open_at(cx, Section::Appearance)
}

/// ⇥ until the control has the keyboard.
fn focus(keys: &mut VisualTestContext, handle: AnyWindowHandle, label: &str) {
    for _ in 0..120 {
        keys.simulate_keystrokes("tab");
        if named(keys, handle, label).2 {
            return;
        }
    }
    panic!("Tab reaches {label}");
}

/// A preset as the theme draws it, as the hex code a choice of it stores.
fn preset(cx: &mut TestAppContext, name: FolderColor) -> String {
    cx.update(|cx| name.color(cx.global::<SettingsStore>().base_tokens()).to_hex())
}

/// The nodes of the last frame with this role, by name.
fn nodes(cx: &mut TestAppContext, handle: AnyWindowHandle, role: Role) -> Vec<String> {
    let update = cx
        .update(|cx| handle.update(cx, |_, window, _| window.last_a11y_tree_update()))
        .expect("open")
        .expect("a tree");
    update
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == role)
        .filter_map(|(_, n)| n.label().map(str::to_string))
        .collect()
}

#[gpui_kit::test]
fn the_primary_says_its_colour_and_a_preset_is_chosen_by_keyboard(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_primary_says_its_colour_and_a_preset_is_chosen_by_keyboard") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let (_, value, _, expanded) = named(cx, handle, PRIMARY);
    assert_eq!(value, "Theme's colour", "the control tells VoiceOver its colour");
    assert_eq!(expanded, Some(false));
    let mut keys = VisualTestContext::from_window(handle, cx);
    focus(&mut keys, handle, PRIMARY);

    activate(&mut keys, "enter");
    assert_eq!(named(&mut keys, handle, PRIMARY).3, Some(true), "↩ opens it");
    let lines: Vec<String> = options(&mut keys, handle).into_iter().map(|(_, name, _)| name).collect();
    assert_eq!(lines, ["Theme's colour", "Custom…"], "the theme's colour and Custom… are lines");
    let presets = nodes(&mut keys, handle, Role::RadioButton);
    let wanted: Vec<&str> = FolderColor::ALL.iter().map(|c| c.label()).collect();
    assert!(wanted.iter().all(|name| presets.iter().any(|p| p == name)), "the eight presets: {presets:?}");

    keys.simulate_keystrokes("escape");
    assert_eq!(named(&mut keys, handle, PRIMARY).3, Some(false), "Esc closes it");
    assert!(named(&mut keys, handle, PRIMARY).2, "and the keyboard stays on it");
    assert!(keys.update(|_, cx| settings(cx).uses_theme_primary()), "closing chooses nothing");

    activate(&mut keys, "space");
    assert_eq!(named(&mut keys, handle, PRIMARY).3, Some(true), "Space opens it too");
    keys.simulate_keystrokes("down");
    keys.simulate_keystrokes("down");
    keys.simulate_keystrokes("enter");
    let orange = preset(&mut keys, FolderColor::Orange);
    let (uses_theme, stored) =
        keys.update(|_, cx| (settings(cx).uses_theme_primary(), settings(cx).primary_color.clone()));
    assert_eq!((uses_theme, stored), (false, orange), "↓ ↓ ↩ chooses the second preset");
    let (_, value, focused, expanded) = named(&mut keys, handle, PRIMARY);
    assert_eq!((value.as_str(), focused, expanded), ("Orange", true, Some(false)), "it closes, named");
}

#[gpui_kit::test]
fn a_click_chooses_the_theme_again_and_a_press_outside_closes(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_click_chooses_the_theme_again_and_a_press_outside_closes") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    cx.update(|cx| update_settings(cx, |s| s.choose_primary("#336699".into())));
    draw(cx, handle);
    let mut keys = VisualTestContext::from_window(handle, cx);
    let (id, value, ..) = named(&mut keys, handle, PRIMARY);
    assert_eq!(value, "#336699", "a colour of one's own is shown by its code");
    click(&mut keys, handle, id);
    assert_eq!(named(&mut keys, handle, PRIMARY).3, Some(true), "a click opens it");
    let theme = options(&mut keys, handle)
        .into_iter()
        .find(|(_, name, _)| name == "Theme's colour")
        .expect("a line")
        .0;
    click(&mut keys, handle, theme);
    let (uses_theme, stored) =
        keys.update(|_, cx| (settings(cx).uses_theme_primary(), settings(cx).primary_color.clone()));
    assert!(uses_theme && stored.is_empty(), "the theme's colour is the default again, nothing left over");
    assert_eq!(named(&mut keys, handle, PRIMARY).3, Some(false), "and the card closes");

    let (id, ..) = named(&mut keys, handle, PRIMARY);
    click(&mut keys, handle, id);
    keys.simulate_click(gpui_kit::point(gpui_kit::px(2.0), gpui_kit::px(400.0)), Modifiers::none());
    assert_eq!(named(&mut keys, handle, PRIMARY).3, Some(false), "a press outside closes it");
    assert!(keys.update(|_, cx| settings(cx).uses_theme_primary()), "and chooses nothing");
}

#[gpui_kit::test]
fn the_editor_applies_a_typed_code_and_refuses_what_is_not_a_colour(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_editor_applies_a_typed_code_and_refuses_what_is_not_a_colour") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let mut keys = VisualTestContext::from_window(handle, cx);
    focus(&mut keys, handle, PRIMARY);
    activate(&mut keys, "enter");
    keys.simulate_keystrokes("end");
    keys.simulate_keystrokes("enter");
    draw(&mut keys, handle);
    let sliders = nodes(&mut keys, handle, Role::Slider);
    assert!(
        sliders.iter().any(|n| n == "Saturation and brightness") && sliders.iter().any(|n| n == "Hue"),
        "Custom… opens the editor: {sliders:?}"
    );
    assert!(keys.update(|_, cx| settings(cx).uses_theme_primary()), "opening it chooses nothing");

    // The area is a slider for the keyboard: → adds a hundredth of saturation to the colour in force.
    keys.simulate_keystrokes("right");
    let moved = keys.update(|_, cx| settings(cx).primary_color.clone());
    assert!(moved.starts_with('#') && moved.len() == 7, "an arrow in the area chose a colour: {moved}");

    // ⇥ from the area to the hue bar, then the hex field.
    keys.simulate_keystrokes("tab");
    keys.simulate_keystrokes("tab");
    keys.simulate_keystrokes("cmd-a");
    keys.simulate_input("#112233");
    keys.simulate_keystrokes("enter");
    assert_eq!(keys.update(|_, cx| settings(cx).primary_color.clone()), "#112233", "↩ applies a typed code");

    keys.simulate_keystrokes("cmd-a");
    keys.simulate_input("zzz");
    keys.simulate_keystrokes("enter");
    assert_eq!(
        keys.update(|_, cx| settings(cx).primary_color.clone()),
        "#112233",
        "what isn't a colour is not applied"
    );
    let page = keys.update(|_, cx| current_window(cx)).expect("the window").1;
    assert!(keys.update(|_, cx| page.read(cx).primary_pick_says_bad(cx)), "and the field says so");
    keys.simulate_keystrokes("backspace");
    assert!(
        !keys.update(|_, cx| page.read(cx).primary_pick_says_bad(cx)),
        "the message goes with the next key"
    );

    // Colours are never capped: black and white are colours.
    for code in ["#000000", "#ffffff"] {
        keys.simulate_keystrokes("cmd-a");
        keys.simulate_input(code);
        keys.simulate_keystrokes("enter");
        assert_eq!(keys.update(|_, cx| settings(cx).primary_color.clone()), code);
    }
}

#[gpui_kit::test]
fn every_stored_colour_shows_and_a_hand_set_one_is_left_alone(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "every_stored_colour_shows_and_a_hand_set_one_is_left_alone") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let teal = preset(cx, FolderColor::Teal);
    for (stored, shown) in
        [(teal.as_str(), "Teal"), ("#123456", "#123456"), ("#FFFFFF", "#FFFFFF"), ("orange", "orange")]
    {
        cx.update(|cx| {
            update_settings(cx, |s| {
                s.primary_color = stored.into();
                s.theme_primary = Some(false);
            })
        });
        draw(cx, handle);
        assert_eq!(named(cx, handle, PRIMARY).1, shown, "{stored}");
    }
    // Looking at it, even opening the card on it, never rewrites it.
    let mut keys = VisualTestContext::from_window(handle, cx);
    let (id, ..) = named(&mut keys, handle, PRIMARY);
    click(&mut keys, handle, id);
    keys.simulate_keystrokes("escape");
    assert_eq!(keys.update(|_, cx| settings(cx).primary_color.clone()), "orange");

    // The branch colour: the theme's, the primary, a preset or its own.
    for (branch, custom, shown) in [
        (BranchColor::Theme, "", "Theme's colour"),
        (BranchColor::Primary, "", "Primary"),
        (BranchColor::Custom, teal.as_str(), "Teal"),
        (BranchColor::Custom, "#abcdef", "#abcdef"),
        (BranchColor::Custom, "", "Custom"),
    ] {
        cx.update(|cx| {
            update_settings(cx, |s| {
                s.branch_color = branch;
                s.branch_color_custom = custom.into();
            })
        });
        draw(cx, handle);
        assert_eq!(named(cx, handle, BRANCH).1, shown, "{branch:?} {custom}");
    }
}

#[gpui_kit::test]
fn the_branch_can_follow_the_primary_and_come_back_to_the_theme(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "the_branch_can_follow_the_primary_and_come_back_to_the_theme") {
        return;
    }
    start(cx, 1);
    let handle = open(cx);
    let mut keys = VisualTestContext::from_window(handle, cx);
    let (id, ..) = named(&mut keys, handle, BRANCH);
    click(&mut keys, handle, id);
    let lines: Vec<String> = options(&mut keys, handle).into_iter().map(|(_, name, _)| name).collect();
    assert_eq!(lines, ["Theme's colour", "Primary", "Custom…"], "the branch has one more way: the primary");
    let primary =
        options(&mut keys, handle).into_iter().find(|(_, name, _)| name == "Primary").expect("a line").0;
    click(&mut keys, handle, primary);
    assert_eq!(keys.update(|_, cx| settings(cx).branch_color), BranchColor::Primary);
    assert_eq!(named(&mut keys, handle, BRANCH).1, "Primary");

    let (id, ..) = named(&mut keys, handle, BRANCH);
    click(&mut keys, handle, id);
    keys.simulate_keystrokes("home");
    keys.simulate_keystrokes("enter");
    let (branch, custom) =
        keys.update(|_, cx| (settings(cx).branch_color, settings(cx).branch_color_custom.clone()));
    assert_eq!((branch, custom.as_str()), (BranchColor::Theme, ""), "back at the default, nothing left over");
}
