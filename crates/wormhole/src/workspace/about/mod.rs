//! The About window (Wormhole → About Wormhole: icon, version and copyright), and the
//! Licenses window (Help → Licenses).

mod licenses;

use super::aux_window;
use super::*;
use crate::icons::app_icon;

pub(crate) use licenses::open_licenses;

/// The system About panel's proportions (Terminal's is 284 wide; the copyright line here is longer), and
/// its icon.
const SIZE: (f32, f32) = (340.0, 196.0);
const ICON: f32 = 64.0;

/// `packaging/app.env` is where the app's identity lives; the bundle's Info.plist is filled from
/// it too, so the copyright line here can't drift from the one Finder shows.
const APP_ENV: &str = include_str!("../../../../../packaging/app.env");

/// The value of `key` in a `KEY=value` file, without surrounding quotes.
fn env_value<'a>(env: &'a str, key: &str) -> Option<&'a str> {
    env.lines()
        .filter_map(|line| line.trim().strip_prefix(key)?.strip_prefix('='))
        .map(|value| value.trim().trim_matches('"'))
        .next()
}

fn version_line() -> String {
    let version = env!("CARGO_PKG_VERSION");
    if cfg!(debug_assertions) {
        format!("Version {version} (debug build)")
    } else {
        format!("Version {version}")
    }
}

/// About's and Licenses' colours: opaque windows, with the theme's own text colours. It shadows
/// `settings::ui` for both (Licenses reaches it through `use super::*`).
fn ui(cx: &App) -> UiColors {
    crate::settings::window_ui(cx, false)
}

pub(crate) struct AboutWindow {
    focus_handle: FocusHandle,
}

/// Open the About window, or bring it to the front.
pub(crate) fn open_about(cx: &mut App) {
    let size = size(px(SIZE.0), px(SIZE.1));
    let mut options = aux_window::window_options("About Wormhole", size, size, cx);
    options.is_resizable = false;
    options.is_minimizable = false;
    aux_window::open_single(
        |state| &mut state.about_window,
        options,
        |window, cx| cx.new(|cx| AboutWindow::new(window, cx)),
        cx,
    );
}

impl AboutWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);
        Self { focus_handle }
    }
}

impl Render for AboutWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        // Opaque: a small window of text, not a place for the blur.
        let surfaces = aux_window::surfaces(cx, false);
        let copyright = env_value(APP_ENV, "COPYRIGHT").unwrap_or_default().to_string();

        // The system's About panel, as Terminal and Finder show it: icon, name, version, copyright,
        // nothing to press. Updates are in the app menu and the licences in Help.
        div()
            .id("about-window")
            .track_focus(&self.focus_handle)
            .key_context("AboutWindow")
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, window, _| {
                aux_window::close_on_esc_or_cmd_w(event, window);
            }))
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .px_4()
            .pt(px(style::TITLE_ROW_H))
            .bg(surfaces.background)
            .text_color(ui.text)
            // First, so it is behind what is drawn over it (`chrome::TitleDrag`).
            .child(aux_window::drag_strip())
            .child(img(app_icon(cx)).size(px(ICON)).flex_none())
            .child(div().mt_2().text_heading().font_weight(FontWeight::BOLD).child("Wormhole"))
            .child(div().mt_1().text_caption().text_color(ui.muted).child(version_line()))
            .children(
                (!copyright.is_empty())
                    .then(|| div().mt_1().text_caption().text_color(ui.muted).text_center().child(copyright)),
            )
    }
}

#[cfg(feature = "devctl")]
pub(crate) use devctl::command as devctl_command;

#[cfg(feature = "devctl")]
mod devctl {
    use std::path::Path;

    use super::*;

    /// The About and Licenses commands of devctl: what to print, or `None` if `cmd` isn't one.
    pub(crate) fn command(cmd: &str, arg: &str, dir: &Path, cx: &mut App) -> anyhow::Result<Option<String>> {
        match cmd {
            "about" => cx.defer(open_about),
            "licenses" => cx.defer(open_licenses),
            "about-shot" | "licenses-shot" => {
                let (slot, default) = if cmd == "about-shot" {
                    (app(cx).about_window.clone(), "about")
                } else {
                    (app(cx).licenses_window.clone(), "licenses")
                };
                let (handle, _) = slot.ok_or_else(|| anyhow::anyhow!("that window is not open"))?;
                let name = if arg.is_empty() { default } else { arg };
                let path = dir.join(format!("{name}.png"));
                // Draw first: a window behind others may not have redrawn since its last change.
                let image = handle.update(cx, |_, window, cx| {
                    window.draw(cx).clear(cx);
                    window.render_to_image()
                })??;
                image.save(&path)?;
                return Ok(Some(format!("saved {}\n", path.display())));
            }
            "licenses-search" | "licenses-select" => return licenses::devctl(cmd, arg, cx).map(Some),
            _ => return Ok(None),
        }
        Ok(Some(String::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::env_value;

    #[test]
    fn env_values_lose_their_quotes() {
        let env =
            "# comment\nAPP_NAME=Wormhole\nCOPYRIGHT=\"Copyright © 2026 Someone. All rights reserved.\"\n";
        assert_eq!(env_value(env, "APP_NAME"), Some("Wormhole"));
        assert_eq!(env_value(env, "COPYRIGHT"), Some("Copyright © 2026 Someone. All rights reserved."));
        assert_eq!(env_value(env, "MISSING"), None);
    }

    #[test]
    fn a_key_is_not_matched_by_its_prefix() {
        assert_eq!(env_value("APP_NAME_LONG=x\n", "APP_NAME"), None);
    }

    #[test]
    fn the_bundled_env_has_a_copyright() {
        let copyright = env_value(super::APP_ENV, "COPYRIGHT").unwrap_or_default();
        assert!(copyright.starts_with("Copyright"), "packaging/app.env has no COPYRIGHT: {copyright:?}");
    }
}
