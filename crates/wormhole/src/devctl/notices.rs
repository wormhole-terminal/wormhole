//! The notice chip from a script: say a notice, and read how each window's chip stands (the panel is
//! native and in no GPUI screenshot: `notices` gives the panel's window number for a capture of that one
//! window with `screencapture -l`).

use std::fmt::Write as _;

use anyhow::{Result, anyhow};
use gpui_kit::*;

use super::Ctx;
use crate::app_state::app;
use crate::platform::notice::Material;
use wormhole_core::view::notice::Press;

/// `notice <text>`: put a notice that says `<text>` in this window, as a copy would. Does nothing with the
/// Notices flag off (`set feature_flags={"notices":true}`).
///
/// `notice sticky [<text>]` puts a sticky one: it has no life and stays until it is clicked (`notice-click`).
/// Without `<text>` it is the real warning that a terminal's session host couldn't start, with its own
/// words; with `<text>` a warning that says that. A dismissed one is not said again by the same words.
pub(super) fn notice(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let arg = arg.trim();
    if arg == "sticky" || arg == "sticky again" {
        crate::notice::test_sticky(ctx.window, None, arg.ends_with("again"), ctx.cx);
    } else if let Some(words) = arg.strip_prefix("sticky ") {
        crate::notice::test_sticky(ctx.window, Some(words.trim()), false, ctx.cx);
    } else {
        crate::notice::test_notice(ctx.window, arg, ctx.cx);
    }
    Ok(String::new())
}

/// `notice-click [close|info|body]`: click this window's chip at the × (the default), at the ⓘ or on the
/// capsule, as a person does, over the channel the panel uses. The × dismisses a sticky notice; the ⓘ and
/// the capsule open its explanation, or dismiss it where it has none; a copy's chip takes no click, and
/// nothing happens. Needs a chip (a real window, not a test).
pub(super) fn click(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let press = match arg.trim() {
        "" | "close" => Press::Close,
        "info" => Press::Info,
        "body" => Press::Body,
        other => return Err(anyhow!("expected close, info or body, got {other:?}")),
    };
    let id = ctx.window.window_handle().window_id();
    if crate::notice::click(id, press, ctx.cx) {
        Ok(String::new())
    } else {
        Err(anyhow!("no chip to click in this window"))
    }
}

/// `notices`: each window's chip, a line each: its number, the phase (`Hidden`, `Showing`, `Leaving`), the
/// words, `sticky`, `clickable` and `hidden` (the panel waits out of sight) when they are so, `look <appearance> [plain]`, the terminal it is about (its pane's x y w h, and the window's size) and the panel if there is one: the chip's frame at rest in the window's points (x y w h), its
/// material, its window number and the app window's.
pub(super) fn notices(ctx: &mut Ctx, _: &str) -> Result<String> {
    let windows: Vec<WindowId> = app(ctx.cx).windows.iter().map(|w| w.handle.window_id()).collect();
    let mut out = String::new();
    for report in crate::notice::notices(ctx.cx) {
        let number = windows.iter().position(|id| *id == report.window).map_or(0, |ix| ix + 1);
        write!(out, "window {number} {:?} {:?}", report.phase, report.words)?;
        if report.sticky {
            out.push_str(" sticky");
        }
        if report.clickable {
            out.push_str(" clickable");
        }
        if report.buttons.info {
            out.push_str(" info");
        }
        if report.buttons.close {
            out.push_str(" close");
        }
        if report.info_open {
            out.push_str(" info-open");
        }
        if report.hidden {
            out.push_str(" hidden");
        }
        if let Some((theme, plain)) = report.look {
            write!(out, " look {theme:?}{}", if plain { " plain" } else { "" })?;
        }
        if let Some(spot) = report.spot {
            let r = spot.anchor;
            write!(out, " terminal {} {} {} {} in {}x{}", r.x, r.y, r.w, r.h, spot.window.0, spot.window.1)?;
            if spot.full_screen {
                out.push_str(" full-screen");
            }
        }
        if let Some(panel) = report.panel {
            let [x, y, w, h] = panel.frame;
            write!(
                out,
                " frame {x} {y} {w} {h} material {}{} panel {} window {}",
                panel.material.name(),
                if panel.takes_mouse { " mouse" } else { "" },
                panel.window_number,
                panel.parent_number
            )?;
        }
        out.push('\n');
    }
    if out.is_empty() {
        out.push_str("none\n");
    }
    Ok(out)
}

/// `notice-material glass|frosted|solid|system`: the chip is made of that from the next one on
/// (`system`: as the Mac allows), to see all three on one Mac.
pub(super) fn material(_: &mut Ctx, arg: &str) -> Result<String> {
    let material = match arg.trim() {
        "glass" => Some(Material::Glass),
        "frosted" => Some(Material::Frosted),
        "solid" => Some(Material::Solid),
        "system" => None,
        other => return Err(anyhow!("expected glass, frosted, solid or system, got {other:?}")),
    };
    crate::platform::notice::force_material(material);
    Ok(String::new())
}
