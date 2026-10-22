//! The fonts the app can draw with: the bundled JetBrains Mono and the user's
//! font files (loaded at startup, the fonts folder watched after that), the
//! installed families for the font pickers and which of them are monospace,
//! and the resolved interface / code fonts views draw with.
//!
//! Listing families costs ~100 ms and classifying them a few hundred more, so
//! both run on the background executor the first time a picker needs them
//! (again after the fonts folder changes); the pickers show what's there so far.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

use gpui_kit::{App, FontFeatures, Global, Pixels, SharedString, px};
use wormhole_core::fonts::FontSlot;
use wormhole_core::fonts::bundled;
use wormhole_core::fonts::custom::{self, FontFile, Scan};
use wormhole_core::profile::ProfileId;

use crate::platform::fonts::{classify, register};
use crate::settings::SettingsStore;

/// The fonts Wormhole loaded itself, as opposed to the installed ones.
#[derive(Default)]
pub struct LoadedFonts {
    /// The bundled families (empty when a custom font took the name).
    pub bundled: Vec<String>,
    /// The user's font files that are registered right now.
    custom: Vec<FontFile>,
    fingerprint: custom::Fingerprint,
}

impl Global for LoadedFonts {}

impl LoadedFonts {
    /// The custom families, sorted.
    pub fn custom_families(&self) -> Vec<String> {
        custom::families(&self.custom).into_iter().map(|f| f.name).collect()
    }

    /// Bundled and custom families that are monospace.
    fn monospace(&self) -> Vec<String> {
        let custom = custom::families(&self.custom).into_iter().filter(|f| f.monospace).map(|f| f.name);
        self.bundled.iter().cloned().chain(custom).collect()
    }
}

/// Load the bundled and custom fonts. Runs before the first window, so the
/// terminal never starts in a fallback font.
pub fn init(cx: &mut App) {
    let dir = crate::active_profile::paths().fonts_dir();
    let fingerprint = custom::fingerprint(&dir);
    let scan = custom::scan(&dir);
    let custom = register_all(scan);

    // GPUI looks fonts it was handed up before installed ones, so this copy
    // wins over an installed JetBrains Mono of another version. A custom font
    // of the same family is the user's deliberate choice: let that one win.
    let overridden = custom.iter().flat_map(|f| &f.families).any(|f| f.name == bundled::FAMILY);
    let mut bundled_families = Vec::new();
    if overridden {
        tracing::info!("fonts: a custom {} replaces the bundled one", bundled::FAMILY);
    } else {
        let faces = bundled::FACES.iter().map(|bytes| Cow::Borrowed(*bytes)).collect();
        match cx.text_system().add_fonts(faces) {
            Ok(()) => bundled_families.push(bundled::FAMILY.to_string()),
            Err(err) => tracing::error!("loading the bundled {}: {err:#}", bundled::FAMILY),
        }
    }
    cx.set_global(LoadedFonts { bundled: bundled_families, custom, fingerprint });
}

/// Pick up fonts added to, changed in or removed from the fonts folder.
/// Returns true if anything changed.
pub fn poll_custom(cx: &mut App) -> bool {
    let fingerprint = custom::fingerprint(&crate::active_profile::paths().fonts_dir());
    let loaded = cx.global_mut::<LoadedFonts>();
    if fingerprint == loaded.fingerprint {
        return false;
    }
    let (removed, added) = custom::changes(&loaded.fingerprint, &fingerprint);
    loaded.fingerprint = fingerprint;
    loaded.custom.retain(|file| {
        let keep = !removed.contains(&file.path);
        if !keep && let Err(err) = register(&file.path, false) {
            tracing::warn!("font {}: {err:#}", file.path.display());
        }
        keep
    });
    let mut scan = Scan::default();
    for path in added {
        match custom::read_font_file(&path) {
            Ok(file) => scan.files.push(file),
            Err(err) => scan.skipped.push((path, format!("{err:#}"))),
        }
    }
    let registered = register_all(scan);
    loaded.custom.extend(registered);
    tracing::info!("fonts: fonts folder changed, {} custom families", loaded.custom_families().len());
    FontCatalog::reload(cx);
    true
}

/// Register the files that parsed (for this process only); warn about the rest.
fn register_all(scan: Scan) -> Vec<FontFile> {
    for (path, reason) in &scan.skipped {
        tracing::warn!("font {} skipped: {reason}", path.display());
    }
    scan.files
        .into_iter()
        .filter(|file| {
            register(&file.path, true)
                .inspect_err(|err| tracing::warn!("font {} skipped: {err:#}", file.path.display()))
                .is_ok()
        })
        .collect()
}

#[derive(Default)]
pub struct FontCatalog {
    started: bool,
    /// Bumped by [`FontCatalog::reload`], so a listing that was already
    /// running doesn't overwrite the newer one.
    generation: u64,
    /// Installed families, sorted; empty until listed.
    pub names: Arc<Vec<String>>,
    /// Fixed-pitch families; `None` until classified.
    pub monospace: Option<Arc<HashSet<String>>>,
    /// Symbol/ornament fonts, whose own face can't spell their name.
    pub symbolic: Arc<HashSet<String>>,
}

impl Global for FontCatalog {}

impl FontCatalog {
    /// Start listing and classifying the installed fonts (once per run).
    pub fn ensure_loaded(cx: &mut App) {
        if cx.try_global::<FontCatalog>().is_some_and(|c| c.started) {
            return;
        }
        let generation = cx.try_global::<FontCatalog>().map_or(0, |c| c.generation);
        cx.set_global(FontCatalog { started: true, generation, ..FontCatalog::default() });
        // CoreText doesn't know the bundled font (GPUI holds it in memory), so
        // its classification comes from the font file instead.
        let known_mono = cx.try_global::<LoadedFonts>().map(LoadedFonts::monospace).unwrap_or_default();
        let text_system = cx.text_system().clone();
        let executor = cx.background_executor().clone();
        let current = move |cx: &App| cx.global::<FontCatalog>().generation == generation;
        cx.spawn(async move |cx| {
            let names = Arc::new(executor.spawn(async move { text_system.all_font_names() }).await);
            cx.update(|cx| {
                if current(cx) {
                    cx.global_mut::<FontCatalog>().names = names.clone();
                    cx.refresh_windows();
                }
            });
            let (mut monospace, symbolic) = executor.spawn(async move { classify_all(&names) }).await;
            monospace.extend(known_mono);
            tracing::debug!("fonts: {} monospace, {} symbolic", monospace.len(), symbolic.len());
            cx.update(|cx| {
                if current(cx) {
                    let catalog = cx.global_mut::<FontCatalog>();
                    catalog.monospace = Some(Arc::new(monospace));
                    catalog.symbolic = Arc::new(symbolic);
                    cx.refresh_windows();
                }
            });
        })
        .detach();
    }

    /// List again after the fonts changed, if a picker listed them before.
    fn reload(cx: &mut App) {
        if cx.try_global::<FontCatalog>().is_some_and(|c| c.started) {
            let catalog = cx.global_mut::<FontCatalog>();
            catalog.started = false;
            catalog.generation += 1;
            Self::ensure_loaded(cx);
        }
    }
}

/// The resolved family and size of one of the fonts, for views to draw with.
pub fn font(slot: FontSlot, cx: &App) -> (SharedString, Pixels) {
    let fonts = &cx.global::<SettingsStore>().fonts;
    (fonts.family(slot).to_string().into(), px(fonts.size(slot)))
}

/// [`font`] for a window in `profile` (an odd window draws with its own profile's fonts).
pub fn font_in(profile: &ProfileId, slot: FontSlot, cx: &App) -> (SharedString, Pixels) {
    let fonts = &crate::settings::store_in(profile, cx).fonts;
    (fonts.family(slot).to_string().into(), px(fonts.size(slot)))
}

/// [`code_font`] for a window in `profile`.
pub fn code_font_in(profile: &ProfileId, cx: &App) -> (SharedString, Pixels) {
    font_in(profile, FontSlot::Code, cx)
}

/// Code text: diffs, file lists, hashes, samples.
pub fn code_font(cx: &App) -> (SharedString, Pixels) {
    font(FontSlot::Code, cx)
}

/// OpenType features for monospace text (terminal and code): ligatures off
/// when the user turned them off. Programming fonts form `=>`, `!=` and the
/// like through contextual alternates (`calt`); `liga` covers fonts that use
/// standard ligatures instead.
pub fn ligature_features(enabled: bool) -> FontFeatures {
    if enabled {
        FontFeatures::default()
    } else {
        FontFeatures(Arc::new(vec![("calt".into(), 0), ("liga".into(), 0)]))
    }
}

/// The features code text (`code_font`) is drawn with, following `font_ligatures`.
pub fn code_font_features(cx: &App) -> FontFeatures {
    ligature_features(cx.global::<SettingsStore>().settings.font_ligatures)
}

/// [`code_font_features`] for a window in `profile`.
pub fn code_font_features_in(profile: &ProfileId, cx: &App) -> FontFeatures {
    ligature_features(crate::settings::settings_in(profile, cx).font_ligatures)
}

fn classify_all(names: &[String]) -> (HashSet<String>, HashSet<String>) {
    let mut monospace = HashSet::new();
    let mut symbolic = HashSet::new();
    for name in names.iter().filter(|n| !n.starts_with('.')) {
        if let Some(traits) = classify(name) {
            if traits.monospace {
                monospace.insert(name.clone());
            }
            if traits.symbolic {
                symbolic.insert(name.clone());
            }
        }
    }
    (monospace, symbolic)
}
