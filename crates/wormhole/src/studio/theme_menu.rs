//! The theme menu: the name in the title row opens it as a native menu. Your themes (the one being
//! edited checked), the project's theme, the built-ins to start a copy from, then New Theme,
//! Duplicate, Rename and Revert to Saved.

use gpui_kit::WeakEntity;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_text::keys::{
    STUDIO_MENU_BUILT_IN, STUDIO_MENU_NEW, STUDIO_MENU_PROJECT_THEME, STUDIO_MENU_RENAME, STUDIO_MENU_REVERT,
    STUDIO_MENU_THIS_PROJECT, STUDIO_MENU_YOUR_THEMES, studio_menu_duplicate,
};
use wormhole_text::text;

use super::StudioWindow;
use super::document::Subject;
use super::opening::Target;
use super::theme_list::{Choice, ThemeList};
use crate::menubar::model::KeyEquivalent;
use crate::platform::menu::{Item, Menu, run_on};

/// What the menu shows about the theme being edited.
pub(super) struct Facts<'a> {
    pub(super) subject: &'a Subject,
    pub(super) name: &'a str,
    pub(super) dirty: bool,
}

fn open(
    target: Target,
) -> impl Fn(&mut StudioWindow, &mut gpui_kit::Window, &mut gpui_kit::Context<StudioWindow>) {
    move |studio, window, cx| studio.open_target(&target, window, cx)
}

fn choice(choice: &Choice, this: &WeakEntity<StudioWindow>, checked: bool) -> Item {
    Item::new(choice.name.clone()).checked(checked).on_select(run_on(this, open(choice.target.clone())))
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
    menu = menu.header(text(STUDIO_MENU_BUILT_IN));
    for family in &list.builtins {
        menu = match family.choices.as_slice() {
            [only] => menu.item(choice(only, this, false)),
            many => menu.submenu(
                family.name.clone(),
                many.iter().fold(Menu::new(), |sub, c| sub.item(choice(c, this, false))),
            ),
        };
    }
    let duplicate_key = keymap::key(ActionId::DuplicateTheme).and_then(KeyEquivalent::from_spec);
    menu.separator()
        .item(Item::new(text(STUDIO_MENU_NEW)).on_select(run_on(this, open(Target::New))))
        .item(
            Item::new(studio_menu_duplicate(facts.name))
                .key(duplicate_key)
                .on_select(run_on(this, |studio, window, cx| studio.duplicate(window, cx))),
        )
        .item(
            Item::new(text(STUDIO_MENU_RENAME))
                .on_select(run_on(this, |studio, window, cx| studio.start_rename(window, cx))),
        )
        .separator()
        .item(
            Item::new(text(STUDIO_MENU_REVERT))
                .enabled(facts.dirty)
                .on_select(run_on(this, |studio, window, cx| studio.discard_changes(window, cx))),
        )
}

#[cfg(test)]
mod tests {
    use gpui_kit::WeakEntity;

    use super::*;
    use crate::studio::theme_list::Family;

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
                    name: "Nord".into(),
                    choices: vec![Choice { name: "Nord".into(), target: Target::Builtin("Nord".into()) }],
                },
            ],
        }
    }

    #[test]
    fn the_menu_lists_your_themes_the_project_the_built_ins_and_what_to_do() {
        let subject = Subject::Own { path: "/themes/mine.json".into() };
        let facts = Facts { subject: &subject, name: "Mine", dirty: true };
        let menu = build(&list(), &facts, &WeakEntity::new_invalid());
        assert_eq!(
            menu.outline(),
            [
                "# Your Themes",
                "- Dusk",
                "- Mine [x]",
                "# This Project",
                "- The project\u{2019}s theme",
                "# Built-in",
                "> Wormhole",
                "  - Dark",
                "  - Light",
                "- Nord",
                "---",
                "- New Theme\u{2026}",
                "- Duplicate \u{201c}Mine\u{201d}",
                "- Rename\u{2026}",
                "---",
                "- Revert to Saved",
            ]
        );
    }

    #[test]
    fn revert_is_off_without_edits_and_a_copy_checks_nothing() {
        let subject = Subject::Untitled { from: "Nord".into() };
        let facts = Facts { subject: &subject, name: "Untitled (from Nord)", dirty: false };
        let menu = build(&ThemeList { project: false, ..list() }, &facts, &WeakEntity::new_invalid());
        let lines = menu.outline();
        assert!(lines.contains(&"- Revert to Saved [off]".to_string()), "{lines:?}");
        assert!(!lines.iter().any(|l| l.contains("[x]")), "{lines:?}");
        assert!(!lines.contains(&"# This Project".to_string()));
    }
}
