//! The notice chip from a script: say a notice, and read how each window's chip stands (the panel is
//! native and in no GPUI screenshot: `notices` gives the panel's window number for a capture of that one
//! window with `screencapture -l`).

use std::fmt::Write as _;

use anyhow::{Result, anyhow};
use gpui_kit::*;

use super::Ctx;
use crate::app_state::app;
use crate::platform::notice::Material;

/// `notice <text>`: put a notice that says `<text>` in this window, as a copy would. Does nothing with the
/// Notices flag off (`set feature_flags={"notices":true}`).
pub(super) fn notice(ctx: &mut Ctx, arg: &str) -> Result<String> {
    if !crate::notice::enabled(ctx.cx) {
        return Ok("notices are off (feature flag)\n".to_string());
    }
    crate::notice::test_notice(ctx.window, arg.trim(), ctx.cx);
    Ok(String::new())
}

/// `notices`: each window's chip, a line each: its number, the phase (`Hidden`, `Showing`, `Leaving`), the
/// words, and the panel if there is one: the chip's frame at rest in the window's points (x y w h), its
/// material and its window number.
pub(super) fn notices(ctx: &mut Ctx, _: &str) -> Result<String> {
    let windows: Vec<WindowId> = app(ctx.cx).windows.iter().map(|w| w.handle.window_id()).collect();
    let mut out = String::new();
    for report in crate::notice::notices(ctx.cx) {
        let number = windows.iter().position(|id| *id == report.window).map_or(0, |ix| ix + 1);
        write!(out, "window {number} {:?} {:?}", report.phase, report.words)?;
        if let Some(panel) = report.panel {
            let [x, y, w, h] = panel.frame;
            write!(
                out,
                " frame {x} {y} {w} {h} material {} panel {}",
                panel.material.name(),
                panel.window_number
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
