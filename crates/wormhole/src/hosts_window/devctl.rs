//! devctl's `hosts-window`, `hosts-shot [name]`, `hosts-toggle [id]` and `hosts-text`: open the window,
//! save a picture of it, open or fold a host's card, print what Copy all would copy.

use std::path::Path;

use anyhow::{Result, anyhow};
use gpui_kit::App;

use super::{open, open_handle, open_view};

/// What to print, or `None` if `cmd` isn't one of the hosts window's.
pub(crate) fn command(cmd: &str, arg: &str, dir: &Path, cx: &mut App) -> Result<Option<String>> {
    match cmd {
        "hosts-window" => {
            cx.defer(open);
            Ok(Some(String::new()))
        }
        "hosts-shot" => {
            let handle = open_handle(cx).ok_or_else(|| anyhow!("the hosts window is not open"))?;
            let name = if arg.is_empty() { "hosts-window" } else { arg };
            let path = dir.join(format!("{name}.png"));
            // Draw first: a window behind others may not have redrawn since its last change.
            let image = handle.update(cx, |_, window, cx| {
                window.draw(cx).clear(cx);
                window.render_to_image()
            })??;
            image.save(&path)?;
            Ok(Some(format!("saved {}\n", path.display())))
        }
        "hosts-scroll" => {
            let view = open_view(cx).ok_or_else(|| anyhow!("the hosts window is not open"))?;
            let y: f32 = arg.trim().parse()?;
            view.update(cx, |window, cx| {
                window.scroll.set_offset(gpui_kit::point(gpui_kit::px(0.0), gpui_kit::px(-y)));
                cx.notify();
            });
            Ok(Some(String::new()))
        }
        "hosts-toggle" => {
            // `hosts-toggle [id]`: open or fold a host's card, the first one without an id.
            let view = open_view(cx).ok_or_else(|| anyhow!("the hosts window is not open"))?;
            let id = view
                .read(cx)
                .report()
                .and_then(|report| {
                    let arg = arg.trim();
                    if arg.is_empty() { report.rows.first() } else { report.row(arg) }
                })
                .map(|row| row.id.clone())
                .ok_or_else(|| anyhow!("no such host"))?;
            view.update(cx, |window, cx| window.toggle(&id, cx));
            Ok(Some(String::new()))
        }
        "hosts-text" => {
            let view = open_view(cx).ok_or_else(|| anyhow!("the hosts window is not open"))?;
            let text = view
                .read(cx)
                .report()
                .map(|report| report.to_text())
                .ok_or_else(|| anyhow!("the window has not looked yet"))?;
            Ok(Some(text))
        }
        _ => Ok(None),
    }
}
