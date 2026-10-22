//! File icons from the active icon theme (Zed's JSON format),
//! and the asset source that serves the bundled SVGs.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::theme::icon_theme::{self, IconSource, IconTheme, RenderMode};
use wormhole_core::theme::tokens::Appearance;

/// Setting value that turns file icons off.
pub const NONE: &str = "None";
const SIZE: f32 = 14.0;

// GPUI Kit's default bundle is small; these are the extra Lucide icons the app draws itself.
gpui_kit::assets::icon_assets!(
    PanelIcons,
    [
        StickyNote,
        Gauge,
        GitCompare,
        GitCommitHorizontal,
        Funnel,
        X,
        ArrowUpRight,
        Command,
        Sparkles,
        Terminal,
        TextWrap,
        Shield,
        ShieldCheck,
        GitBranch,
        RotateCcw,
        Monitor,
        Keyboard,
        GripVertical,
        FilePlus,
        Pin,
        GitFork,
        // A split's members that aren't together in the list.
        Link2,
        // Settings: the Advanced and Updates nav icons, "Open config.json", the
        // diff layout's segments, "Trust with Touch ID…".
        SlidersHorizontal,
        Download,
        CodeXml,
        Columns2,
        Rows2,
        FingerprintPattern,
        // The command palette: a project's actions.
        Zap,
        // Theme Studio: Export… and the gallery's "uses of" line.
        Upload,
        Target,
        // The first-run sheet's places: Home, Developer, Recents, and the drop zone;
        // a file row's mark for a file moved to another folder.
        House,
        Hammer,
        Clock,
        FolderInput,
        // The find bar's toggles.
        CaseSensitive,
        WholeWord,
        Regex,
        // The title row's pull request, and how its checks stand (passing and failing are
        // GPUI Kit's own circles).
        GitPullRequest,
        GitPullRequestDraft,
        CircleDashed,
        // A diff line under the pointer can be commented on.
        MessageSquarePlus,
        // The terminal list's New group, and the tab bars' group switcher.
        FolderPlus,
        Layers,
        // Settings' Feature flags section (its nav and the Studio's gallery).
        FlaskConical,
        // The palette's profile commands.
        AppWindow,
        ArrowRightLeft,
        Trash,
        // A submodule's row in the Changes and History lists.
        FolderGit2,
    ]
);

/// The status glyphs `style::status_glyph` draws, at the design's own stroke widths (a Lucide
/// icon scaled down to 12 or 14 pt draws thinner lines than the design).
pub mod glyph {
    pub const SPINNER: &str = "wormhole/glyphs/spinner.svg";
    pub const CHECK: &str = "wormhole/glyphs/check.svg";
    pub const BELL: &str = "wormhole/glyphs/bell.svg";
    pub const FAILED: &str = "wormhole/glyphs/failed.svg";
    pub const TERMINAL: &str = "wormhole/glyphs/terminal.svg";
}

const GLYPHS: [(&str, &[u8]); 5] = [
    (glyph::SPINNER, include_bytes!("../assets/glyphs/spinner.svg")),
    (glyph::CHECK, include_bytes!("../assets/glyphs/check.svg")),
    (glyph::BELL, include_bytes!("../assets/glyphs/bell.svg")),
    (glyph::FAILED, include_bytes!("../assets/glyphs/failed.svg")),
    (glyph::TERMINAL, include_bytes!("../assets/glyphs/terminal.svg")),
];

/// GPUI Kit's assets plus the bundled icon theme's SVGs and the agent logos.
pub struct Assets;

/// The app icon as the Dock shows it, light and dark (`packaging/icon/build-icns.sh` renders them from
/// the bundle's icon), for what the app draws itself: the About window and the intro's logo.
const APP_ICON_LIGHT: &str = "wormhole/app-icon.png";
const APP_ICON_DARK: &str = "wormhole/app-icon-dark.png";
const APP_ICON_PNGS: [(&str, &[u8]); 2] = [
    (APP_ICON_LIGHT, include_bytes!("../../../packaging/icon/AppIcon.png")),
    (APP_ICON_DARK, include_bytes!("../../../packaging/icon/AppIcon-dark.png")),
];

/// Path of the app icon for `img()` on a theme of `appearance`.
pub fn app_icon_for(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::Light => APP_ICON_LIGHT,
        Appearance::Dark => APP_ICON_DARK,
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = APP_ICON_PNGS.iter().find(|(icon, _)| *icon == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        if path.starts_with(icon_theme::BUILTIN_PREFIX) {
            return Ok(icon_theme::builtin_asset(path).map(Cow::Borrowed));
        }
        if path.starts_with(wormhole_core::agents::ICON_PREFIX) {
            return Ok(wormhole_core::agents::icon_bytes(path).map(Cow::Borrowed));
        }
        if let Some((_, bytes)) = GLYPHS.iter().find(|(glyph, _)| *glyph == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        if let Some(bytes) = PanelIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit::assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(PanelIcons.list(path)?);
        paths.extend(
            GLYPHS.iter().filter(|(glyph, _)| glyph.starts_with(path)).map(|(glyph, _)| (*glyph).into()),
        );
        Ok(paths)
    }
}

/// Where users put their own icon themes (a folder per theme, or a Zed extension).
pub fn user_dir() -> PathBuf {
    crate::active_profile::paths().icon_themes_dir()
}

/// How long the picker's list of icon themes is kept before the folder is read again.
const LISTING_TTL: Duration = Duration::from_secs(2);

/// Names for the picker: the bundled theme first, then the user's. Finding a user theme's
/// name means reading and parsing its whole file (3 ms for one big pack, more for several),
/// and the settings window asks on every frame it draws, so the list is kept for a couple of
/// seconds: a theme dropped into the folder shows up within that.
pub fn available() -> Vec<String> {
    static LISTING: Mutex<Option<(Instant, Vec<String>)>> = Mutex::new(None);
    let Ok(mut listing) = LISTING.lock() else { return list_available() };
    fresh(&mut listing, Instant::now(), LISTING_TTL, list_available).clone()
}

fn list_available() -> Vec<String> {
    let mut names = vec![icon_theme::builtin_family().name.clone()];
    names.extend(icon_theme::discover(&user_dir()).into_iter().map(|(name, _)| name));
    names.dedup();
    names.push(NONE.into());
    names
}

/// The value in `slot` if it was loaded less than `ttl` before `now`, else a new one.
fn fresh<T>(slot: &mut Option<(Instant, T)>, now: Instant, ttl: Duration, load: impl FnOnce() -> T) -> &T {
    if slot.as_ref().is_some_and(|(at, _)| now.saturating_duration_since(*at) >= ttl) {
        *slot = None;
    }
    &slot.get_or_insert_with(|| (now, load())).1
}

/// Whether `name` needs no disk: "None" and the bundled theme, as opposed to one of the
/// user's (which `resolve` reads from their icon themes folder). Lets the store skip
/// resolving again (and bumping the icons revision) on changes that don't touch the theme.
pub fn is_bundled(name: &str) -> bool {
    name == NONE || name == icon_theme::builtin_family().name
}

/// The theme named `name` in the light or dark variant; falls back to the
/// bundled theme if a user theme is missing or broken.
pub fn resolve(name: &str, light: bool) -> Option<Arc<IconTheme>> {
    if name == NONE {
        return None;
    }
    icon_theme::theme_named(&user_dir(), name, light)
}

/// A file's icon from `theme` (the settings store's `icons`; `None` when they are turned off).
/// Single-color icons are drawn in `tint` (muted text, or the on-selected colors on a selected row).
/// The caller hands both in: settings sits above the icons and holds the theme in force.
pub fn file(theme: Option<&IconTheme>, path: &str, tint: Hsla) -> Option<AnyElement> {
    let theme = theme?;
    theme.file_icon(path).map(|icon| element(icon, theme.render, tint))
}

/// A folder's icon from `theme`, open or closed, drawn as [`file`] draws.
pub fn folder(theme: Option<&IconTheme>, name: &str, expanded: bool, tint: Hsla) -> Option<AnyElement> {
    let theme = theme?;
    theme.directory_icon(name, expanded).map(|icon| element(icon, theme.render, tint))
}

fn element(icon: &IconSource, render: RenderMode, tint: Hsla) -> AnyElement {
    let size = px(SIZE);
    match (icon, render) {
        // `svg()` only reads from the asset source, so mono user icons are drawn in color.
        (IconSource::Builtin(key), RenderMode::Mono) => {
            svg().path(*key).size(size).flex_none().text_color(tint).into_any_element()
        }
        (IconSource::Builtin(key), RenderMode::Color) => img(*key).size(size).flex_none().into_any_element(),
        (IconSource::File(path), _) => img(path.clone()).size(size).flex_none().into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use gpui_kit::AssetSource as _;

    use super::{Assets, GLYPHS, NONE, fresh, glyph, is_bundled};

    #[test]
    fn a_listing_is_read_once_within_its_lifetime_and_again_after_it() {
        let ttl = Duration::from_secs(2);
        let start = Instant::now();
        let mut reads = 0;
        let mut slot = None;
        for step in [0, 100, 1999] {
            let value = *fresh(&mut slot, start + Duration::from_millis(step), ttl, || {
                reads += 1;
                reads
            });
            assert_eq!(value, 1, "still the first listing at {step} ms");
        }
        assert_eq!(reads, 1);
        let value = *fresh(&mut slot, start + Duration::from_millis(2000), ttl, || {
            reads += 1;
            reads
        });
        assert_eq!((value, reads), (2, 2));
    }

    #[test]
    fn only_none_and_the_bundled_theme_need_no_disk() {
        assert!(is_bundled(NONE));
        assert!(is_bundled(&wormhole_core::theme::icon_theme::builtin_family().name));
        assert!(!is_bundled("Some Zed Pack"));
    }

    /// Every icon the app draws by name is served: one that isn't draws nothing, and logs an error
    /// and asks the asset source again on every frame it is on screen.
    #[test]
    fn every_icon_the_app_names_is_served() {
        use gpui_kit::assets::IconName;

        fn sources(dir: &std::path::Path, text: &mut String) {
            for entry in std::fs::read_dir(dir).expect("a source folder") {
                let path = entry.expect("an entry").path();
                if path.is_dir() {
                    sources(&path, text);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    text.push_str(&std::fs::read_to_string(&path).expect("a source file"));
                }
            }
        }
        let mut text = String::new();
        sources(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut text);
        let used: std::collections::BTreeSet<&str> = text
            .split("IconName::")
            .skip(1)
            .map(|rest| &rest[..rest.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(rest.len())])
            .collect();
        let (mut checked, mut missing) = (0, Vec::new());
        for icon in IconName::ALL {
            if !used.contains(format!("{icon:?}").as_str()) {
                continue;
            }
            checked += 1;
            if !matches!(Assets.load(&icon.path()), Ok(Some(_))) {
                missing.push(format!("{icon:?} ({})", icon.path()));
            }
        }
        assert!(missing.is_empty(), "not bundled: {}", missing.join(", "));
        assert!(checked > 40, "only {checked} icons found in the sources");
    }

    #[test]
    fn every_status_glyph_is_served() {
        for path in [glyph::SPINNER, glyph::CHECK, glyph::BELL, glyph::FAILED, glyph::TERMINAL] {
            let bytes = Assets.load(path).expect("loads").expect("is there");
            assert!(bytes.starts_with(b"<svg"), "{path}");
        }
        assert_eq!(Assets.list("wormhole/glyphs/").expect("lists").len(), GLYPHS.len());
    }
}
