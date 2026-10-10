---
paths:
  - "crates/wormhole/**/*.rs"
---

# GPUI in this crate

Before writing or changing GPUI code here, invoke the `gpui` skill. GPUI is the
locked `gpui-pre` through `gpui-kit`, and its API differs from older GPUI and from
Zed's `main`:

- No `ViewContext`, `ModelContext`, `WindowContext`, `Model<T>` or `View<T>`.
  Use `Context<T>`, `&mut Window`, `Entity<T>` and `cx.new(…)`.
- `Render::render` is `render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement`.
- `cx.spawn` takes `async move |this, cx| …`. `AsyncApp::update` returns `R`, not
  `Result`.
- `window.focus(&handle, cx)` takes `cx`.
- Paths are `gpui_kit::…`, never `gpui::…`; tests use `#[gpui_kit::test]`.

Type-check new GPUI calls on their own, and fix them until this passes:
`.claude/skills/gpui/scripts/gpui-docs check - <<'EOF' … EOF`.
