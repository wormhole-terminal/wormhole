//! What devctl drives in the Theme Studio window (`studio-*` commands, `state`'s studio line).
//! Compiled with the `devctl` feature only.

use gpui_kit::*;

use super::draft::Show;
use super::gallery::Area;
use super::title::SaveAction;
use super::{StudioWindow, WINDOW_MIN};

/// ⌥⌘, (Wormhole → Theme Studio), as devctl's `studio` presses it.
pub(crate) use super::toggle_studio;

#[cfg(feature = "devctl")]
/// What happens to the gallery before each frame `studio-frames` draws.
#[derive(Clone, Copy)]
enum FrameMode {
    /// Nothing: the window draws again.
    Rest,
    /// An animation's tick: the gallery is notified.
    Tick,
    /// A wheel tick: the gallery scrolls 40 px further down.
    Scroll,
}

impl FrameMode {
    fn parse(word: &str) -> Self {
        match word {
            "scroll" => Self::Scroll,
            "tick" => Self::Tick,
            _ => Self::Rest,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Rest => "rest",
            Self::Tick => "tick",
            Self::Scroll => "scroll",
        }
    }
}

#[cfg(feature = "devctl")]
/// devctl `studio-frames`: draw the Studio window `frames` times and append the timings to
/// `out`, each frame after what `mode` says happened to the gallery.
pub(crate) fn frames(frames: usize, mode: &str, out: std::path::PathBuf, cx: &mut App) -> anyhow::Result<()> {
    let (handle, studio) =
        super::current_window(cx).ok_or_else(|| anyhow::anyhow!("Theme Studio not open"))?;
    let mode = FrameMode::parse(mode);
    // Weak: the loop must not keep the Studio alive when its window closes meanwhile.
    let studio = studio.downgrade();
    // Outside the tick's update: drawing renders (and so borrows) the windows.
    cx.spawn(async move |cx| {
        let mut times = Vec::with_capacity(frames);
        for _ in 0..frames {
            let Some(elapsed) = cx.update(|cx| {
                let studio = studio.upgrade()?;
                handle
                    .update(cx, |_, window, cx| {
                        let gallery = studio.read(cx).gallery.clone();
                        match mode {
                            FrameMode::Rest => {}
                            FrameMode::Tick => gallery.update(cx, |_, cx| cx.notify()),
                            FrameMode::Scroll => {
                                gallery.update(cx, |gallery, cx| gallery.scroll_by(40.0, cx))
                            }
                        }
                        let start = std::time::Instant::now();
                        let arena = window.draw(cx);
                        arena.clear(cx);
                        start.elapsed()
                    })
                    .ok()
            }) else {
                return;
            };
            times.push(elapsed.as_secs_f64() * 1000.0);
            cx.background_executor().timer(std::time::Duration::from_millis(2)).await;
        }
        // The first frames warm caches (shaped text, icons); report the rest.
        let steady = &times[times.len().min(3)..];
        let avg = steady.iter().sum::<f64>() / steady.len().max(1) as f64;
        let max = steady.iter().cloned().fold(0.0, f64::max);
        let line = format!(
            "studio-frames {} n={} avg={avg:.2}ms max={max:.2}ms first={:.2}ms\n",
            mode.name(),
            times.len(),
            times[0]
        );
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).create(true).open(out) {
            use std::io::Write as _;
            let _ = file.write_all(line.as_bytes());
        }
    })
    .detach();
    Ok(())
}

#[cfg(feature = "devctl")]
/// devctl `state`: the Studio's line while it is open: whether it has edits, and whether a
/// sheet (which no shot shows) is waiting for an answer.
pub(crate) fn state(cx: &App) -> Option<String> {
    let (_, studio) = super::current_window(cx)?;
    let studio = studio.read(cx);
    Some(format!("studio dirty={} asking={}", studio.dirty, studio.asking))
}

/// Run `f` on the open Studio (devctl drives it from a window's tick).
#[cfg(feature = "devctl")]
pub(crate) fn with_studio(
    cx: &mut App,
    f: impl FnOnce(&mut StudioWindow, &mut Window, &mut Context<StudioWindow>),
) -> anyhow::Result<()> {
    let (handle, studio) =
        super::current_window(cx).ok_or_else(|| anyhow::anyhow!("Theme Studio not open"))?;
    handle.update(cx, |_, window, cx| studio.update(cx, |s, cx| f(s, window, cx)))
}

/// devctl: render the Studio window to a PNG.
#[cfg(feature = "devctl")]
pub(crate) fn shot(path: &std::path::Path, cx: &mut App) -> anyhow::Result<()> {
    let (handle, _) = super::current_window(cx).ok_or_else(|| anyhow::anyhow!("Theme Studio not open"))?;
    // Draw first: a window behind others may not have redrawn since its last change.
    let image = handle.update(cx, |_, window, cx| {
        window.draw(cx).clear(cx);
        window.render_to_image()
    })??;
    image.save(path)?;
    Ok(())
}

impl StudioWindow {
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_select(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.select_token(name, window, cx);
    }

    /// `studio-save <name>`: save under that name, replacing a theme of that name without a
    /// question (a script that saves twice under one name means to). Without a name, Save
    /// theme as the person's ⌘S does it, so a name clash shows its notice.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_save(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if name.is_empty() {
            return self.request_save(window, cx);
        }
        let name = name.to_string();
        self.name.update(cx, |i, cx| i.set_value(name.clone(), window, cx));
        self.write_theme(name, cx);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_scroll(&mut self, y: f32, cx: &mut Context<Self>) {
        self.gallery.update(cx, |gallery, cx| gallery.devctl_scroll(y, cx));
    }

    /// Type `value` into the field and press Return, as the person would: an expression
    /// that doesn't work stays in the field with its error.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_set(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.expression.update(cx, |e, cx| e.set_value(value.to_string(), window, cx));
        self.check_field(value, cx);
        self.apply_field(value.trim().to_string(), window, cx);
    }

    /// `studio-type <text>`: type `text` into the field without pressing Return, so what the
    /// field offers for it shows.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_type(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.expression.update(cx, |e, cx| e.set_value(value.to_string(), window, cx));
        self.check_field(value, cx);
        self.refresh_completions(cx);
    }

    /// `studio-filter <all|changed|low> [text]` and `studio-area <area>`.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_filter(&mut self, arg: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (show, text) = arg.split_once(' ').unwrap_or((arg, ""));
        let show = match show {
            "changed" => Show::Changed,
            "low" => Show::LowContrast,
            _ => Show::All,
        };
        self.list.update(cx, |list, cx| list.devctl_filter(show, text, window, cx));
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_area(&mut self, arg: &str, cx: &mut Context<Self>) {
        let area =
            Area::ALL.iter().copied().find(|a| a.label().eq_ignore_ascii_case(arg)).unwrap_or_default();
        self.set_area(area, cx);
    }

    /// `studio-resize <w> <h>`: the window's size, as far as its minimum lets it (AppKit holds
    /// the minimum against the person's hand on the edge, not against a resize from code).
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_resize(&mut self, arg: &str, window: &mut Window) {
        let mut dims = arg.split_whitespace().filter_map(|n| n.parse::<f32>().ok());
        if let (Some(w), Some(h)) = (dims.next(), dims.next()) {
            window.resize(size(px(w.max(WINDOW_MIN.0)), px(h.max(WINDOW_MIN.1))));
        }
    }

    /// `studio-menu [save|copy|switch|json|project]`: open the Save theme menu, or pick
    /// one of its entries (Export's file dialog is the person's to answer, so it isn't one).
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_menu(&mut self, arg: &str, cx: &mut Context<Self>) {
        let action = match arg {
            "save" => Some(SaveAction::Save),
            "copy" => Some(SaveAction::SaveCopy),
            "switch" => Some(SaveAction::SaveCopyAndSwitch),
            "json" => Some(SaveAction::CopyJson),
            "project" => Some(SaveAction::UseInProject),
            _ => None,
        };
        // devctl runs inside the main window's update, where its workspace (which Use in this
        // project reads) can't be read: go on once that is over.
        cx.defer(move |cx| {
            let _ = with_studio(cx, |studio, window, cx| match action {
                Some(action) => studio.run_save_action(action, window, cx),
                None => studio.open_save_menu(cx),
            });
        });
    }
}
