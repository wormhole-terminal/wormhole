//! Typing, the pointer and the keyboard: what a person does to the window.

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use gpui_kit::*;

use super::{Ctx, coordinates};

pub(super) fn type_text(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.active_terminal().update(ctx.cx, |t, _| t.write_input(arg.as_bytes().to_vec()));
    Ok(String::new())
}

pub(super) fn paste(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.workspace.active_terminal().update(ctx.cx, |t, _| t.paste(arg));
    Ok(String::new())
}

/// The three steps of dragging a file in from the Finder.
pub(super) enum FileDrag {
    Enter,
    Drop,
    End,
}

pub(super) fn file_drag(ctx: &mut Ctx, arg: &str, step: FileDrag) -> Result<String> {
    let mut args = arg.splitn(3, ' ');
    let mut coord = || args.next().and_then(|v| v.parse::<f32>().ok()).map(px);
    let position = coord().zip(coord()).map(|(x, y)| point(x, y));
    let events = match (step, position) {
        (FileDrag::Enter, Some(position)) => vec![
            FileDropEvent::Entered {
                position,
                paths: ExternalPaths(args.next().map(PathBuf::from).into_iter().collect()),
            },
            FileDropEvent::Pending { position },
        ]
        .into_iter()
        .map(FileDropEvent::to_platform_input)
        .collect(),
        (FileDrag::Drop, Some(position)) => {
            vec![FileDropEvent::Pending { position }.to_platform_input(), release(position)]
        }
        (FileDrag::End, _) => vec![FileDropEvent::Exited.to_platform_input()],
        _ => return Err(anyhow!("expected <x> <y>")),
    };
    // The synthetic mouse events reach the workspace's own listeners, which can't run
    // while `poll` holds it: wait for the end of this update.
    ctx.window.defer(ctx.cx, move |window, cx| {
        for event in events {
            window.dispatch_event(event, cx);
        }
    });
    Ok(String::new())
}

/// The drop itself. GPUI turns a drop into a mouse-up and activates the app, as the Finder's drop
/// would; a test instance in background mode gets the same mouse-up without being brought forward.
fn release(position: Point<Pixels>) -> PlatformInput {
    if !crate::background::on() {
        return FileDropEvent::Submit { position }.to_platform_input();
    }
    PlatformInput::MouseUp(MouseUpEvent {
        button: MouseButton::Left,
        position,
        modifiers: Modifiers::default(),
        click_count: 1,
    })
}

/// The most one wheel event moves: a notch or so, as a trackpad's scroll arrives.
const SCROLL_STEP: f32 = 100.0;

/// `scroll <x> <y> <dy>`: turn the wheel over a point, `dy` logical pixels down (negative = up,
/// as the other scroll commands count). It arrives in steps with a frame drawn between them,
/// as a trackpad's scroll does: a virtual list measures the rows a step brings into view before
/// the next one, which is how the end of a long list is reached (`panel-scroll` moves the list's
/// state directly and can't pass rows it never measured). The rest of the batch waits for the
/// last step.
pub(super) fn scroll(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let position = coordinates(arg)?;
    let dy = arg
        .split_whitespace()
        .nth(2)
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|dy| dy.is_finite())
        .ok_or_else(|| anyhow!("expected <x> <y> <dy>"))?;
    ctx.wait = Some(ctx.cx.spawn_in(ctx.window, async move |_, cx| {
        let mut left = dy;
        while left != 0.0 {
            let step = left.clamp(-SCROLL_STEP, SCROLL_STEP);
            left -= step;
            let event = ScrollWheelEvent {
                position,
                // A wheel's delta is the way the content moves: up, for scrolling down.
                delta: ScrollDelta::Pixels(point(px(0.0), px(-step))),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            };
            let drawn = cx.update(|window, cx| {
                window.dispatch_event(PlatformInput::ScrollWheel(event), cx);
                window.draw(cx).clear(cx);
            });
            if drawn.is_err() {
                break;
            }
        }
        String::new()
    }));
    Ok(String::new())
}

pub(super) fn hover(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let event =
        MouseMoveEvent { position: coordinates(arg)?, pressed_button: None, modifiers: Modifiers::default() };
    ctx.window.defer(ctx.cx, move |window, cx| {
        window.dispatch_event(PlatformInput::MouseMove(event), cx);
    });
    Ok(String::new())
}

pub(super) enum Mouse {
    Down,
    Drag,
    Up,
}

/// Press the left button, move with it held, or release it.
pub(super) fn mouse(ctx: &mut Ctx, arg: &str, what: Mouse) -> Result<String> {
    let (position, modifiers, button) = (coordinates(arg)?, Modifiers::default(), MouseButton::Left);
    let pressing = matches!(what, Mouse::Down);
    let event = match what {
        Mouse::Down => PlatformInput::MouseDown(MouseDownEvent {
            button,
            position,
            modifiers,
            click_count: 1,
            first_mouse: false,
        }),
        Mouse::Drag => {
            PlatformInput::MouseMove(MouseMoveEvent { position, pressed_button: Some(button), modifiers })
        }
        Mouse::Up => PlatformInput::MouseUp(MouseUpEvent { button, position, modifiers, click_count: 1 }),
    };
    // Deferred like the clicks: the listeners need the workspace itself. A real
    // press comes after the pointer got there, which is what hit-testing goes by.
    let arrive = pressing.then_some(PlatformInput::MouseMove(MouseMoveEvent {
        position,
        pressed_button: None,
        modifiers,
    }));
    ctx.window.defer(ctx.cx, move |window, cx| {
        for event in arrive.into_iter().chain([event]) {
            window.dispatch_event(event, cx);
        }
    });
    Ok(String::new())
}

pub(super) fn click(ctx: &mut Ctx, arg: &str) -> Result<String> {
    clicks(ctx, arg, 1)
}

/// Two clicks in a row, the second one counted as a double-click (a resize handle's reset).
pub(super) fn double_click(ctx: &mut Ctx, arg: &str) -> Result<String> {
    clicks(ctx, arg, 2)
}

fn clicks(ctx: &mut Ctx, arg: &str, count: usize) -> Result<String> {
    let position = coordinates(arg)?;
    let modifiers = Modifiers::default();
    // Deferred like the drag events: the listeners need the workspace itself.
    ctx.window.defer(ctx.cx, move |window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent { position, pressed_button: None, modifiers }),
            cx,
        );
        for click_count in 1..=count {
            let events = [
                PlatformInput::MouseDown(MouseDownEvent {
                    button: MouseButton::Left,
                    position,
                    modifiers,
                    click_count,
                    first_mouse: false,
                }),
                PlatformInput::MouseUp(MouseUpEvent {
                    button: MouseButton::Left,
                    position,
                    modifiers,
                    click_count,
                }),
            ];
            for event in events {
                window.dispatch_event(event, cx);
            }
        }
    });
    Ok(String::new())
}

/// Type into whatever has the focus, one key per character.
pub(super) fn text(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let keys: Vec<Keystroke> = arg
        .chars()
        .filter_map(|c| Keystroke::parse(&if c == ' ' { "space".into() } else { c.to_string() }).ok())
        .collect();
    // Deferred like the clicks: the editors' listeners need the workspace itself.
    ctx.window.defer(ctx.cx, move |window, cx| {
        for keystroke in keys {
            window.dispatch_keystroke(keystroke, cx);
        }
    });
    Ok(String::new())
}

pub(super) fn key(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let keystroke = Keystroke::parse(arg.trim())?;
    // Deferred like the clicks: the workspace's key listener needs the workspace itself.
    ctx.window.defer(ctx.cx, move |window, cx| {
        let event = KeyDownEvent { keystroke, is_held: false, prefer_character_input: false };
        window.dispatch_event(PlatformInput::KeyDown(event), cx);
    });
    Ok(String::new())
}

#[cfg(any(feature = "inspector", debug_assertions))]
pub(super) fn inspect(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.window.toggle_inspector(ctx.cx);
    Ok(String::new())
}
