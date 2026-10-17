//! The settings as a GPUI global: the core [`Config`] plus what the UI derives
//! from it (design tokens, terminal palette, window chrome colors), and the
//! active session's project overrides (`.wormhole/config.json`, `theme.json`).
//!
//! Every change goes through one of the mutators below, which re-derive in place
//! and only what their input touched: a setting that is not part of the theme
//! (wrap lines, a sidebar option) leaves the tokens, the terminal palette and the
//! syntax theme alone, and the choice of primary color is remembered per base
//! theme. [`Revisions`] says what moved, so a view that depends on the theme
//! alone does not rebuild for an unrelated setting. A change to the theme or the
//! fonts also reaches GPUI Kit's theme right away (`theme_bridge`).
//!
//! What the system says counts too ([`SystemLook`]): its appearance picks the theme under
//! Match system, Reduce Transparency makes every window opaque, whatever opacity the settings
//! ask for, and Increase Contrast does too and shows the theme's high-contrast variant.
//!
//! Each window draws with the colours of its own look: the translucent main window with text
//! judged on what shows through it, an opaque one (the settings, the Studio, About, Licenses)
//! with the theme's own. Both sets are kept ([`SettingsStore::ui_for`]).

mod overlay;
mod ui_colors;

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::{App, Global, SharedString};
use wormhole_core::config::Config;
pub use wormhole_core::config::Settings;
use wormhole_core::fonts::ResolvedFonts;
use wormhole_core::tokens::{Appearance, Color, ThemeTokens, Translucency};

use crate::palette::Palette;
use crate::theme_bridge;
pub use ui_colors::UiColors;

/// How many times each derived value has changed since the app started: a view that
/// caches something built from one of them compares its number, not the store's
/// [`SettingsStore::revision`], which moves for every change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Revisions {
    /// Any setting in force: after project overrides and what the system says.
    pub settings: u64,
    /// The resolved tokens, and so the terminal palette and the syntax theme.
    pub theme: u64,
    pub fonts: u64,
    pub icons: u64,
}

/// What one re-derive found different.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Changed {
    settings: bool,
    theme: bool,
    fonts: bool,
    icons: bool,
    error: bool,
}

impl Changed {
    fn any(self) -> bool {
        self.settings || self.theme || self.fonts || self.icons || self.error
    }
}

/// What macOS says about how windows should look, as the app last heard it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemLook {
    /// Light or Dark Mode: which pick Match system shows.
    pub appearance: Appearance,
    /// Accessibility → Display → Reduce Transparency: every window opaque, without blur.
    pub reduce_transparency: bool,
    /// Accessibility → Display → Increase Contrast: the theme's high-contrast variant, every
    /// window opaque.
    pub increase_contrast: bool,
}

impl SystemLook {
    /// Whether the system wants every window opaque, whatever the opacity setting says.
    pub fn opaque(self) -> bool {
        self.reduce_transparency || self.increase_contrast
    }
}

impl Default for SystemLook {
    fn default() -> Self {
        Self { appearance: Appearance::Dark, reduce_transparency: false, increase_contrast: false }
    }
}

/// The primary-colored variant of a base theme, kept until either of them changes.
struct PrimaryMemo {
    base: Arc<ThemeTokens>,
    primary: Color,
    tokens: Arc<ThemeTokens>,
}

/// The high-contrast variant of the tokens in force, kept while they stay the same allocation.
struct ContrastMemo {
    base: Arc<ThemeTokens>,
    tokens: Arc<ThemeTokens>,
}

pub struct SettingsStore {
    config: Config,
    /// The active session's project settings, if that feature is enabled.
    project_overrides: Option<serde_json::Map<String, serde_json::Value>>,
    /// The keys of `project_overrides` that applied (known, and with values that read).
    project_keys: Vec<String>,
    /// The active session's project theme, if that feature is enabled.
    project_theme: Option<Arc<ThemeTokens>>,
    /// Unsaved tokens being edited in the Theme Studio (wins over everything).
    preview: Option<Arc<ThemeTokens>>,
    /// `config.tokens`, shared. Core resolves the theme again on every update and hands
    /// out a new value each time; this keeps the old allocation while the theme is the
    /// same, so what is remembered per base (`primary_memo`) and what compares by
    /// pointer stays valid.
    config_tokens: Arc<ThemeTokens>,
    primary_memo: Option<PrimaryMemo>,
    contrast_memo: Option<ContrastMemo>,
    /// The icon theme and appearance `icons` was resolved for.
    icon_key: (String, bool),
    /// What the system said last: Match system's appearance, Reduce Transparency, Increase
    /// Contrast.
    system: SystemLook,
    /// The opacity the user's and the project's settings ask for, before the system's say.
    asked_opacity: f32,
    /// The settings in force: the user's, the project's on top, and the system's say (an
    /// opacity of 1 under Reduce Transparency or Increase Contrast).
    pub settings: Settings,
    pub tokens: Arc<ThemeTokens>,
    pub palette: Rc<Palette>,
    /// The colours of a translucent window (the main window's), at the opacity in force.
    pub ui: UiColors,
    /// The colours of an opaque window: the theme's own. The same as `ui` while the settings in
    /// force are opaque.
    ui_opaque: UiColors,
    pub highlight: Arc<HighlightTheme>,
    /// Terminal, interface and code fonts after the theme and fallbacks.
    pub fonts: ResolvedFonts,
    /// File icons (`None` when turned off).
    pub icons: Option<Arc<wormhole_core::icon_theme::IconTheme>>,
    /// Bumped on every change, whatever it touched.
    pub revision: u64,
    pub revisions: Revisions,
    /// A problem to show in the settings panel.
    pub error: Option<SharedString>,
}

impl Global for SettingsStore {}

impl SettingsStore {
    /// The `config.json` of the profile Wormhole is in.
    pub fn config_path() -> PathBuf {
        crate::active_profile::paths().config_file()
    }

    pub fn load() -> Self {
        Self::load_for(&crate::active_profile::paths())
    }

    pub fn load_for(paths: &wormhole_core::profile::ProfilePaths) -> Self {
        Self::new(Config::load_for(paths))
    }

    fn new(config: Config) -> Self {
        let settings = config.settings.clone();
        let config_tokens = Arc::new(config.tokens.clone());
        let mut primary_memo = None;
        let tokens = themed(None, None, &config_tokens, settings.primary(), &mut primary_memo);
        let (palette, highlight) = theme_parts(&tokens);
        let icon_key = (settings.icon_theme.clone(), tokens.appearance == Appearance::Light);
        tally("icons");
        Self {
            asked_opacity: settings.opacity,
            fonts: ResolvedFonts::resolve(&settings, &tokens.metrics),
            icons: crate::icons::resolve(&icon_key.0, icon_key.1),
            ui: UiColors::new(&tokens, translucency(&settings)),
            ui_opaque: UiColors::new(&tokens, OPAQUE_LOOK),
            error: config.error.clone().map(Into::into),
            palette,
            highlight,
            tokens,
            settings,
            config,
            config_tokens,
            primary_memo,
            contrast_memo: None,
            icon_key,
            system: SystemLook::default(),
            project_overrides: None,
            project_keys: Vec::new(),
            project_theme: None,
            preview: None,
            revision: 1,
            revisions: Revisions { settings: 1, theme: 1, fonts: 1, icons: 1 },
        }
    }

    /// A store with the default settings and nothing behind it: no file is read or written
    /// (a save fails quietly), so a test never touches the user's data folder. Its tests need
    /// GPUI's test support, which comes with devctl.
    #[cfg(all(test, feature = "devctl"))]
    pub(crate) fn for_test() -> Self {
        Self::new(Config::placeholder())
    }

    /// Re-derive after something changed, in place and only what that touched.
    fn derive(&mut self) -> Changed {
        let mut changed = Changed::default();

        let mut settings = match &self.project_overrides {
            None => {
                self.project_keys.clear();
                self.config.settings.clone()
            }
            Some(overrides) => {
                let (settings, keys) = self.config.settings.with_overrides_reported(overrides);
                self.project_keys = keys;
                settings
            }
        };
        self.asked_opacity = settings.opacity;
        if self.system.opaque() {
            settings.opacity = 1.0;
        }
        let settings = Some(settings).filter(|s| *s != self.settings);
        let look_changed = settings.as_ref().is_some_and(|s| translucency(s) != translucency(&self.settings));
        if let Some(settings) = settings {
            self.settings = settings;
            self.revisions.settings += 1;
            changed.settings = true;
        }

        if *self.config_tokens != self.config.tokens {
            self.config_tokens = Arc::new(self.config.tokens.clone());
        }
        let tokens = themed(
            self.preview.as_ref(),
            self.project_theme.as_ref(),
            &self.config_tokens,
            self.settings.primary(),
            &mut self.primary_memo,
        );
        let tokens = contrasted(tokens, self.system.increase_contrast, &mut self.contrast_memo);
        let mut theme_changed = false;
        if !Arc::ptr_eq(&tokens, &self.tokens) {
            // The same theme under a new allocation (the Studio re-sending its draft)
            // keeps the palette and the syntax theme.
            if *tokens != *self.tokens {
                (self.palette, self.highlight) = theme_parts(&tokens);
                self.revisions.theme += 1;
                theme_changed = true;
                changed.theme = true;
            }
            self.tokens = tokens;
        }
        if theme_changed || look_changed {
            self.ui = UiColors::new(&self.tokens, translucency(&self.settings));
        }
        if theme_changed {
            self.ui_opaque = UiColors::new(&self.tokens, OPAQUE_LOOK);
        }

        let fonts = ResolvedFonts::resolve(&self.settings, &self.tokens.metrics);
        if fonts != self.fonts {
            self.fonts = fonts;
            self.revisions.fonts += 1;
            changed.fonts = true;
        }

        // A user's icon theme is read from disk, so it is read again every time (an edit to
        // its files shows up on the next change, as it always did); the bundled one and
        // "None" only change with the name or the appearance.
        let light = self.tokens.appearance == Appearance::Light;
        let name = &self.settings.icon_theme;
        if !crate::icons::is_bundled(name) || self.icon_key.0 != *name || self.icon_key.1 != light {
            tally("icons");
            self.icons = crate::icons::resolve(name, light);
            self.icon_key = (name.clone(), light);
            self.revisions.icons += 1;
            changed.icons = true;
        }

        let error = self.config.error.as_deref();
        if self.error.as_deref() != error {
            self.error = error.map(|e| SharedString::from(e.to_string()));
            changed.error = true;
        }

        if changed.any() {
            self.revision += 1;
        }
        changed
    }

    /// Hand a change to GPUI Kit's theme now, rather than at the next tick: our own views
    /// read the store directly, so the Kit's controls would otherwise show the old colors
    /// until the tick caught up.
    fn publish(changed: Changed, cx: &mut App) {
        if changed.theme || changed.fonts {
            theme_bridge::apply_store(cx);
        }
    }

    /// Run a mutation on the store, re-derive and publish it.
    fn mutate<R>(cx: &mut App, f: impl FnOnce(&mut SettingsStore) -> R) -> R {
        let store = cx.global_mut::<SettingsStore>();
        let result = f(store);
        let changed = store.derive();
        Self::publish(changed, cx);
        result
    }

    /// Check whether config.json changed on disk. Returns true if reloaded. (Nothing is
    /// re-derived otherwise: this runs every tick, and a re-derive would also reset a
    /// problem a settings page put in `error`, such as a failed import.)
    pub fn poll(cx: &mut App) -> bool {
        if !cx.global_mut::<SettingsStore>().config.poll() {
            return false;
        }
        Self::mutate(cx, |_| ());
        true
    }

    /// Change settings from the UI and persist them.
    pub fn update(cx: &mut App, f: impl FnOnce(&mut Settings)) {
        Self::mutate(cx, |store| store.config.update(f));
    }

    /// Layer (or remove) the active project's settings. A project can change how its sessions
    /// look and behave, not the app as a whole: keys outside `config::PROJECT_SETTINGS`
    /// (the hotkey, the menu bar item, updates, notifications, the settings and Studio
    /// windows) are left out.
    pub fn set_project_overrides(
        cx: &mut App,
        overrides: Option<serde_json::Map<String, serde_json::Value>>,
    ) {
        Self::mutate(cx, |store| store.project_overrides = overrides.map(overlay::project_scoped));
    }

    /// The colours of a window that is translucent (with the settings' opacity, the main
    /// window's look) or opaque.
    pub fn ui_for(&self, translucent: bool) -> UiColors {
        if translucent { self.ui } else { self.ui_opaque }
    }

    /// What the system said last about how windows look.
    pub fn system(&self) -> SystemLook {
        self.system
    }

    /// The system's appearance, Reduce Transparency or Increase Contrast changed (or are known
    /// for the first time). Returns whether anything did.
    pub fn set_system(cx: &mut App, look: SystemLook) -> bool {
        if cx.global::<SettingsStore>().system == look {
            return false;
        }
        Self::mutate(cx, |store| store.take_system(look));
        true
    }

    fn take_system(&mut self, look: SystemLook) {
        self.config.set_appearance(look.appearance);
        self.system = look;
    }

    /// The opacity the settings ask for, which Reduce Transparency and Increase Contrast overrule
    /// while one is on: what the opacity slider shows.
    pub fn asked_opacity(&self) -> f32 {
        self.asked_opacity
    }

    /// The name of the theme shown: the dark or the light pick (`theme_mode`, Match system).
    pub fn shown_theme(&self) -> &str {
        self.config.shown_theme()
    }

    /// The user's own settings, without the project's: what "changed from its default" and
    /// the reset arrows of the settings window look at.
    pub fn user_settings(&self) -> &Settings {
        &self.config.settings
    }

    /// The keys the active project's `.wormhole/config.json` sets (sorted).
    pub fn project_keys(&self) -> &[String] {
        &self.project_keys
    }

    /// Whether config.json failed to read in full, so that changes made in the app are not
    /// saved until it is fixed: the settings page says so.
    pub fn config_unreadable(&self) -> bool {
        self.config.is_unreadable()
    }

    /// Use (or stop using) the active project's theme.
    pub fn set_project_theme(cx: &mut App, tokens: Option<ThemeTokens>) {
        Self::mutate(cx, |store| {
            // The same theme again keeps its allocation, and what is remembered about it.
            if store.project_theme.as_deref() != tokens.as_ref() {
                store.project_theme = tokens.map(Arc::new);
            }
        });
    }

    /// Preview tokens without saving (the Theme Studio's live editing).
    pub fn preview_tokens(cx: &mut App, tokens: Option<ThemeTokens>) {
        Self::mutate(cx, |store| store.preview = tokens.map(Arc::new));
    }

    /// The theme in effect without the Studio preview (project theme or the user's).
    pub fn base_tokens(&self) -> &ThemeTokens {
        self.project_theme.as_deref().unwrap_or(&self.config.tokens)
    }

    /// Whether a project theme is currently shown instead of the user's.
    pub fn has_project_theme(&self) -> bool {
        self.project_theme.is_some()
    }

    /// Copy a theme file into the themes dir and select it.
    pub fn import_theme(cx: &mut App, source: &Path) -> Result<(), String> {
        Self::mutate(cx, |store| store.config.import_theme(source))
    }
}

/// The tokens in effect: the Studio's preview exactly as drafted, otherwise the
/// project's theme or the user's with the chosen primary on top. Re-deriving a theme
/// around a primary is the one costly step, so the last result is reused while the
/// base theme (by identity) and the primary stay the same.
fn themed(
    preview: Option<&Arc<ThemeTokens>>,
    project: Option<&Arc<ThemeTokens>>,
    config: &Arc<ThemeTokens>,
    primary: Option<Color>,
    memo: &mut Option<PrimaryMemo>,
) -> Arc<ThemeTokens> {
    if let Some(preview) = preview {
        return preview.clone();
    }
    let base = project.unwrap_or(config);
    let Some(primary) = primary else {
        return base.clone();
    };
    if let Some(memo) = memo.as_ref()
        && Arc::ptr_eq(&memo.base, base)
        && memo.primary == primary
    {
        return memo.tokens.clone();
    }
    tally("with_primary");
    let tokens = Arc::new(base.with_primary(primary));
    *memo = Some(PrimaryMemo { base: base.clone(), primary, tokens: tokens.clone() });
    tokens
}

/// Under Increase Contrast, the theme's high-contrast variant (core derives it from the theme's
/// own rules, so what the theme sets still wins). It is worked out after the theme, never cached
/// with it: the built-in themes stay keyed by appearance alone. The last one is reused while the
/// tokens it came from are the same allocation, which `themed` keeps them while nothing changes.
fn contrasted(tokens: Arc<ThemeTokens>, high: bool, memo: &mut Option<ContrastMemo>) -> Arc<ThemeTokens> {
    if !high {
        return tokens;
    }
    if let Some(memo) = memo.as_ref()
        && Arc::ptr_eq(&memo.base, &tokens)
    {
        return memo.tokens.clone();
    }
    tally("high_contrast");
    let high = Arc::new(tokens.high_contrast());
    *memo = Some(ContrastMemo { base: tokens, tokens: high.clone() });
    high
}

/// How the windows let the desktop through, by the settings in force.
fn translucency(settings: &Settings) -> Translucency {
    Translucency { opacity: settings.opacity, blur: settings.blur }
}

/// An opaque window's look: nothing shows through, so the tokens stand as they are.
const OPAQUE_LOOK: Translucency = Translucency { opacity: 1.0, blur: false };

/// What follows from a theme's tokens alone: the terminal palette and the syntax theme.
fn theme_parts(tokens: &ThemeTokens) -> (Rc<Palette>, Arc<HighlightTheme>) {
    tally("theme_parts");
    let palette = Palette::from_theme(&tokens.to_theme()).with_tokens(tokens);
    (Rc::new(palette), theme_bridge::highlight_for(tokens))
}

// Counts the derivations a test is about, per thread (tests run in parallel).
#[cfg(test)]
thread_local! {
    static TALLY: std::cell::RefCell<std::collections::BTreeMap<&'static str, usize>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

#[cfg(test)]
fn tally(what: &'static str) {
    TALLY.with(|t| *t.borrow_mut().entry(what).or_default() += 1);
}

#[cfg(not(test))]
#[inline(always)]
fn tally(_: &'static str) {}

pub fn settings(cx: &App) -> &Settings {
    &cx.global::<SettingsStore>().settings
}

/// The main window's colours. The settings, Studio and About windows define a `ui` of their own
/// with [`window_ui`], which shadows this one for everything their modules draw (they reach it
/// through `use super::*`).
pub fn ui(cx: &App) -> UiColors {
    cx.global::<SettingsStore>().ui
}

/// The colours of a window that is `translucent` (the main window's look) or opaque.
pub fn window_ui(cx: &App, translucent: bool) -> UiColors {
    cx.global::<SettingsStore>().ui_for(translucent)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::sync::Arc;

    use wormhole_core::config::{Config, Settings, ThemeMode};
    use wormhole_core::tokens::{Appearance, Translucency};

    use super::{OPAQUE_LOOK, SettingsStore, SystemLook, TALLY, UiColors};

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
        let (tokens, palette, highlight) =
            (store.tokens.clone(), store.palette.clone(), store.highlight.clone());
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
        assert_eq!(tally().get("with_primary"), None);
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
        update(&mut store, |s| s.primary_color = "#ff8800".into());
        assert_eq!(tally().get("with_primary"), Some(&1));
        assert_eq!(tally().get("theme_parts"), Some(&1));
        assert!(store.tokens.get("primary").is_some_and(|c| c.to_hex() == "#ff8800"));
        let theme = store.revisions.theme;
        let tokens = store.tokens.clone();

        // Other settings change while the primary stays: nothing is derived again.
        update(&mut store, |s| s.tab_width = 8);
        update(&mut store, |s| s.opacity = 0.9);
        assert_eq!(tally().get("with_primary"), Some(&1));
        assert_eq!(tally().get("theme_parts"), Some(&1));
        assert!(Arc::ptr_eq(&tokens, &store.tokens));
        assert_eq!(store.revisions.theme, theme);

        // Another primary is another derivation; clearing it goes back to the theme's own.
        update(&mut store, |s| s.primary_color = "#0088ff".into());
        assert_eq!(tally().get("with_primary"), Some(&2));
        update(&mut store, |s| s.primary_color.clear());
        assert_eq!(tally().get("with_primary"), Some(&2));
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
        update(&mut store, |s| s.opacity = 0.8);
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
        assert_eq!((store.tokens.name.as_str(), store.shown_theme()), ("GitHub Light", "GitHub Light"));
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
        update(&mut store, |s| s.opacity = 0.8);
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
        let other = wormhole_core::tokens::builtin_tokens()
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
        assert!(draft.set("primary", wormhole_core::tokens::Color::rgba(1.0, 0.0, 0.0, 1.0)));
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
}

#[cfg(all(test, feature = "devctl"))]
mod app_tests;
