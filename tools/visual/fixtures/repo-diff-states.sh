#!/usr/bin/env bash
# Every state of the diff card, one changed file each (git lists them by path):
#   1 assets/demo.mov             stored with Git LFS, both sides pointers
#   2 assets/icon.png             binary, 48.2 KB → 52.9 KB
#   3 scripts/bundle.sh           only its mode changed
#   4 src/empty.rs                a new empty file (staged)
#   5 src/style.rs                long lines (wrap, sideways scroll)
#   6 src/style/chip.rs           a new file (staged)
#   7 src/workspace/notes.rs      renamed from panel.rs, one line changed (staged)
#   8 src/workspace/old_panel.rs  deleted
#   tools/visual/fixtures/repo-diff-states.sh <directory to create>
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

pointer() {
  printf 'version https://git-lfs.github.com/spec/v1\noid sha256:%s\nsize %s\n' "$1" "$2"
}

write .gitattributes <<'EOF'
*.mov filter=lfs diff=lfs merge=lfs -text
EOF
pointer 1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7f809 12300000 | write assets/demo.mov
mkdir -p "$repo/assets"
head -c 48200 /dev/zero > "$repo/assets/icon.png"
write scripts/bundle.sh <<'EOF'
#!/bin/sh
cargo build --release
EOF
write src/style.rs <<'EOF'
pub fn icon_tab(id: &'static str, ui: UiColors, icon: IconName, count: Option<usize>, active: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .when(count.is_some(), |d| d.px_2())
        .rounded(ui.radius_chip)
        .cursor_pointer()
        .text_color(if active { ui.on_selected } else { ui.muted })
        .when(active, |d| d.bg(ui.selected_chip))
        .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
}
EOF
write src/workspace/panel.rs <<'EOF'
impl Workspace {
    pub(super) fn render_notes_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        div()
            .flex_col()
            .child(self.render_notes(cx))
            .into_any_element()
    }
}
EOF
write src/workspace/old_panel.rs <<'EOF'
//! The old right panel: notes above, tasks below.

use super::*;

pub(super) fn render_panel(ws: &Workspace, cx: &mut Context<Workspace>) -> AnyElement {
    let ui = ui(cx);
    div()
        .flex_col()
        .gap_1p5()
        .p_3()
        .child(ws.render_notes(cx))
        .child(ws.render_tasks(cx))
        .into_any_element()
}
EOF
commit 2 "Start the panel"

# The working tree.
pointer 4d7a214614ab2935c943f9e0ff69f22eadbb8f32b1258daaa5e2ca24d17e2393 12800000 | write assets/demo.mov
head -c 52900 /dev/zero > "$repo/assets/icon.png"
chmod +x "$repo/scripts/bundle.sh"
write src/style.rs <<'EOF'
pub fn icon_tab(id: &'static str, ui: UiColors, icon: IconName, count: Option<usize>, active: bool) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .when(count.is_some(), |d| d.px_2())
        .rounded(ui.radius)
        .cursor_pointer()
        // Tinted like the current terminal's row, not filled: the one solid primary left is the primary button.
        .text_color(if active { ui.primary } else { ui.muted })
        .when(active, |d| d.bg(ui.selected_row))
        .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
}
EOF
git -C "$repo" mv src/workspace/panel.rs src/workspace/notes.rs
write src/workspace/notes.rs <<'EOF'
impl Workspace {
    pub(super) fn render_notes_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = crate::settings::ui(cx);
        div()
            .flex_col()
            .child(self.render_notes(cx))
            .into_any_element()
    }
}
EOF
rm "$repo/src/workspace/old_panel.rs"
: > "$repo/src/empty.rs"
write src/style/chip.rs <<'EOF'
//! The branch chip in a folder band.

use gpui_kit::prelude::*;
use crate::settings::UiColors;

pub fn branch_chip(ui: UiColors, branch: Option<&str>) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(20.0))
        .px_2()
        .rounded(ui.radius_chip)
        .bg(ui.branch_bg)
        .text_color(ui.branch)
        .children(branch.map(str::to_string))
}
EOF
git -C "$repo" add src/empty.rs src/style/chip.rs src/workspace/notes.rs
