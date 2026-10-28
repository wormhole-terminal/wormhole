//! devctl's `folder-picker` command: open the picker, type into its field, walk it with the keys, click a row or
//! a crumb, pick, and read what it lists, so a scripted check can see the text, the ghost, the crumbs, the rows
//! with their badges, chevrons and recent marks, and the panel's frame. The queries run off the main thread,
//! so `state` right after a key may still say `busy true`: ask again.

use anyhow::{Result, bail};
use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::picker::{Field, Key};

use super::{FolderPicker, open};
use crate::platform::picker::Event;
use crate::window::Workspace;

const USAGE: &str = "folder-picker open|close|type <text>|tab|right [mid]|left [start|mid]|accept-ghost|up|down|\
pageup|pagedown|enter|pick|forget|click <line>|crumb <n>|scroll <lines>|state";

/// `folder-picker <what>`: see [`USAGE`]. Every word but `open` and `close` needs the picker to be open.
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
        "type" | "tab" | "right" | "left" | "accept-ghost" | "up" | "down" | "pageup" | "pagedown"
        | "enter" | "pick" | "forget" | "click" | "crumb" | "scroll" | "state" => {}
        _ => bail!("usage: {USAGE}"),
    }
    let Some(host) = ws.overlay_of::<FolderPicker>() else { bail!("the folder picker is not open") };
    let picker = host.read(cx).picker.clone();
    let rest = rest.trim();
    let number = |what: &str| rest.parse::<isize>().map_err(|_| anyhow::anyhow!("{what} needs a number"));
    picker.update(cx, |picker, cx| -> Result<String> {
        let text = picker.model.text().to_string();
        let at_end = Field::at_end(&text);
        let event = match word {
            "type" => {
                picker.set_text(rest, cx);
                return Ok(String::new());
            }
            "tab" => Event::Key(Key::Tab, at_end),
            "accept-ghost" => Event::Key(Key::Right, at_end),
            "right" if rest == "mid" => {
                Event::Key(Key::Right, Field { text: text.clone(), caret: text.len() / 2, selected: 0 })
            }
            "right" => Event::Key(Key::Right, at_end),
            "left" => Event::Key(
                Key::Left,
                match rest {
                    "start" => Field { text, caret: 0, selected: 0 },
                    "mid" => Field { caret: text.len() / 2, text, selected: 0 },
                    _ => at_end,
                },
            ),
            "up" => Event::Key(Key::Up, at_end),
            "down" => Event::Key(Key::Down, at_end),
            "pageup" => Event::Key(Key::PageUp, at_end),
            "pagedown" => Event::Key(Key::PageDown, at_end),
            "enter" | "pick" => Event::Key(Key::Enter, at_end),
            "forget" => Event::Key(Key::Forget, at_end),
            "click" => Event::Row(number("click")? as usize),
            "crumb" => Event::Crumb(number("crumb")? as usize),
            "scroll" => Event::Scroll(number("scroll")?),
            _ => return Ok(crate::picker::summary(picker)),
        };
        picker.feed(event, cx);
        Ok(String::new())
    })
}
