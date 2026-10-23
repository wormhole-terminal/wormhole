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

mod derive;
mod overlay;
mod profile;
mod ui_colors;

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::{App, Global, SharedString};
pub use wormhole_core::config::Settings;
use wormhole_core::config::{Config, FeatureMask};
use wormhole_core::theme::fonts::ResolvedFonts;
use wormhole_core::theme::tokens::{Appearance, Color, ThemeTokens};

use crate::palette::Palette;
use crate::theme_bridge;
use derive::{OPAQUE_LOOK, contrasted, main_ui, tally, theme_parts, themed, translucency};
pub use profile::{ParkedSettings, features_in, settings_in, store_in, ui_in};
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
    /// Which features are on ([`SettingsStore::features`]).
    pub features: u64,
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
    /// Which features the settings in force leave on, worked out when they change so that asking
    /// is a bit test (`FeatureMask::resolve`).
    features: FeatureMask,
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
    pub icons: Option<Arc<wormhole_core::theme::icon_theme::IconTheme>>,
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

    pub fn load_for(paths: &wormhole_core::storage::paths::ProfilePaths) -> Self {
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
            fonts: settings.resolved_fonts(&tokens.metrics),
            icons: crate::icons::resolve(&icon_key.0, icon_key.1),
            ui: main_ui(&tokens, &settings, false),
            features: FeatureMask::resolve(&settings),
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
            revisions: Revisions { settings: 1, theme: 1, fonts: 1, icons: 1, features: 1 },
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
        let look_changed = settings.as_ref().is_some_and(|s| {
            translucency(s) != translucency(&self.settings)
                || s.column_dividers != self.settings.column_dividers
        });
        if let Some(settings) = settings {
            self.settings = settings;
            self.revisions.settings += 1;
            changed.settings = true;
            let features = FeatureMask::resolve(&self.settings);
            if features != self.features {
                self.features = features;
                self.revisions.features += 1;
            }
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
            self.ui = main_ui(&self.tokens, &self.settings, self.system.increase_contrast);
        }
        if theme_changed {
            self.ui_opaque = UiColors::new(&self.tokens, OPAQUE_LOOK);
        }

        let fonts = self.settings.resolved_fonts(&self.tokens.metrics);
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
            let store = cx.global::<SettingsStore>();
            let (tokens, fonts, highlight) =
                (store.tokens.clone(), store.fonts.clone(), store.highlight.clone());
            theme_bridge::apply_shared(&tokens, &fonts, &highlight, cx);
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

    /// Which features are on under the settings in force.
    pub fn features(&self) -> FeatureMask {
        self.features
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

pub fn settings(cx: &App) -> &Settings {
    &cx.global::<SettingsStore>().settings
}

/// Which features are on in the shown profile (`SettingsStore::features`): one global read and a
/// bit test, no lookup by name.
pub fn features(cx: &App) -> FeatureMask {
    cx.global::<SettingsStore>().features()
}

/// Switches a feature on or off in the shown profile, as a person's own switch will once Settings has a
/// page for it (there is none yet, so nothing in the app does this). A test of what an off feature hides
/// uses it; a later change to the settings works the mask out again from them.
#[cfg(test)]
pub fn switch_feature(cx: &mut App, id: wormhole_core::config::FeatureId, on: bool) {
    let store = cx.global_mut::<SettingsStore>();
    store.features = store.features.with(id, on);
    store.revisions.features += 1;
    cx.refresh_windows();
}

/// Update settings and notify every view (they re-read the global).
pub fn update_settings(cx: &mut App, f: impl FnOnce(&mut Settings)) {
    SettingsStore::update(cx, f);
    cx.refresh_windows();
}

/// The main window's colours. The settings, Studio and About windows define a `ui` of their own
/// with [`window_ui`], which shadows this one for everything their modules draw (each of their
/// files imports it).
pub fn ui(cx: &App) -> UiColors {
    cx.global::<SettingsStore>().ui
}

/// The colours of a window that is `translucent` (the main window's look) or opaque.
pub fn window_ui(cx: &App, translucent: bool) -> UiColors {
    cx.global::<SettingsStore>().ui_for(translucent)
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "devctl"))]
mod app_tests;
