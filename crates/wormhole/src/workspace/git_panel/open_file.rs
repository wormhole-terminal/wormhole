//! The small icon that opens a file of the git panel in the editor Open in… uses: on a Changes or
//! History row and on a diff line while the pointer is over it, and in the diff's header. A click
//! asks that app to open the file (at the line, where the app can: `wormhole_core::open_file`);
//! for Finder it reveals the file.
//!
//! As for Open in… itself, the person asks an app they chose to open a file, so nothing from the
//! project runs and project trust (`ProjectHub::enabled`) doesn't come into it. What the path
//! names is checked first, though: repository content is hostile, so a path that leads out of
//! the repository (a planted symlink) is refused (`fsutil::resolve_within`).

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::{self, IconButton};
use crate::workspace::open_in::{self, OpenIn};
use wormhole_core::fsutil;
use wormhole_core::open_file::{self, Launch};
use wormhole_core::open_in::Installed;
use wormhole_core::spoken;

/// The repository, and the app a click opens its files in: worked out once per frame of a list,
/// not per row.
#[derive(Clone)]
pub(crate) struct OpenTarget(Rc<Target>);

struct Target {
    root: PathBuf,
    app: Installed,
}

impl OpenTarget {
    pub(crate) fn new(root: &Path, cx: &App) -> Self {
        Self(Rc::new(Target { root: root.to_path_buf(), app: OpenIn::resolved(cx).app }))
    }

    /// One file of the repository, by its path relative to the root.
    pub(crate) fn file(&self, path: &str) -> OpenFile {
        OpenFile { target: self.clone(), path: path.to_string().into() }
    }
}

#[derive(Clone)]
pub(crate) struct OpenFile {
    target: OpenTarget,
    path: SharedString,
}

impl OpenFile {
    fn tooltip(&self, line: Option<u32>) -> String {
        let app = &self.target.0.app;
        open_file::tooltip(&app.bundle, &app.name, line)
    }

    fn open(&self, line: Option<u32>) {
        let Target { root, app } = &*self.target.0;
        open(&app.bundle, root, &self.path, line);
    }

    fn button(&self, id: impl Into<ElementId>, ui: UiColors, line: Option<u32>) -> IconButton {
        let tooltip = self.tooltip(line);
        let name = self.path.rsplit_once('/').map_or(&*self.path, |(_, name)| name);
        let spoken = spoken::open_file(&tooltip, name);
        let this = self.clone();
        style::icon_button(id, ui, IconName::ExternalLink, tooltip).spoken(spoken).on_click(
            move |_, _, cx| {
                // The click is the icon's, not the row's (which opens the diff).
                cx.stop_propagation();
                this.open(line);
            },
        )
    }

    /// The icon in a diff's header, always there, beside the other controls.
    pub(crate) fn header_button(&self, ui: UiColors) -> IconButton {
        self.button("diff-open-in-editor", ui, None).in_title_row()
    }

    /// `row` with the icon at its right end while the pointer is over the row: it takes no room
    /// until then (`style::row_actions`), when the text before it gives way.
    pub(crate) fn on_row(&self, row: Stateful<Div>, ui: UiColors, id: &str) -> Stateful<Div> {
        let group = SharedString::from(format!("open-row-{id}"));
        let action = self.button(SharedString::from(format!("open-{id}")), ui, None).in_row();
        row.group(group.clone()).child(style::row_actions(
            SharedString::from(format!("open-actions-{id}")),
            group,
            false,
            vec![action.into_any_element()],
        ))
    }

    /// The icon at the right end of a diff line, opening the file at `line`, while the pointer is
    /// over the line (`group`). `height` is the line's: the icon is clipped to it.
    pub(crate) fn on_line(
        &self,
        row: Stateful<Div>,
        ui: UiColors,
        group: &'static str,
        (id, box_id): (ElementId, ElementId),
        line: u32,
        height: f32,
    ) -> Stateful<Div> {
        let action = self.button(id, ui, Some(line)).in_row();
        row.relative().group(group).child(
            div()
                .absolute()
                .top_0()
                .right_3()
                .h(px(height))
                .flex()
                .items_center()
                .overflow_hidden()
                .child(style::row_actions(box_id, group, false, vec![action.into_any_element()])),
        )
    }
}

/// Open `relative` (a path of the repository at `root`) in the app with this bundle identifier.
pub(crate) fn open(bundle: &str, root: &Path, relative: &str, line: Option<u32>) {
    let path = match fsutil::resolve_within(root, relative) {
        Ok(path) => path,
        Err(err) => {
            tracing::warn!("not opening {relative} from {}: {err}", root.display());
            return;
        }
    };
    let Some(launch) = open_file::launch(bundle, &path, line) else { return };
    let cmd = command(bundle, launch);
    if let Err(err) = crate::workspace::spawn::spawn_detached(cmd) {
        tracing::warn!("opening {} in {bundle}: {err}", path.display());
    }
}

/// The process for a [`Launch`]: `open` aimed at the app, or the command line the app brings.
fn command(bundle: &str, launch: Launch) -> std::process::Command {
    let open = || std::process::Command::new("open");
    let in_app = |what: &dyn AsRef<std::ffi::OsStr>| {
        let mut cmd = open();
        cmd.arg("-b").arg(bundle).arg(what.as_ref());
        cmd
    };
    match launch {
        Launch::Open { path } => in_app(&path),
        Launch::Url { url } => in_app(&url),
        Launch::Reveal { path } => {
            let mut cmd = open();
            cmd.arg("-R").arg(path);
            cmd
        }
        Launch::Args { args } => {
            let mut cmd = open();
            cmd.arg("-n").arg("-b").arg(bundle).arg("--args").args(args);
            cmd
        }
        Launch::Bundled { relative, arg } => {
            match open_in::app_path(bundle).map(|app| app.join(relative)).filter(|exe| exe.is_file()) {
                Some(exe) => {
                    let mut cmd = std::process::Command::new(exe);
                    cmd.arg(arg);
                    cmd
                }
                // The app doesn't have it (an older version): the file, without the line.
                None => {
                    let path = arg.rsplit_once(':').map_or(arg.as_str(), |(path, _)| path).to_string();
                    in_app(&path)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{command, open};
    use wormhole_core::open_file::Launch;

    fn words(cmd: &std::process::Command) -> Vec<String> {
        std::iter::once(cmd.get_program())
            .chain(cmd.get_args())
            .map(|word| word.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn each_launch_is_aimed_at_the_chosen_app() {
        let path = PathBuf::from("/repo/a.rs");
        assert_eq!(
            words(&command("dev.zed.Zed", Launch::Open { path: path.clone() })),
            ["open", "-b", "dev.zed.Zed", "/repo/a.rs"]
        );
        assert_eq!(
            words(&command("com.microsoft.VSCode", Launch::Url { url: "vscode://file/repo/a.rs:3".into() })),
            ["open", "-b", "com.microsoft.VSCode", "vscode://file/repo/a.rs:3"]
        );
        assert_eq!(
            words(&command("com.apple.finder", Launch::Reveal { path })),
            ["open", "-R", "/repo/a.rs"]
        );
        let args = ["--line", "3", "/repo/a.rs"].map(String::from).into();
        assert_eq!(
            words(&command("com.jetbrains.PhpStorm", Launch::Args { args })),
            ["open", "-n", "-b", "com.jetbrains.PhpStorm", "--args", "--line", "3", "/repo/a.rs"]
        );
    }

    #[test]
    fn an_app_without_its_command_line_gets_the_file_without_the_line() {
        // No such app is installed (or this isn't a Mac): the bundled command line is not found.
        let launch = Launch::Bundled { relative: "Contents/MacOS/cli", arg: "/repo/a.rs:3".into() };
        assert_eq!(
            words(&command("com.example.NotInstalled", launch)),
            ["open", "-b", "com.example.NotInstalled", "/repo/a.rs"]
        );
    }

    #[test]
    fn a_path_that_leads_out_of_the_repository_starts_nothing() {
        // `open` would fail the test by launching something on a Mac; nothing is spawned for these.
        let root = std::env::temp_dir();
        for relative in ["../x", "/etc/hosts", "oie-no-such-file.rs"] {
            open("com.example.NotInstalled", &root, relative, Some(1));
        }
    }
}
