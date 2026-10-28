//! devctl's `folder-picker` command: open the picker, type into its field, walk it with the keys, pick, and
//! read what it lists, so a scripted check can see the folder, the filter, the rows and the selection.

use anyhow::{Result, bail};
use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;

use super::{FolderPicker, open};
use crate::window::Workspace;

const USAGE: &str = "folder-picker open|type <text>|tab|up|down|pick|close|state";

/// `folder-picker <what>`: see [`USAGE`]. `state` and every other word but `open` and `close` need the picker
/// to be open.
pub(crate) fn devctl_folder_picker(
    ws: &mut Workspace,
    arg: &str,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Result<String> {
    let arg = arg.trim_start();
    let (word, rest) = arg.split_once(' ').unwrap_or((arg, ""));
    match word {
        "open" => {
            if !super::flag_on(ws, cx) {
                bail!(
                    "the {} flag is off: `set feature_flags={{\"{}\":true}}`",
                    FeatureFlag::FolderPicker.title(),
                    FeatureFlag::FolderPicker.key()
                );
            }
            if !ws.folder_picker_open() {
                open(ws, window, cx);
            }
            return Ok(String::new());
        }
        "close" => {
            ws.close_overlay(window, cx);
            return Ok(String::new());
        }
        "type" | "tab" | "up" | "down" | "pick" | "state" => {}
        _ => bail!("usage: {USAGE}"),
    }
    let Some(picker) = ws.overlay_of::<FolderPicker>() else { bail!("the folder picker is not open") };
    // The picker closes through the workspace, which devctl is already updating: close it here, then insert.
    if word == "pick" {
        let (chosen, terminal) = picker.read_with(cx, |p, _| (p.state.choice(), p.terminal.clone()));
        let Some(inserted) = chosen else { return Ok("nothing selected\n".to_string()) };
        let there = terminal.upgrade().is_some_and(|terminal| terminal == ws.active_terminal());
        ws.close_overlay(window, cx);
        if !there {
            return Ok("not the terminal in front\n".to_string());
        }
        let _ = terminal.update(cx, |terminal, cx| {
            terminal.paste(&inserted);
            cx.notify();
        });
        return Ok(format!("inserted {inserted}\n"));
    }
    let rest = rest.to_string();
    Ok(picker.update(cx, |picker, cx| match word {
        "type" => {
            picker.field.update(cx, |field, cx| field.set_value(rest.clone(), window, cx));
            picker.typed(&rest, cx);
            String::new()
        }
        "tab" => {
            picker.complete(window, cx);
            String::new()
        }
        "up" | "down" => {
            picker.step(if word == "up" { -1 } else { 1 }, cx);
            String::new()
        }
        _ => picker.summary(),
    }))
}

impl FolderPicker {
    /// What the picker holds, for `state`: the folder listed, what narrows it, the rows with the selection
    /// marked, and where the card was drawn.
    fn summary(&self) -> String {
        let target = self.state.target();
        let mut out = format!(
            "folder {}\ntyped {:?}\nfilter {:?}\nprefix {:?}\nloaded {}\nunreadable {}\ncut {:?}\nrows {}\n",
            target.folder.display(),
            self.state.typed(),
            target.filter,
            target.prefix,
            self.state.loaded(),
            self.state.unreadable(),
            self.state.cut(),
            self.state.rows().len(),
        );
        for (ix, row) in self.state.rows().iter().enumerate() {
            let mark = if self.state.selected() == Some(ix) { '>' } else { ' ' };
            out.push_str(&format!("{mark} {}\n", row.name));
        }
        match self.placed.get() {
            Some(placed) => out.push_str(&format!(
                "card x {:.0} y {:.0} w {:.0} h {:.0} {} rows {}\n",
                placed.rect.x,
                placed.rect.y,
                placed.rect.w,
                placed.rect.h,
                if placed.above { "above" } else { "below" },
                placed.rows,
            )),
            None => out.push_str("card not drawn yet\n"),
        }
        out
    }
}
