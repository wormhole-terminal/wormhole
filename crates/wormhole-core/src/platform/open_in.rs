//! The apps a terminal's folder can be opened in from the title row's Open in… button:
//! the editors Wormhole knows by bundle identifier, which of them are
//! installed (the app asks the system, this only holds the rule) and which one a button click
//! opens.
//!
//! Opening a folder in an editor is the person's own action in the app they chose, as when they
//! drag the folder onto it. Wormhole runs nothing from the project and a project can't choose the
//! app (`open_in` is an app-wide setting), so none of this is gated by project trust.

/// An app that can open a folder: Finder, an editor or an IDE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Editor {
    /// The bundle identifier: how the app is found and launched (`open -b`), and what `open_in`
    /// stores, so renaming or moving the app doesn't lose the choice.
    pub bundle: &'static str,
    pub name: &'static str,
    /// Two letters for the title row's button when the app's own icon can't be read.
    pub monogram: &'static str,
}

const fn editor(bundle: &'static str, name: &'static str, monogram: &'static str) -> Editor {
    Editor { bundle, name, monogram }
}

/// Finder: always there, and where a click ends up when nothing else is installed.
pub const FINDER: Editor = editor("com.apple.finder", "Finder", "Fi");

/// The apps offered, in the order a click picks the first one found when nothing is chosen. Finder
/// is not in the list; it comes last in every menu.
pub const EDITORS: &[Editor] = &[
    editor("dev.zed.Zed", "Zed", "Z"),
    editor("dev.zed.Zed-Preview", "Zed Preview", "ZP"),
    editor("com.microsoft.VSCode", "Visual Studio Code", "VS"),
    editor("com.microsoft.VSCodeInsiders", "VS Code Insiders", "VI"),
    editor("com.todesktop.230313mzl4w4u92", "Cursor", "Cu"),
    editor("com.exafunction.windsurf", "Windsurf", "Ws"),
    editor("com.vscodium", "VSCodium", "VC"),
    editor("com.sublimetext.4", "Sublime Text", "St"),
    editor("com.sublimetext.3", "Sublime Text 3", "St"),
    editor("com.apple.dt.Xcode", "Xcode", "Xc"),
    editor("com.jetbrains.intellij", "IntelliJ IDEA", "IJ"),
    editor("com.jetbrains.intellij.ce", "IntelliJ IDEA Community", "IC"),
    editor("com.jetbrains.PhpStorm", "PhpStorm", "Ph"),
    editor("com.jetbrains.WebStorm", "WebStorm", "WS"),
    editor("com.jetbrains.pycharm", "PyCharm", "PC"),
    editor("com.jetbrains.pycharm.ce", "PyCharm Community", "PC"),
    editor("com.jetbrains.goland", "GoLand", "GL"),
    editor("com.jetbrains.rubymine", "RubyMine", "RM"),
    editor("com.jetbrains.CLion", "CLion", "CL"),
    editor("com.jetbrains.rider", "Rider", "Ri"),
    editor("com.jetbrains.datagrip", "DataGrip", "DG"),
    editor("com.jetbrains.rustrover", "RustRover", "RR"),
    editor("com.google.android.studio", "Android Studio", "AS"),
    editor("com.panic.Nova", "Nova", "No"),
    editor("com.barebones.bbedit", "BBEdit", "BB"),
    editor("com.macromates.TextMate", "TextMate", "TM"),
];

/// An app found on this Mac, by the bundle identifier the system knows it by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub bundle: String,
    pub name: String,
    /// What its monogram says when its icon can't be read.
    pub monogram: String,
}

impl From<Editor> for Installed {
    fn from(editor: Editor) -> Self {
        Self { bundle: editor.bundle.into(), name: editor.name.into(), monogram: editor.monogram.into() }
    }
}

/// The known app with this bundle identifier.
pub fn known(bundle: &str) -> Option<Editor> {
    std::iter::once(FINDER).chain(EDITORS.iter().copied()).find(|editor| editor.bundle == bundle)
}

/// What a click on the button opens, and what the setting would say about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub app: Installed,
    /// The setting names an app that isn't installed (any more), so `app` stands in for it: the
    /// button's tooltip says so.
    pub missing: Option<String>,
}

/// The app a click opens: the one `chosen` names (the `open_in` setting) when it is among
/// `installed`; else, with nothing chosen, the first of them that isn't Finder; else Finder. A
/// chosen app that is gone falls back the same way and is reported in `missing`.
///
/// `installed` is in the order the menus list them (Finder last).
pub fn resolve(chosen: &str, installed: &[Installed]) -> Resolved {
    let chosen = chosen.trim();
    if let Some(app) = installed.iter().find(|app| !chosen.is_empty() && app.bundle == chosen) {
        return Resolved { app: app.clone(), missing: None };
    }
    let app =
        installed.iter().find(|app| app.bundle != FINDER.bundle).cloned().unwrap_or_else(|| FINDER.into());
    let missing =
        (!chosen.is_empty()).then(|| known(chosen).map_or(chosen, |editor| editor.name).to_string());
    Resolved { app, missing }
}

/// The apps to list, from the bundle identifiers `found` on this Mac (in any order) plus the
/// `chosen` one when it is an app the table doesn't know but that is installed (picked with
/// "Choose another app…", named `chosen_name`): the known ones in the table's order, then that
/// one, then Finder.
pub fn listing(found: &[String], chosen: Option<Installed>) -> Vec<Installed> {
    let mut apps: Vec<Installed> = EDITORS
        .iter()
        .filter(|editor| found.iter().any(|bundle| bundle == editor.bundle))
        .map(|editor| (*editor).into())
        .collect();
    if let Some(chosen) = chosen
        && known(&chosen.bundle).is_none()
        && !apps.iter().any(|app| app.bundle == chosen.bundle)
    {
        apps.push(chosen);
    }
    apps.push(FINDER.into());
    apps
}

/// Two letters for an app the table doesn't know: the initials of its name's first two words, or
/// its first two letters.
pub fn monogram(name: &str) -> String {
    let mut words = name.split_whitespace();
    let (Some(first), second) = (words.next(), words.next()) else { return String::new() };
    match second {
        Some(second) => {
            first.chars().take(1).chain(second.chars().take(1)).flat_map(char::to_uppercase).collect()
        }
        None => {
            let mut letters = first.chars();
            let head = letters.next().into_iter().flat_map(char::to_uppercase);
            head.chain(letters.next().into_iter().flat_map(char::to_lowercase)).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{EDITORS, FINDER, Installed, known, listing, monogram, resolve};

    fn app(bundle: &str) -> Installed {
        known(bundle).map(Installed::from).unwrap_or_else(|| panic!("{bundle} is not in the table"))
    }

    #[test]
    fn the_table_has_each_app_once() {
        let mut bundles = HashSet::new();
        for editor in EDITORS {
            assert!(bundles.insert(editor.bundle), "{} twice", editor.bundle);
            assert!(!editor.name.is_empty() && (1..=2).contains(&editor.monogram.chars().count()));
        }
        assert!(!bundles.contains(FINDER.bundle), "Finder is added last, not listed");
    }

    #[test]
    fn the_chosen_app_opens_when_it_is_installed() {
        let installed = listing(&["dev.zed.Zed".into(), "com.jetbrains.PhpStorm".into()], None);
        let resolved = resolve("com.jetbrains.PhpStorm", &installed);
        assert_eq!((resolved.app, resolved.missing), (app("com.jetbrains.PhpStorm"), None));
    }

    #[test]
    fn nothing_chosen_opens_the_first_editor_found_else_finder() {
        let installed = listing(
            &["com.microsoft.VSCode".into(), "dev.zed.Zed".into(), "com.apple.dt.Xcode".into()],
            None,
        );
        assert_eq!(resolve("", &installed).app, app("dev.zed.Zed"), "in the table's order, not the system's");
        assert_eq!(resolve("  ", &installed).missing, None, "nothing is not a missing app");
        let none = listing(&[], None);
        assert_eq!((resolve("", &none).app, resolve("", &none).missing), (FINDER.into(), None));
    }

    #[test]
    fn an_app_that_is_gone_falls_back_and_says_which() {
        let installed = listing(&["com.microsoft.VSCode".into()], None);
        let resolved = resolve("dev.zed.Zed", &installed);
        assert_eq!((resolved.app, resolved.missing.as_deref()), (app("com.microsoft.VSCode"), Some("Zed")));
        let resolved = resolve("com.example.Unknown", &listing(&[], None));
        assert_eq!((resolved.app, resolved.missing.as_deref()), (FINDER.into(), Some("com.example.Unknown")));
    }

    #[test]
    fn finder_can_be_chosen() {
        let installed = listing(&["dev.zed.Zed".into()], None);
        assert_eq!(resolve(FINDER.bundle, &installed).app, FINDER.into());
    }

    #[test]
    fn the_listing_is_the_tables_order_then_a_chosen_stranger_then_finder() {
        let stranger =
            Installed { bundle: "com.example.Ed".into(), name: "Ed".into(), monogram: "Ed".into() };
        let found: Vec<String> =
            ["com.microsoft.VSCode", "dev.zed.Zed", "com.not.in.table"].map(String::from).into();
        let bundles: Vec<String> =
            listing(&found, Some(stranger.clone())).into_iter().map(|app| app.bundle).collect();
        assert_eq!(bundles, ["dev.zed.Zed", "com.microsoft.VSCode", "com.example.Ed", FINDER.bundle]);
        // Without being chosen, an app the table doesn't know isn't offered.
        let bundles: Vec<String> = listing(&found, None).into_iter().map(|app| app.bundle).collect();
        assert_eq!(bundles, ["dev.zed.Zed", "com.microsoft.VSCode", FINDER.bundle]);
        // A chosen app the table knows is listed once, in its place.
        let zed = Some(app("dev.zed.Zed"));
        assert_eq!(listing(&found, zed).iter().filter(|app| app.bundle == "dev.zed.Zed").count(), 1);
        // And a chosen stranger counts as installed for the button.
        let installed = listing(&found, Some(stranger.clone()));
        assert_eq!(resolve("com.example.Ed", &installed).app, stranger);
    }

    #[test]
    fn a_monogram_is_the_initials_or_the_first_letters() {
        assert_eq!(monogram("Visual Studio Code"), "VS");
        assert_eq!(monogram("Nova"), "No");
        assert_eq!(monogram("x"), "X");
        assert_eq!(monogram(""), "");
        assert_eq!(monogram("sublime text"), "ST");
    }
}
