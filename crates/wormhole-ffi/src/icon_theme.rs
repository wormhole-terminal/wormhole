//! The file icon theme in force, for the Files tab and the Changes rows: core's lookup
//! (`theme::icon_theme`, Zed's format, the bundled Wormhole Icons or a family of the profile's `icon-themes`
//! folder, light and dark), asked one name at a time. The answer is where the SVG is: an absolute path of a
//! user theme's file, or the bundled theme's key (`wormhole-icons/dark/rust.svg`) which Swift finds in its
//! resources. Nothing here reads a setting; the caller says which theme (`SettingsHandle::icon_theme_in_force`).

use std::path::Path;
use std::sync::Arc;

use wormhole_core::theme::icon_theme::{IconSource, IconTheme, RenderMode, theme_named};

/// The value `icon_theme` holds to show no file icons (stored, not interface text).
const NO_ICONS: &str = "None";

/// One icon theme, dark and light.
#[derive(uniffi::Object)]
pub struct IconThemeHandle {
    dark: Option<Arc<IconTheme>>,
    light: Option<Arc<IconTheme>>,
}

fn place(source: &IconSource) -> String {
    match source {
        IconSource::Builtin(key) => (*key).to_string(),
        IconSource::File(path) => path.to_string_lossy().into_owned(),
    }
}

#[uniffi::export]
impl IconThemeHandle {
    /// The theme called `name` (a family of `user_dir`, or the bundled one when there is none by that name or it
    /// does not load); no icons at all for "None".
    #[uniffi::constructor]
    pub fn open(user_dir: String, name: String) -> Arc<Self> {
        if name == NO_ICONS {
            return Arc::new(Self { dark: None, light: None });
        }
        let dir = Path::new(&user_dir);
        Arc::new(Self { dark: theme_named(dir, &name, false), light: theme_named(dir, &name, true) })
    }

    /// Whether there is a theme at all (not "None").
    pub fn has_icons(&self) -> bool {
        self.dark.is_some() || self.light.is_some()
    }

    /// Whether the icons keep their own colours, or are masks tinted with the text colour (`x-wormhole.render`).
    pub fn renders_in_colour(&self) -> bool {
        self.dark.as_ref().or(self.light.as_ref()).is_none_or(|t| t.render == RenderMode::Color)
    }

    /// Where the icon of the file `path` (a bare name or a repo-relative path) is.
    pub fn file(&self, path: String, dark: bool) -> Option<String> {
        self.pick(dark)?.file_icon(&path).map(place)
    }

    /// Where the icon of the folder `name`, open or closed, is.
    pub fn folder(&self, name: String, open: bool, dark: bool) -> Option<String> {
        self.pick(dark)?.directory_icon(&name, open).map(place)
    }
}

impl IconThemeHandle {
    fn pick(&self, dark: bool) -> Option<&IconTheme> {
        let (first, other) = if dark { (&self.dark, &self.light) } else { (&self.light, &self.dark) };
        first.as_ref().or(other.as_ref()).map(|t| &**t)
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods, reason = "a test writing its own scratch icon theme folder")]
mod tests {
    use super::IconThemeHandle;

    #[test]
    fn the_bundled_theme_answers_with_keys_and_none_with_nothing() {
        let dir = std::env::temp_dir().join(format!("wh-icons-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let bundled = IconThemeHandle::open(dir.to_string_lossy().into_owned(), "Wormhole Icons".into());
        assert!(bundled.has_icons() && bundled.renders_in_colour());
        let rust = bundled.file("src/main.rs".into(), true).expect("an icon");
        assert!(rust.starts_with("wormhole-icons/dark/"), "{rust}");
        assert!(
            bundled
                .folder("src".into(), false, false)
                .expect("a folder")
                .starts_with("wormhole-icons/light/")
        );
        let none = IconThemeHandle::open(dir.to_string_lossy().into_owned(), "None".into());
        assert!(!none.has_icons() && none.file("a.rs".into(), true).is_none());
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_user_family_answers_with_its_own_files_over_the_bundled_associations() {
        let dir = std::env::temp_dir().join(format!("wh-icons-user-{}", std::process::id()));
        let theme = dir.join("mine");
        std::fs::create_dir_all(theme.join("icons")).expect("dir");
        std::fs::write(theme.join("icons/zz.svg"), "<svg xmlns='http://www.w3.org/2000/svg'/>").expect("svg");
        std::fs::write(
            theme.join("mine.json"),
            r#"{"name": "Mine", "x-wormhole": {"render": "mono"}, "themes": [{"name": "Mine Dark", "appearance": "dark",
               "file_suffixes": {"zz": "zed"}, "file_icons": {"zed": {"path": "./icons/zz.svg"}, "default": {"path": "./icons/zz.svg"}}}]}"#,
        )
        .expect("theme");
        let handle = IconThemeHandle::open(dir.to_string_lossy().into_owned(), "Mine".into());
        assert!(!handle.renders_in_colour());
        let path = handle.file("a.zz".into(), true).expect("icon");
        assert!(path.ends_with("mine/icons/zz.svg"), "{path}");
        assert_eq!(
            handle.file("x.unknown".into(), false).as_deref(),
            Some(path.as_str()),
            "the default, and one variant for both looks"
        );
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }
}
