//! The theme menu: the name in the title row opens it as a native menu. Your themes (the one being
//! edited checked), the project's theme, then New Theme, New Theme From (your themes, then the
//! built-ins, Wormhole's own together), Duplicate, Rename and Revert to Saved.

use gpui_kit::WeakEntity;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_text::keys::{
    STUDIO_MENU_ADD_DARK, STUDIO_MENU_ADD_LIGHT, STUDIO_MENU_NEW, STUDIO_MENU_NEW_FROM,
    STUDIO_MENU_PROJECT_THEME, STUDIO_MENU_RENAME, STUDIO_MENU_REVERT, STUDIO_MENU_REVERT_TO,
    STUDIO_MENU_THIS_PROJECT, STUDIO_MENU_VERSIONS, STUDIO_MENU_WORMHOLE, STUDIO_MENU_YOUR_THEMES,
    studio_menu_duplicate, studio_menu_update_current, studio_menu_update_missing, studio_update_from,
};
use wormhole_text::text;

use super::StudioWindow;
use super::document::Subject;
use super::opening::Target;
use super::theme_list::{Choice, Family, ThemeList};
use super::update_from::State;
use crate::menubar::model::KeyEquivalent;
use crate::platform::menu::{Item, Menu, run_on};
use wormhole_core::theme::tokens::Appearance;

/// What the menu shows about the theme being edited.
pub(super) struct Facts<'a> {
    pub(super) subject: &'a Subject,
    pub(super) name: &'a str,
    pub(super) dirty: bool,
    /// The appearance Add Appearance would add: the one a single theme lacks.
    pub(super) addable: Option<Appearance>,
    /// The theme has earlier saves to browse (it is a file of its own).
    pub(super) versions: bool,
    /// The theme it was copied from (as people know it) and how the copy compares, when the
    /// shown appearance was copied from one.
    pub(super) update: Option<(String, State)>,
}

fn open(
    target: Target,
) -> impl Fn(&mut StudioWindow, &mut gpui_kit::Window, &mut gpui_kit::Context<StudioWindow>) {
    move |studio, window, cx| studio.open_target(&target, window, cx)
}

fn choice(choice: &Choice, this: &WeakEntity<StudioWindow>, checked: bool) -> Item {
    Item::new(choice.name.clone()).checked(checked).on_select(run_on(this, open(choice.target.clone())))
}

fn is_wormhole_family(name: &str) -> bool {
    name == "Wormhole" || name.starts_with("Wormhole ") // text: name (a built-in theme's own name)
}

/// A built-in family as one item, or a submenu of its appearances.
fn family_entry(menu: Menu, family: &Family, this: &WeakEntity<StudioWindow>) -> Menu {
    match family.choices.as_slice() {
        [only] => menu.item(choice(only, this, false)),
        many => menu.submenu(
            family.name.clone(),
            many.iter().fold(Menu::new(), |sub, c| sub.item(choice(c, this, false))),
        ),
    }
}

/// New Theme From: the person's themes, then the built-ins (a family once), each a copy to start from.
fn new_from(list: &ThemeList, this: &WeakEntity<StudioWindow>) -> Menu {
    let mut menu = Menu::new();
    for own in &list.own {
        if let Target::Own(path) = &own.target {
            let from = Choice { name: own.name.clone(), target: Target::CopyOf(path.clone()) };
            menu = menu.item(choice(&from, this, false));
        }
    }
    if !list.own.is_empty() {
        menu = menu.separator();
    }
    // Wormhole's own families (the accent ones and Wormhole itself) go under one name.
    let (own_families, others): (Vec<&Family>, Vec<&Family>) =
        list.builtins.iter().partition(|f| is_wormhole_family(&f.name));
    if !own_families.is_empty() {
        let sub = own_families.iter().fold(Menu::new(), |sub, f| family_entry(sub, f, this));
        menu = menu.submenu(text(STUDIO_MENU_WORMHOLE), sub);
    }
    for family in others {
        menu = family_entry(menu, family, this);
    }
    menu
}

pub(super) fn build(list: &ThemeList, facts: &Facts<'_>, this: &WeakEntity<StudioWindow>) -> Menu {
    let editing = |target: &Target| matches!((target, facts.subject), (Target::Own(a), Subject::Own { path }) if a == path);
    let mut menu = Menu::new();
    if !list.own.is_empty() {
        menu = menu.header(text(STUDIO_MENU_YOUR_THEMES));
        for own in &list.own {
            menu = menu.item(choice(own, this, editing(&own.target)));
        }
    }
    if list.project {
        let checked = matches!(facts.subject, Subject::Project { .. });
        menu = menu.header(text(STUDIO_MENU_THIS_PROJECT)).item(
            Item::new(text(STUDIO_MENU_PROJECT_THEME))
                .checked(checked)
                .on_select(run_on(this, open(Target::Project))),
        );
    }
    let duplicate_key = keymap::key(ActionId::DuplicateTheme).and_then(KeyEquivalent::from_spec);
    let menu = menu.separator();
    let add = facts.addable.map(|to| {
        let label = if to == Appearance::Light { STUDIO_MENU_ADD_LIGHT } else { STUDIO_MENU_ADD_DARK };
        Item::new(text(label))
            .on_select(run_on(this, move |studio, window, cx| studio.add_appearance(to, window, cx)))
    });
    let revert_to = Menu::new().item(
        Item::new(text(STUDIO_MENU_VERSIONS))
            .enabled(facts.versions)
            .on_select(run_on(this, |studio, _, cx| studio.open_versions(cx))),
    );
    let mut menu = menu
        .item(Item::new(text(STUDIO_MENU_NEW)).on_select(run_on(this, open(Target::New))))
        .submenu(text(STUDIO_MENU_NEW_FROM), new_from(list, this))
        .item(
            Item::new(studio_menu_duplicate(facts.name))
                .key(duplicate_key)
                .on_select(run_on(this, |studio, window, cx| studio.duplicate(window, cx))),
        )
        .item(
            Item::new(text(STUDIO_MENU_RENAME))
                .on_select(run_on(this, |studio, window, cx| studio.start_rename(window, cx))),
        );
    if let Some(add) = add {
        menu = menu.item(add);
    }
    if let Some((from, state)) = &facts.update {
        let item = match state {
            State::NoBase => None,
            State::Changed(_) => Some(
                Item::new(studio_update_from(from))
                    .on_select(run_on(this, |studio, _, cx| studio.open_update_from(cx))),
            ),
            State::UpToDate => Some(Item::new(studio_menu_update_current(from)).enabled(false)),
            State::SourceMissing => Some(Item::new(studio_menu_update_missing(from)).enabled(false)),
        };
        if let Some(item) = item {
            menu = menu.item(item);
        }
    }
    menu.separator()
        .item(
            Item::new(text(STUDIO_MENU_REVERT))
                .enabled(facts.dirty)
                .on_select(run_on(this, |studio, window, cx| studio.discard_changes(window, cx))),
        )
        .submenu(text(STUDIO_MENU_REVERT_TO), revert_to)
}

#[cfg(test)]
mod tests {
    use gpui_kit::WeakEntity;

    use super::*;

    fn list() -> ThemeList {
        ThemeList {
            own: vec![
                Choice { name: "Dusk".into(), target: Target::Own("/themes/dusk.json".into()) },
                Choice { name: "Mine".into(), target: Target::Own("/themes/mine.json".into()) },
            ],
            project: true,
            builtins: vec![
                Family {
                    name: "Wormhole".into(),
                    choices: vec![
                        Choice { name: "Dark".into(), target: Target::Builtin("Wormhole Dark".into()) },
                        Choice { name: "Light".into(), target: Target::Builtin("Wormhole Light".into()) },
                    ],
                },
                Family {
                    name: "Wormhole Electric Blue".into(),
                    choices: vec![
                        Choice {
                            name: "Dark".into(),
                            target: Target::Builtin("Wormhole Electric Blue Dark".into()),
                        },
                        Choice {
                            name: "Light".into(),
                            target: Target::Builtin("Wormhole Electric Blue Light".into()),
                        },
                    ],
                },
                Family {
                    name: "Nord".into(),
                    choices: vec![Choice { name: "Nord".into(), target: Target::Builtin("Nord".into()) }],
                },
            ],
        }
    }

    #[test]
    fn the_top_level_has_no_built_ins_and_new_theme_from_groups_the_wormhole_families() {
        let subject = Subject::Own { path: "/themes/mine.json".into() };
        let facts = Facts {
            subject: &subject,
            name: "Mine",
            dirty: true,
            addable: Some(Appearance::Light),
            versions: true,
            update: None,
        };
        let menu = build(&list(), &facts, &WeakEntity::new_invalid());
        assert_eq!(
            menu.outline(),
            [
                "# Your Themes",
                "- Dusk",
                "- Mine [x]",
                "# This Project",
                "- The project\u{2019}s theme",
                "---",
                "- New Theme\u{2026}",
                "> New Theme From",
                "  - Dusk",
                "  - Mine",
                "  ---",
                "  > Wormhole",
                "    > Wormhole",
                "      - Dark",
                "      - Light",
                "    > Wormhole Electric Blue",
                "      - Dark",
                "      - Light",
                "  - Nord",
                "- Duplicate \u{201c}Mine\u{201d}",
                "- Rename\u{2026}",
                "- Add Light Appearance",
                "---",
                "- Revert to Saved",
                "> Revert To",
                "  - Browse All Versions\u{2026}",
            ]
        );
    }

    #[test]
    fn revert_is_off_without_edits_and_a_copy_checks_nothing() {
        let subject = Subject::Untitled { from: "Nord".into() };
        let facts = Facts {
            subject: &subject,
            name: "Untitled (from Nord)",
            dirty: false,
            addable: None,
            versions: false,
            update: None,
        };
        let menu = build(&ThemeList { project: false, ..list() }, &facts, &WeakEntity::new_invalid());
        let lines = menu.outline();
        assert!(lines.contains(&"- Revert to Saved [off]".to_string()), "{lines:?}");
        assert!(!lines.iter().any(|l| l.contains("[x]")), "{lines:?}");
        assert!(!lines.contains(&"# This Project".to_string()));
        assert!(lines.contains(&"  - Browse All Versions\u{2026} [off]".to_string()), "{lines:?}");
        assert!(!lines.iter().any(|l| l.contains("Appearance")), "{lines:?}");
    }

    #[test]
    fn update_from_is_in_the_menu_and_off_unless_the_source_changed() {
        let subject = Subject::Own { path: "/themes/mine.json".into() };
        let lines = |state| {
            let facts = Facts {
                subject: &subject,
                name: "Mine",
                dirty: false,
                addable: None,
                versions: true,
                update: Some(("Nord".into(), state)),
            };
            build(&list(), &facts, &WeakEntity::new_invalid()).outline()
        };
        assert!(lines(State::Changed(2)).contains(&"- Update from Nord\u{2026}".to_string()));
        assert!(lines(State::UpToDate).contains(&"- Up to date with Nord [off]".to_string()));
        assert!(lines(State::SourceMissing).contains(&"- Nord is unavailable [off]".to_string()));
        assert!(!lines(State::NoBase).iter().any(|l| l.contains("Update from") || l.contains("Up to date")));
    }
}
