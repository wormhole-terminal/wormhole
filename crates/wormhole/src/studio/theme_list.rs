//! The themes the theme menu offers: the person's own, the project's, and the built-ins with a
//! family's appearances together as one name. Pure, built from the catalog.

use wormhole_core::theme::catalog::{ThemeEntry, cards};
use wormhole_core::theme::registry::ThemeId;

use super::opening::Target;

/// One choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Choice {
    pub(super) name: String,
    pub(super) target: Target,
}

/// A built-in theme, or a family of them (a light and a dark one): a family's choices are named by
/// their appearance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Family {
    pub(super) name: String,
    pub(super) choices: Vec<Choice>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ThemeList {
    pub(super) own: Vec<Choice>,
    /// The project's theme, when the front terminal's project has one that may be edited.
    pub(super) project: bool,
    pub(super) builtins: Vec<Family>,
}

/// The list for a catalog. A file that ships several versions is left out until the Studio can
/// edit those.
pub(super) fn theme_list(catalog: &[ThemeEntry], project: bool) -> ThemeList {
    let mut list = ThemeList { project, ..ThemeList::default() };
    for entry in catalog.iter().filter(|e| !e.builtin) {
        if let ThemeId::File(path) = &entry.id {
            list.own.push(Choice { name: entry.label.clone(), target: Target::Own(path.clone()) });
        }
    }
    list.own.sort_by_key(|c| c.name.to_lowercase());
    for card in cards(catalog) {
        let entries: Vec<&ThemeEntry> = card.iter().map(|&ix| &catalog[ix]).filter(|e| e.builtin).collect();
        let Some(first) = entries.first() else { continue };
        let choices = entries
            .iter()
            .map(|e| Choice {
                name: e.family.as_ref().map_or_else(|| e.label.clone(), |(_, version)| version.clone()),
                target: Target::Builtin(e.label.clone()),
            })
            .collect();
        list.builtins.push(Family { name: first.card_label().to_string(), choices });
    }
    list
}

#[cfg(test)]
mod tests {
    use wormhole_core::theme::catalog::catalog_in;

    use super::*;

    #[test]
    fn built_ins_are_listed_once_with_a_family_together() {
        let dir = std::env::temp_dir().join(format!("wormhole-theme-list-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("dusk.json"), r#"{"name": "Dusk"}"#).expect("write");
        std::fs::write(
            dir.join("pair.json"),
            r#"{"name": "Pair", "variants": [{"name": "Light"}, {"name": "Dark"}]}"#,
        )
        .expect("write");
        let list = theme_list(&catalog_in(&dir), true);
        std::fs::remove_dir_all(&dir).expect("clean up");
        assert_eq!(list.own.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Dusk"]);
        assert!(list.project);
        let wormhole = list.builtins.iter().find(|f| f.name == "Wormhole").expect("a Wormhole family");
        assert_eq!(wormhole.choices.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Dark", "Light"]);
        assert_eq!(wormhole.choices[0].target, Target::Builtin("Wormhole Dark".into()));
        let nord = list.builtins.iter().find(|f| f.name == "Nord").expect("Nord");
        assert_eq!(nord.choices.len(), 1);
        assert_eq!(nord.choices[0].name, "Nord");
    }
}
