//! What VoiceOver gets: the window's accessibility tree as GPUI last sent it to AppKit,
//! one node a line with its role, label and states, and the announcements made lately.

use std::time::Duration;

use anyhow::{Result, anyhow};
use gpui_kit::*;
use serde_json::Value;

use super::Ctx;
use crate::workspace::Workspace;

/// How long `a11y` waits for the tree to come up after waking it.
const WAKE_TIMEOUT: Duration = Duration::from_millis(3000);
const WAKE_POLL: Duration = Duration::from_millis(50);

/// `a11y [said]`: the accessibility tree (woken first, as an assistive app would), or with `said`
/// the announcements made lately (`!` before one that interrupts).
pub(super) fn a11y(ctx: &mut Ctx, arg: &str) -> Result<String> {
    match arg.trim() {
        "said" => Ok(crate::platform::a11y::recent()
            .into_iter()
            .map(|(text, interrupt)| format!("said {}{text}\n", if interrupt { "! " } else { "" }))
            .collect()),
        "" => {
            crate::platform::a11y::wake_tree(ctx.window);
            ctx.wait = Some(ctx.cx.spawn_in(ctx.window, async move |this, cx| dump(this, cx).await));
            Ok(String::new())
        }
        other => Err(anyhow!("expected nothing or `said`, got {other:?}")),
    }
}

/// `keyboard-navigation on|off|system`: stand in for macOS's Keyboard navigation (⇥ reaches the
/// title row's buttons too), or follow the system again. Without an argument, what it is.
pub(super) fn keyboard_navigation(_: &mut Ctx, arg: &str) -> Result<String> {
    let on = match arg.trim() {
        "on" => Some(true),
        "off" => Some(false),
        "system" => None,
        "" => {
            let on = crate::platform::a11y::keyboard_navigation();
            return Ok(format!("keyboard-navigation {}\n", if on { "on" } else { "off" }));
        }
        other => return Err(anyhow!("expected on|off|system, got {other:?}")),
    };
    crate::platform::a11y::set_keyboard_navigation(on);
    Ok(String::new())
}

/// `window-front on|off|system`: say this window is in front (or not) as far as the keyboard's
/// focus ring goes (it is hidden in a window that isn't), so a shot shows it without taking the
/// front from whatever app is there; `system` asks macOS again.
pub(super) fn window_front(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let front = match arg.trim() {
        "on" => Some(true),
        "off" => Some(false),
        "system" => None,
        other => return Err(anyhow!("expected on|off|system, got {other:?}")),
    };
    crate::workspace::set_front(front);
    ctx.cx.notify();
    Ok(String::new())
}

/// Draw until GPUI has built the tree (it builds it from the frame after it was woken), then
/// print it.
async fn dump(_: WeakEntity<Workspace>, cx: &mut AsyncWindowContext) -> String {
    let start = cx.background_executor().now();
    loop {
        // Outside the workspace's update: drawing renders it.
        let tree = cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
            window.is_a11y_active().then(|| window.debug_a11y_tree_json()).flatten()
        });
        match tree {
            Err(_) => return "a11y failed: the window closed\n".into(),
            Ok(Some(json)) => {
                return format_tree(&json).unwrap_or_else(|err| format!("a11y failed: {err:#}\n"));
            }
            Ok(None) => {}
        }
        if cx.background_executor().now() - start >= WAKE_TIMEOUT {
            return "a11y failed: the accessibility tree never came up\n".into();
        }
        cx.background_executor().timer(WAKE_POLL).await;
    }
}

/// GPUI's JSON as an indented outline: `Role "label" states`, the focused node marked `*`.
fn format_tree(json: &str) -> Result<String> {
    let tree: Value = serde_json::from_str(json)?;
    let nodes = tree.get("nodes").and_then(Value::as_object).ok_or_else(|| anyhow!("no nodes"))?;
    let root = tree.get("root").and_then(Value::as_str).ok_or_else(|| anyhow!("no root"))?;
    let focus = tree
        .get("active_descendant_focus")
        .and_then(Value::as_str)
        .or(tree.get("gpui_focus").and_then(Value::as_str));
    let mut out = String::new();
    let mut stack = vec![(root.to_string(), 0usize)];
    while let Some((id, depth)) = stack.pop() {
        let Some(node) = nodes.get(&id) else { continue };
        let marker = if focus == Some(id.as_str()) { "* " } else { "" };
        out.push_str(&format!("{}{marker}{}\n", "  ".repeat(depth), describe(node)));
        let children = node.get("children").and_then(Value::as_array).cloned().unwrap_or_default();
        // Reversed onto the stack, so the first child comes out first.
        for child in children.iter().rev().filter_map(Value::as_str) {
            stack.push((child.to_string(), depth + 1));
        }
    }
    Ok(out)
}

/// One node: its role, label and value, then its states.
fn describe(node: &Value) -> String {
    let aria = node.get("aria");
    let text = |key: &str| aria.and_then(|a| a.get(key)).and_then(Value::as_str);
    let number = |key: &str| aria.and_then(|a| a.get(key)).and_then(Value::as_u64);
    let flag = |key: &str| aria.and_then(|a| a.get(key)).and_then(Value::as_bool);
    let mut line = text("role").unwrap_or("?").to_string();
    if let Some(label) = text("label") {
        line.push_str(&format!(" {label:?}"));
    }
    if let Some(value) = text("value") {
        line.push_str(&format!(" value={value:?}"));
    }
    for (key, word) in [("selected", "selected"), ("expanded", "expanded")] {
        match flag(key) {
            Some(true) => line.push_str(&format!(" {word}")),
            Some(false) if key == "expanded" => line.push_str(" collapsed"),
            _ => {}
        }
    }
    if let Some(toggled) = text("toggled") {
        line.push_str(&format!(" toggled={}", toggled.to_lowercase()));
    }
    if let Some(level) = number("level") {
        line.push_str(&format!(" level={level}"));
    }
    if let (Some(at), Some(of)) = (number("position_in_set"), number("size_of_set")) {
        line.push_str(&format!(" {at}/{of}"));
    }
    if let Some(keys) = text("keyboard_shortcut") {
        line.push_str(&format!(" keys={keys}"));
    }
    if let Some(value) = aria.and_then(|a| a.get("numeric_value")).and_then(Value::as_f64) {
        line.push_str(&format!(" numeric={value}"));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::format_tree;

    #[test]
    fn the_tree_prints_as_an_outline_with_states_and_focus() {
        let json = r#"{
            "root": "a",
            "gpui_focus": "b",
            "active_descendant_focus": "c",
            "nodes": {
                "a": {"children": ["b"], "aria": {"role": "Window", "label": "Wormhole"}},
                "b": {"children": ["c", "d"], "aria": {"role": "Tree", "label": "Terminals"}},
                "c": {"aria": {"role": "TreeItem", "label": "dotfiles, 1 terminal, collapsed, 3 of 4",
                       "expanded": false, "level": 1, "position_in_set": 3, "size_of_set": 4}},
                "d": {"aria": {"role": "TreeItem", "label": "zsh", "selected": true, "level": 2,
                       "keyboard_shortcut": "Command+1", "toggled": "True"}}
            }
        }"#;
        assert_eq!(
            format_tree(json).expect("parses"),
            "Window \"Wormhole\"\n  Tree \"Terminals\"\n    * TreeItem \"dotfiles, 1 terminal, collapsed, 3 of 4\" \
             collapsed level=1 3/4\n    TreeItem \"zsh\" selected toggled=true level=2 keys=Command+1\n"
        );
    }
}
