#!/usr/bin/env bash
# What the right panel shows: a branch `ui-unify` two commits
# ahead of `main`, a working tree with long paths, an untracked file, a file stored with Git LFS
# and a binary one, and a `.wormhole/` project with actions, prompts and a terminal layout
# (not trusted, so the Project tab asks).
#   tools/visual/fixtures/repo-panel.sh <directory to create>
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

write README.md <<'EOF'
# terminal

A terminal for working with coding agents.
EOF
write .gitattributes <<'EOF'
*.mov filter=lfs diff=lfs merge=lfs -text
EOF
write crates/wormhole/src/style.rs <<'EOF'
pub fn icon_tab() {}
pub fn band() {}
EOF
write crates/wormhole/src/workspace/sidebar.rs <<'EOF'
pub fn render_sidebar() {}
EOF
write crates/wormhole/src/adapters/session/session_store_adapter.rs <<'EOF'
pub struct Adapter;
EOF
write assets/demo.mov <<'EOF'
version https://git-lfs.github.com/spec/v1
oid sha256:4d7a214614ab2935c943f9e0ff69f22eadbb8f32b1258daaa5e2ca24d17e2393
size 12345678
EOF
printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01' > "$repo/assets/logo.png"
commit 9 "Start the terminal"

write crates/wormhole/src/workspace/tasks.rs <<'EOF'
pub fn render_tasks() {}
EOF
commit 6 "Add next steps to the notes tab"

git -C "$repo" checkout -q -b ui-unify
write crates/wormhole/src/studio/gallery.rs <<'EOF'
pub fn render_gallery() {}
EOF
write crates/wormhole/src/workspace/tasks.rs <<'EOF'
pub fn render_tasks() {}
pub fn render_task() {}
EOF
commit 3 "feat(ui): unify design system across sidebar and panels"
write docs/themes.md <<'EOF'
# Themes

Every color is a token.
EOF
commit 2 "docs: the theme tokens"

# The project folder, committed like a team would.
write .wormhole/actions.json <<'EOF'
[
  { "id": "test", "title": "Test", "key": "ctrl-cmd-t", "type": "run", "command": "cargo test --workspace" },
  { "id": "lint", "title": "Lint", "type": "run", "command": "cargo clippy --workspace -- -D warnings" },
  { "id": "bundle", "title": "Bundle", "type": "run", "command": "scripts/bundle.sh" }
]
EOF
write .wormhole/prompts/review.md <<'EOF'
---
title: Review the diff
description: Point out bugs, then simplifications
---
Review the uncommitted changes. Point out bugs first, then simplifications.
EOF
write .wormhole/prompts/release.md <<'EOF'
---
title: Write the release notes
description: From the commits since the last tag
---
Write release notes from the commits since the last tag.
EOF
write .wormhole/layout.json <<'EOF'
{ "sessions": [
  { "name": "claude", "run": "claude", "focus": true },
  { "name": "cargo watch", "run": "cargo watch -x check" },
  { "name": "zsh" }
] }
EOF
commit 1 "Add the project folder"

# The working tree: edits, a long-named file, an untracked one, an LFS object and a binary.
write crates/wormhole/src/style.rs <<'EOF'
pub fn icon_tab() {}
pub fn band() {}
pub fn item_row() {}
pub fn file_row() {}
EOF
write crates/wormhole/src/workspace/sidebar.rs <<'EOF'
pub fn render_sidebar() {}
pub fn render_band() {}
EOF
write crates/wormhole/src/adapters/session/session_store_adapter.rs <<'EOF'
pub struct Adapter;
impl Adapter {
    pub fn new() -> Self { Self }
}
EOF
write crates/wormhole/src/style/chip.rs <<'EOF'
pub fn chip() {}
EOF
write tests/snapshots/integration__nightly_suite_times_out_on_arm64.snap <<'EOF'
---
source: tests/integration.rs
---
timed out after 600 s
EOF
write assets/demo.mov <<'EOF'
version https://git-lfs.github.com/spec/v1
oid sha256:9a8c3f1b47e6d2058c1f4b9e7a3d6c5b8e2f1a0d9c8b7a6f5e4d3c2b1a0f9e8d
size 12812345
EOF
printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x02' > "$repo/assets/logo.png"
