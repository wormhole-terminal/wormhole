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
use std::path::Path;
use std::sync::Arc;

use gpui_kit::{App, FontFeatures, Global, Pixels, SharedString, px};
use wormhole_core::fonts::FontSlot;
use wormhole_core::fonts::bundled;
use wormhole_core::fonts::custom::{self, FontFile, Scan};

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

struct Traits {
    monospace: bool,
    symbolic: bool,
}

/// Classify a family by its default face. GPUI doesn't expose font traits,
/// so this asks CoreText (thread-safe, and doesn't load the font into GPUI's
/// caches). The fixed-pitch flag alone misses some coding fonts, so equal
/// advances for `iMW0` count as monospace too.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn classify(family: &str) -> Option<Traits> {
    use std::ptr::NonNull;

    use objc2_core_foundation::{CFDictionary, CFString, CFType, CGSize};
    use objc2_core_text::{
        CTFont, CTFontDescriptor, CTFontOrientation, CTFontStylisticClass, CTFontSymbolicTraits,
        kCTFontFamilyNameAttribute,
    };

    let name = CFString::from_str(family);
    // SAFETY: `kCTFontFamilyNameAttribute` is an immutable CoreText constant.
    // The dictionary holds CF types, as `CTFontDescriptorCreateWithAttributes`
    // expects; the font keeps its own references. The glyph and advance
    // buffers have exactly `count` elements and outlive the calls.
    unsafe {
        let key: &CFString = kCTFontFamilyNameAttribute;
        let value: &CFType = name.as_ref();
        let attributes = CFDictionary::<CFString, CFType>::from_slices(&[key], &[value]);
        let descriptor = CTFontDescriptor::with_attributes(attributes.as_opaque());
        let font = CTFont::with_font_descriptor(&descriptor, 12.0, std::ptr::null());
        // CoreText substitutes a default font for a family it can't find.
        if font.family_name().to_string() != family {
            return None;
        }
        let traits = font.symbolic_traits();
        let class = traits.0 & CTFontSymbolicTraits::ClassMaskTrait.0;
        let symbolic = class == CTFontStylisticClass::SymbolicClass.0
            || class == CTFontStylisticClass::OrnamentalsClass.0;

        let mut chars: [u16; 4] = b"iMW0".map(u16::from);
        let mut glyphs = [0u16; 4];
        let mapped = font.glyphs_for_characters(
            NonNull::from(&mut chars).cast(),
            NonNull::from(&mut glyphs).cast(),
            4,
        );
        let mut advances = [CGSize { width: 0.0, height: 0.0 }; 4];
        let equal_advances = mapped && {
            font.advances_for_glyphs(
                CTFontOrientation::Horizontal,
                NonNull::from(&mut glyphs).cast(),
                advances.as_mut_ptr(),
                4,
            );
            advances[0].width > 0.0 && advances.iter().all(|a| (a.width - advances[0].width).abs() < 0.01)
        };
        Some(Traits {
            monospace: traits.contains(CTFontSymbolicTraits::TraitMonoSpace) || equal_advances,
            symbolic: symbolic || !mapped,
        })
    }
}

#[cfg(not(target_os = "macos"))]
fn classify(_family: &str) -> Option<Traits> {
    None
}

/// Make a font file's faces available to this process (or stop that), like
/// an installed font but without touching the system. User fonts go through
/// CoreText rather than GPUI's `add_fonts`, which reads only a collection's
/// first face and can't let go of a font whose file was removed or replaced.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn register(path: &Path, register: bool) -> anyhow::Result<()> {
    use std::ptr::NonNull;

    use objc2_core_foundation::{CFError, CFRetained, CFURL};
    use objc2_core_text::{
        CTFontManagerError, CTFontManagerRegisterFontsForURL, CTFontManagerScope,
        CTFontManagerUnregisterFontsForURL,
    };

    let url = CFURL::from_file_path(path).ok_or_else(|| anyhow::anyhow!("not a file path"))?;
    let mut error: *mut CFError = std::ptr::null_mut();
    // SAFETY: `url` is a valid CFURL for the duration of the call, and `error`
    // points to a local null pointer that CoreText only sets (to an owned,
    // +1 CFError) when the call fails; it's adopted exactly once below.
    let (ok, error) = unsafe {
        let ok = if register {
            CTFontManagerRegisterFontsForURL(&url, CTFontManagerScope::Process, &mut error)
        } else {
            CTFontManagerUnregisterFontsForURL(&url, CTFontManagerScope::Process, &mut error)
        };
        (ok, NonNull::new(error).map(|error| CFRetained::from_raw(error)))
    };
    if ok {
        return Ok(());
    }
    let Some(error) = error else { anyhow::bail!("CoreText refused the file") };
    let code = error.code();
    // Loading a file twice, or dropping one that wasn't loaded, changes nothing.
    if code == CTFontManagerError::AlreadyRegistered.0 || code == CTFontManagerError::NotRegistered.0 {
        return Ok(());
    }
    let description = error.description().map(|d| d.to_string()).unwrap_or_default();
    anyhow::bail!("CoreText error {code}: {description}")
}

#[cfg(not(target_os = "macos"))]
fn register(_path: &Path, _register: bool) -> anyhow::Result<()> {
    anyhow::bail!("custom fonts need macOS")
}
