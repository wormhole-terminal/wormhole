//! The store's derivations: what a change re-derives, and what it leaves alone.

use std::rc::Rc;
use std::sync::Arc;

use wormhole_core::config::{BranchColor, Config, Settings, ThemeMode};
use wormhole_core::theme::tokens::{Appearance, Translucency};

use super::derive::{OPAQUE_LOOK, TALLY};
use super::{SettingsStore, SystemLook, UiColors};

fn store() -> SettingsStore {
    SettingsStore::new(Config::placeholder())
}

/// The path `update` takes from the settings window, minus the GPUI global (the
/// placeholder config saves nowhere).
fn update(store: &mut SettingsStore, f: impl FnOnce(&mut Settings)) {
    store.config.update(f);
    store.derive();
}

fn tally() -> std::collections::BTreeMap<&'static str, usize> {
    TALLY.with(|t| t.borrow().clone())
}

fn reset_tally() {
    TALLY.with(|t| t.borrow_mut().clear());
}

#[test]
fn a_setting_outside_the_theme_derives_nothing() {
    let mut store = store();
    let (tokens, palette, highlight) = (store.tokens.clone(), store.palette.clone(), store.highlight.clone());
    let (settings, theme) = (store.revisions.settings, store.revisions.theme);
    let revision = store.revision;
    reset_tally();

    update(&mut store, |s| s.diff_wrap = !s.diff_wrap);

    assert!(Arc::ptr_eq(&tokens, &store.tokens), "the tokens were rebuilt");
    assert!(Rc::ptr_eq(&palette, &store.palette), "the palette was rebuilt");
    assert!(Arc::ptr_eq(&highlight, &store.highlight), "the syntax theme was rebuilt");
    assert_eq!(store.revisions.theme, theme);
    assert_eq!(store.revisions.settings, settings + 1);
    assert_eq!(store.revision, revision + 1);
    // No theme was derived: the old code derived the palette and syntax theme (and
    // the whole token set around a primary) on every one of these.
    assert_eq!(tally().get("theme_parts"), None);
    assert_eq!(tally().get("with_choices"), None);
    // The bundled icon theme is only resolved when the name or appearance changes.
    assert_eq!(tally().get("icons"), None);
}

#[test]
fn saving_the_same_settings_changes_nothing() {
    let mut store = store();
    update(&mut store, |s| s.diff_wrap = !s.diff_wrap);
    let revision = store.revision;
    let revisions = store.revisions;
    update(&mut store, |_| {});
    assert_eq!(store.revision, revision);
    assert_eq!(store.revisions, revisions);
}

#[test]
fn choosing_a_primary_derives_the_theme_once_and_remembers_it() {
    let mut store = store();
    reset_tally();
    update(&mut store, |s| s.choose_primary("#ff8800".into()));
    assert_eq!(tally().get("with_choices"), Some(&1));
    assert_eq!(tally().get("theme_parts"), Some(&1));
    assert!(store.tokens.get("primary").is_some_and(|c| c.to_hex() == "#ff8800"));
    let theme = store.revisions.theme;
    let tokens = store.tokens.clone();

    // Other settings change while the primary stays: nothing is derived again.
    update(&mut store, |s| s.tab_width = 8);
    update(&mut store, |s| s.opacity = 0.9);
    assert_eq!(tally().get("with_choices"), Some(&1));
    assert_eq!(tally().get("theme_parts"), Some(&1));
    assert!(Arc::ptr_eq(&tokens, &store.tokens));
    assert_eq!(store.revisions.theme, theme);

    // Another primary is another derivation; clearing it goes back to the theme's own.
    update(&mut store, |s| s.choose_primary("#0088ff".into()));
    assert_eq!(tally().get("with_choices"), Some(&2));
    update(&mut store, |s| s.choose_primary(String::new()));
    assert_eq!(tally().get("with_choices"), Some(&2));
    assert!(Arc::ptr_eq(&store.tokens, &store.config_tokens));
}

#[test]
fn a_branch_color_in_the_primary_follows_it_and_the_theme_comes_back() {
    let mut store = store();
    reset_tally();
    update(&mut store, |s| s.branch_color = BranchColor::Primary);
    assert_eq!(tally().get("with_choices"), Some(&1));
    let primary = store.tokens.get("primary").map(|c| c.to_hex());
    assert!(primary.is_some() && store.tokens.get("git.branch").map(|c| c.to_hex()) == primary);

    // A primary of the person's own takes the branch along in the same derivation.
    update(&mut store, |s| s.choose_primary("#ff8800".into()));
    assert_eq!(tally().get("with_choices"), Some(&2));
    assert!(store.tokens.get("git.branch").is_some_and(|c| c.to_hex() == "#ff8800"));

    // The theme's own again: no derivation, the theme as loaded.
    update(&mut store, |s| {
        s.branch_color = BranchColor::Theme;
        s.choose_primary(String::new());
    });
    assert_eq!(tally().get("with_choices"), Some(&2));
    assert!(Arc::ptr_eq(&store.tokens, &store.config_tokens));
}

#[test]
fn the_opacity_alone_refreshes_the_interface_colors() {
    let mut store = store();
    let palette = store.palette.clone();
    let before = store.ui.background.a;
    update(&mut store, |s| s.opacity = 0.5);
    assert!(store.ui.background.a < before);
    assert!(Rc::ptr_eq(&palette, &store.palette));
}

fn system(store: &mut SettingsStore, look: SystemLook) {
    store.take_system(look);
    store.derive();
}

const LIGHT: SystemLook =
    SystemLook { appearance: Appearance::Light, reduce_transparency: false, increase_contrast: false };
const REDUCED: SystemLook =
    SystemLook { appearance: Appearance::Dark, reduce_transparency: true, increase_contrast: false };
const CONTRAST: SystemLook =
    SystemLook { appearance: Appearance::Dark, reduce_transparency: false, increase_contrast: true };

/// Reduce Transparency: the windows are opaque whatever the user's opacity, which
/// stays theirs and comes back with the translucency.
#[test]
fn reduce_transparency_makes_every_window_opaque() {
    let mut store = store();
    // A theme whose muted text is lifted at 80 %: the default's is dark enough without.
    update(&mut store, |s| (s.theme, s.opacity) = ("Tokyo Night".into(), 0.8));
    assert!(store.ui.panel.a < 1.0);
    let translucent_muted = store.ui.muted;

    system(&mut store, REDUCED);
    assert_eq!(store.settings.opacity, 1.0);
    assert_eq!((store.user_settings().opacity, store.asked_opacity()), (0.8, 0.8));
    assert_eq!((store.ui.background.a, store.ui.panel.a, store.ui.panel_right.a), (1.0, 1.0, 1.0));
    // The text is the opaque window's again.
    assert_eq!(store.ui.muted, crate::theme_bridge::token(&store.tokens, "text.muted"));
    assert_ne!(store.ui.muted, translucent_muted);

    system(&mut store, SystemLook::default());
    assert_eq!(store.settings.opacity, 0.8);
    assert_eq!(store.ui.muted, translucent_muted);
}

/// Column dividers off draws no line between the main window's columns whatever the theme
/// says, and no other window's; Increase Contrast keeps the lines; the person's choice stays
/// for when it goes off.
#[test]
fn column_dividers_off_hides_them_in_the_main_window_unless_contrast_is_increased() {
    let mut store = store();
    let theme_width = gpui_kit::px(store.tokens.metrics.divider_width);
    assert_eq!(store.ui.divider_width, theme_width, "shown: the theme's width");
    assert!(theme_width > gpui_kit::px(0.0));

    update(&mut store, |s| s.column_dividers = false);
    assert_eq!(store.ui.divider_width, gpui_kit::px(0.0));
    assert_eq!(store.ui.divider, crate::theme_bridge::token(&store.tokens, "divider"), "the color stays");
    assert_eq!(store.ui_for(false).divider_width, theme_width, "the Studio and Settings show the theme's");

    system(&mut store, CONTRAST);
    assert_eq!(store.ui.divider_width, gpui_kit::px(store.tokens.metrics.divider_width));
    assert!(store.ui.divider_width > gpui_kit::px(0.0), "Increase Contrast keeps the lines");

    system(&mut store, SystemLook::default());
    assert_eq!(store.ui.divider_width, gpui_kit::px(0.0), "off again with the setting");
    update(&mut store, |s| s.column_dividers = true);
    assert_eq!(store.ui.divider_width, theme_width);
}

/// Animate icons off holds every window's icons still, Settings and the Studio as well as the main
/// window, and on again lets them all move.
#[test]
fn animate_icons_off_holds_the_icons_of_every_window_still() {
    let mut store = store();
    assert!(store.ui_for(true).icon_motion && store.ui_for(false).icon_motion);
    update(&mut store, |s| s.icon_animations = false);
    assert!(!store.ui_for(true).icon_motion, "the main window");
    assert!(!store.ui_for(false).icon_motion, "Settings, the Studio and About");
    update(&mut store, |s| s.icon_animations = true);
    assert!(store.ui_for(true).icon_motion && store.ui_for(false).icon_motion);
}

/// Increase Contrast: the theme's high-contrast variant and opaque windows, the
/// user's opacity kept for when it goes off; the variant is derived once while nothing
/// else changes.
#[test]
fn increase_contrast_shows_the_high_contrast_variant_on_opaque_windows() {
    let mut store = store();
    update(&mut store, |s| s.opacity = 0.8);
    let plain = store.tokens.clone();
    let theme = store.revisions.theme;
    reset_tally();

    system(&mut store, CONTRAST);
    assert_eq!(*store.tokens, plain.high_contrast());
    assert_eq!(store.revisions.theme, theme + 1);
    assert_eq!(store.settings.opacity, 1.0);
    assert_eq!(store.asked_opacity(), 0.8);
    assert_eq!(store.ui.panel.a, 1.0);
    assert_eq!(store.ui.focus_ring_width, gpui_kit::px(store.tokens.metrics.focus_ring_width));
    assert!(store.tokens.metrics.focus_ring_width >= 3.0, "the ring is 3 under Increase Contrast");
    // A setting outside the theme derives nothing again.
    let high = store.tokens.clone();
    update(&mut store, |s| s.diff_wrap = !s.diff_wrap);
    assert!(Arc::ptr_eq(&high, &store.tokens));
    assert_eq!(tally().get("high_contrast"), Some(&1));

    system(&mut store, SystemLook::default());
    assert_eq!(*store.tokens, *plain);
    assert_eq!(store.settings.opacity, 0.8);
}

/// Match system: the light pick in a light appearance, the dark pick in a dark one; Dark
/// and Light hold their pick whatever the system says.
#[test]
fn match_system_follows_the_system_appearance() {
    let mut store = store();
    update(&mut store, |s| {
        s.theme = "Tokyo Night".into();
        s.theme_light = "GitHub Light".into();
        s.theme_mode = ThemeMode::System;
    });
    assert_eq!(store.tokens.name, "Tokyo Night");
    let theme = store.revisions.theme;
    system(&mut store, LIGHT);
    assert_eq!((store.tokens.name.as_str(), store.config.shown_theme()), ("GitHub Light", "GitHub Light"));
    assert_eq!(store.tokens.appearance, Appearance::Light);
    assert_eq!(store.revisions.theme, theme + 1);
    update(&mut store, |s| s.theme_mode = ThemeMode::Dark);
    assert_eq!(store.tokens.name, "Tokyo Night");
    system(&mut store, SystemLook::default());
    update(&mut store, |s| s.theme_mode = ThemeMode::Light);
    assert_eq!(store.tokens.name, "GitHub Light");
}

/// The window look (opacity, blur, Reduce Transparency, Increase Contrast) and the
/// appearance never reach the cached derivations: after any sequence of flips the store
/// holds what a fresh one, told the same, derives from scratch.
#[test]
fn flipping_the_look_gives_what_a_fresh_derivation_does() {
    enum Flip {
        Opacity(f32),
        Blur(bool),
        System(SystemLook),
    }
    let mut store = store();
    update(&mut store, |s| s.theme_light = "Catppuccin Latte".into());
    let steps = [
        Flip::Opacity(0.6),
        Flip::Blur(false),
        Flip::System(REDUCED),
        Flip::System(SystemLook { reduce_transparency: true, ..LIGHT }),
        Flip::System(CONTRAST),
        Flip::System(SystemLook { increase_contrast: true, ..LIGHT }),
        Flip::Opacity(0.7),
        Flip::System(LIGHT),
        Flip::Blur(true),
        Flip::System(CONTRAST),
        Flip::System(SystemLook::default()),
    ];
    for (n, step) in steps.into_iter().enumerate() {
        match step {
            Flip::Opacity(opacity) => update(&mut store, |s| s.opacity = opacity),
            Flip::Blur(blur) => update(&mut store, |s| s.blur = blur),
            Flip::System(look) => system(&mut store, look),
        }
        let what = format!("step {n}");
        let mut fresh = SettingsStore::new(Config::placeholder());
        update(&mut fresh, |s| *s = store.user_settings().clone());
        system(&mut fresh, store.system());
        assert_eq!(*fresh.tokens, *store.tokens, "after {what}");
        assert_eq!(fresh.ui, store.ui, "after {what}");
        let look = Translucency { opacity: store.settings.opacity, blur: store.settings.blur };
        assert_eq!(UiColors::new(&fresh.tokens, look), store.ui, "after {what}");
        // An opaque window's colours follow the same flips.
        assert_eq!(fresh.ui_for(false), store.ui_for(false), "after {what}");
        assert_eq!(UiColors::new(&fresh.tokens, OPAQUE_LOOK), store.ui_for(false), "after {what}");
    }
}

/// A translucent window draws its text lifted for what shows through, an opaque
/// one (the settings, the Studio, About, Licenses) the theme's own. Where
/// the system makes every window opaque, the two are one.
#[test]
fn each_window_draws_with_the_colours_of_its_own_look() {
    let mut store = store();
    update(&mut store, |s| (s.theme, s.opacity) = ("Tokyo Night".into(), 0.8));
    let token = |name: &str| crate::theme_bridge::token(&store.tokens, name);
    let (translucent, opaque) = (store.ui_for(true), store.ui_for(false));
    assert_eq!(translucent, store.ui, "the main window's");
    assert_ne!(translucent.muted, token("text.muted"), "lifted at 80 %");
    assert_eq!(opaque.muted, token("text.muted"));
    assert_eq!(
        (opaque.text, opaque.faint, opaque.branch),
        (token("text.primary"), token("text.faint"), token("git.branch"))
    );
    assert!(translucent.panel.a < 1.0);
    assert_eq!(opaque.panel, token("sidebar.background"), "nothing shows through");

    for look in [REDUCED, CONTRAST] {
        system(&mut store, look);
        assert_eq!(store.ui_for(true), store.ui_for(false), "{look:?}: every window is opaque");
        assert_eq!(store.ui_for(false).muted, crate::theme_bridge::token(&store.tokens, "text.muted"));
    }
    system(&mut store, SystemLook::default());
    assert_eq!(store.ui_for(false), opaque);
    update(&mut store, |s| s.opacity = 1.0);
    assert_eq!(store.ui_for(true), store.ui_for(false), "an opaque main window");
}

#[test]
fn a_new_theme_derives_the_palette_and_syntax_theme_once() {
    let mut store = store();
    let palette = store.palette.clone();
    reset_tally();
    let other = wormhole_core::theme::tokens::builtin_tokens()
        .into_iter()
        .find(|t| t.name != store.tokens.name)
        .expect("more than one built-in theme");
    update(&mut store, |s| s.theme = other.name.clone());
    assert_eq!(store.tokens.name, other.name);
    assert!(!Rc::ptr_eq(&palette, &store.palette));
    assert_eq!(tally().get("theme_parts"), Some(&1));
}

#[test]
fn a_preview_of_the_same_tokens_keeps_the_derived_theme() {
    let mut store = store();
    let palette = store.palette.clone();
    let theme = store.revisions.theme;
    store.preview = Some(Arc::new(store.tokens.as_ref().clone()));
    reset_tally();
    store.derive();
    assert_eq!(tally().get("theme_parts"), None);
    assert!(Rc::ptr_eq(&palette, &store.palette));
    assert_eq!(store.revisions.theme, theme);

    // A different draft is a different theme; dropping the preview goes back.
    let mut draft = store.tokens.as_ref().clone();
    assert!(draft.set("primary", wormhole_core::theme::tokens::Color::rgba(1.0, 0.0, 0.0, 1.0)));
    store.preview = Some(Arc::new(draft));
    store.derive();
    assert_eq!(store.revisions.theme, theme + 1);
    store.preview = None;
    store.derive();
    assert_eq!(store.revisions.theme, theme + 2);
}

#[test]
fn project_overrides_layer_over_the_settings_and_back() {
    let mut store = store();
    let diff_wrap = store.settings.diff_wrap;
    let overrides = serde_json::json!({ "diff_wrap": !diff_wrap });
    store.project_overrides = overrides.as_object().cloned();
    let changed = store.derive();
    assert!(changed.settings && !changed.theme);
    assert_eq!(store.settings.diff_wrap, !diff_wrap);
    // The user's own value is untouched underneath.
    assert_eq!(store.config.settings.diff_wrap, diff_wrap);
    assert_eq!(store.project_keys(), ["diff_wrap"], "what the settings window marks as the project's");
    store.project_overrides = None;
    store.derive();
    assert_eq!(store.settings.diff_wrap, diff_wrap);
    assert!(store.project_keys().is_empty());
}

#[test]
fn a_config_problem_is_shown_and_cleared() {
    let mut store = store();
    store.config.error = Some("config.json: broken".into());
    let changed = store.derive();
    assert!(changed.error);
    assert_eq!(store.error.as_deref(), Some("config.json: broken"));
    store.config.error = None;
    store.derive();
    assert_eq!(store.error, None);
}
