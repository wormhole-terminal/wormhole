//! Profile: the name, colour and avatar of the profile Wormhole is in, where its terminals start,
//! the folders that open in it, its environment, git identity, agent logins, and deleting it.
//! These live in `profiles.json`, not in `config.json`, so no row has a reset arrow; the text
//! fields are made in `profile_form`.

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState, Textarea};
use gpui_kit::*;
use wormhole_core::profile::Profile;
use wormhole_core::settings_index::SectionId as Section;
use wormhole_core::sidebar::GroupColor;

use super::profile_form::{LOGIN_AGENTS, Pick, ProfileForm};
use crate::settings::SettingsStore;
use crate::style::{self, TextRole};
use crate::workspace::app::{AppState, app};
use crate::workspace::settings_page::registry::Setting;
use crate::workspace::settings_page::{SettingsWindow, ui};

pub(super) const SETTINGS: &[Setting] = &[
    Setting::custom("profile-name", draw_name),
    Setting::custom("profile-start", draw_start).describe(start_caption).below(folder_list),
    Setting::custom("profile-env", draw_env),
    Setting::custom("profile-git", draw_git),
    Setting::custom("profile-logins", draw_logins),
    Setting::custom("profile-delete", draw_delete).describe(delete_caption).below(delete_question),
];

/// The widths of the fields.
const NAME_W: f32 = 180.0;
const AVATAR_FIELD_W: f32 = 48.0;
const PATH_W: f32 = 240.0;
const GIT_NAME_W: f32 = 140.0;
const GIT_EMAIL_W: f32 = 200.0;
const ENV_W: f32 = 340.0;
const AGENT_LABEL_W: f32 = 48.0;

/// A text field of `width`.
fn field(input: &Entity<InputState>, width: f32) -> Div {
    div().flex_none().w(px(width)).child(Input::new(input).small())
}

/// The profile Wormhole is in, copied out so no borrow of the app is held while a row is built.
fn shown(window: &SettingsWindow, cx: &App) -> Option<(Profile, ColorPalette)> {
    let form = window.profile_form.as_ref()?;
    let profile = app(cx).profiles.get(&form.id)?.clone();
    let tokens = &cx.global::<SettingsStore>().tokens;
    let palette = GroupColor::ALL.map(|color| (color, crate::theme_bridge::hsla(color.color(tokens))));
    Some((profile, palette))
}

type ColorPalette = [(GroupColor, Hsla); 9];

fn color_of(palette: &ColorPalette, color: GroupColor) -> Hsla {
    palette.iter().find(|(c, _)| *c == color).map_or(palette[0].1, |(_, hsla)| *hsla)
}

fn draw_name(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let (Some((profile, palette)), Some(form)) = (shown(window, cx), window.profile_form.as_ref()) else {
        return div().into_any_element();
    };
    let typed_letter = form
        .avatar
        .read(cx)
        .value()
        .chars()
        .next()
        .unwrap_or_else(|| wormhole_core::profile::first_letter(&form.name.read(cx).value()));
    div()
        .flex()
        .flex_col()
        .items_end()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(style::profile_avatar(
                    ui,
                    color_of(&palette, profile.color),
                    typed_letter,
                    style::TITLE_GLYPH,
                ))
                .child(field(&form.name, NAME_W))
                .child(field(&form.avatar, AVATAR_FIELD_W)),
        )
        .child(div().flex().items_center().children(palette.iter().enumerate().map(|(ix, (color, hsla))| {
            let color = *color;
            style::color_choice(
                ("profile-color", ix),
                ui,
                color.label(),
                Some(*hsla),
                color == profile.color,
                true,
            )
            .on_click(cx.listener(move |this, _, _, cx| this.set_profile_color(color, cx)))
        })))
        .into_any_element()
}

fn draw_start(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let Some(form) = window.profile_form.as_ref() else { return div().into_any_element() };
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(field(&form.start, PATH_W))
        .child(choose("profile-start-choose", Pick::Start, cx))
        .into_any_element()
}

/// The button that picks a folder for a field.
fn choose(id: impl Into<ElementId>, pick: Pick, cx: &mut Context<SettingsWindow>) -> Button {
    Button::new(id)
        .outline()
        .small()
        .icon(IconName::Folder)
        .tooltip("Choose a folder")
        .on_click(cx.listener(move |this, _, window, cx| this.pick_profile_folder(pick, window, cx)))
}

/// "Folders under ~/work and ~/clients open in Work too."
fn start_caption(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let (profile, _) = shown(window, cx)?;
    let folders = profile.folders.iter().map(String::as_str).collect::<Vec<_>>();
    Some(
        match folders.as_slice() {
            [] => {
                format!("Where a new terminal starts. A folder you add below opens in {} too.", profile.name)
            }
            [one] => format!("Folders under {one} open in {} too.", profile.name),
            [head @ .., last] => {
                format!("Folders under {} and {last} open in {} too.", head.join(", "), profile.name)
            }
        }
        .into(),
    )
}

/// The folders that open in the profile, with a way to take one out and to add one.
fn folder_list(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let ui = ui(cx);
    let (profile, _) = shown(window, cx)?;
    let rows = profile.folders.iter().enumerate().map(|(ix, folder)| {
        let folder = folder.clone();
        div()
            .flex()
            .items_center()
            .gap_2()
            .h(px(style::CONTROL_H))
            .child(style::icon(IconName::Folder).text_color(ui.muted))
            .child(div().flex_1().min_w_0().truncate().text_body().child(folder.clone()))
            .child(
                Button::new(("profile-folder-remove", ix))
                    .ghost()
                    .small()
                    .icon(IconName::X)
                    .tooltip("Stop opening this folder in the profile")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let folder = folder.clone();
                        this.edit_profile(move |p| p.folders.retain(|f| *f != folder), cx);
                    })),
            )
    });
    Some(
        div()
            .flex()
            .flex_col()
            .pb_2()
            .children(rows)
            .child(div().pt_1().child(
                style::link_button("profile-folder-add", ui, "Add a folder…", false).on_click(
                    cx.listener(|this, _, window, cx| this.pick_profile_folder(Pick::Folder, window, cx)),
                ),
            ))
            .into_any_element(),
    )
}

fn draw_env(window: &SettingsWindow, _: &mut Context<SettingsWindow>) -> AnyElement {
    let Some(form) = window.profile_form.as_ref() else { return div().into_any_element() };
    div().flex_none().w(px(ENV_W)).child(Textarea::new(&form.env).small()).into_any_element()
}

fn draw_git(window: &SettingsWindow, _: &mut Context<SettingsWindow>) -> AnyElement {
    let Some(form) = window.profile_form.as_ref() else { return div().into_any_element() };
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(field(&form.git_name, GIT_NAME_W))
        .child(field(&form.git_email, GIT_EMAIL_W))
        .into_any_element()
}

fn draw_logins(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(form) = window.profile_form.as_ref() else { return div().into_any_element() };
    div()
        .flex()
        .flex_col()
        .items_end()
        .gap_2()
        .children(LOGIN_AGENTS.iter().zip(&form.logins).enumerate().map(|(ix, ((_, name, _), input))| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(AGENT_LABEL_W)).text_body().text_color(ui.muted).child(*name))
                .child(field(input, PATH_W))
                .child(choose(("profile-login-choose", ix), Pick::Login(ix), cx))
        }))
        .into_any_element()
}

fn delete_caption(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let (profile, _) = shown(window, cx)?;
    profile
        .id
        .is_default()
        .then(|| "The Default profile stays: it holds the data Wormhole always had.".into())
}

fn draw_delete(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let is_default = app(cx).active_profile().is_default();
    Button::new("profile-delete")
        .outline()
        .small()
        .icon(IconName::Trash)
        .label("Delete profile…")
        .disabled(is_default || window.confirm_delete_profile)
        .on_click(cx.listener(|this, _, _, cx| {
            this.confirm_delete_profile = true;
            cx.notify();
        }))
        .into_any_element()
}

/// The question under the row once "Delete profile…" was pressed.
fn delete_question(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    if !window.confirm_delete_profile {
        return None;
    }
    let ui = ui(cx);
    let (profile, _) = shown(window, cx)?;
    let id = profile.id.clone();
    Some(
        style::banner(
            ui,
            style::BannerKind::Danger,
            false,
            format!(
                "Delete {}? Its terminals, notes, tasks and settings are removed, and its windows close. Wormhole goes to Default.",
                profile.name
            ),
        )
        .child(Button::new("profile-delete-cancel").ghost().small().label("Cancel").on_click(cx.listener(
            |this, _, _, cx| {
                this.confirm_delete_profile = false;
                cx.notify();
            },
        )))
        .child(Button::new("profile-delete-confirm").danger().small().label("Delete").on_click(cx.listener(
            move |this, _, _, cx| {
                this.confirm_delete_profile = false;
                let id = id.clone();
                let page = cx.weak_entity();
                cx.defer(move |cx| {
                    if let Err(message) = AppState::delete_profile(&id, cx) {
                        page.update(cx, |this, cx| {
                            this.notice = Some((Section::Profile, message.into()));
                            cx.notify();
                        })
                        .ok();
                    }
                });
                cx.notify();
            },
        )))
        .into_any_element(),
    )
}

impl ProfileForm {
    /// What the page says above its rows when an edit was refused.
    pub(in crate::workspace::settings_page) fn problem(&self) -> Option<SharedString> {
        self.error.clone()
    }
}
