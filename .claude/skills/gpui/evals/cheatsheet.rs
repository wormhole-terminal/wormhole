// Every line of the gpui skill's cheat sheet and "API you may remember that is gone here", as code
// that has to compile against the GPUI in Cargo.lock. `gpui-docs selftest` checks it: run it after a
// gpui-kit bump, and update SKILL.md and this file together until it passes again.

use std::ops::Range;
use std::time::Duration;

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;

actions!(workspace, [NewTab, CloseTab]);
actions!(wormhole, [Quit]);

enum PanelEvent {
    Closed,
}

#[derive(Default)]
struct Store {
    n: usize,
}
impl Global for Store {}

struct Panel {
    data: Vec<String>,
    items: Vec<SharedString>,
    load: Option<Task<()>>,
    focus_handle: FocusHandle,
    scroll: UniformListScrollHandle,
    state: ListState,
    input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PanelEvent> for Panel {}

impl Focusable for Panel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn parse(raw: String) -> Vec<String> {
    raw.lines().map(str::to_owned).collect()
}

impl Panel {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx));
        let other = cx.new(|_| Store::default());
        let subscriptions = vec![
            cx.subscribe(&input, |this, _input, ev: &InputEvent, cx| {}),
            cx.subscribe_in(&input, window, |this, _input, ev: &InputEvent, window, cx| {}),
            cx.observe(&other, |this, other, cx| {}),
            cx.observe_global::<Store>(|this, cx| cx.notify()),
        ];
        Self {
            data: Vec::new(),
            items: Vec::new(),
            load: None,
            focus_handle: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
            state: ListState::new(0, ListAlignment::Top, px(300.)),
            input,
            _subscriptions: subscriptions,
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {}
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {}
    fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {}
    fn row(&self, ix: usize) -> Div {
        div()
    }

    fn cheats(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = String::new();
        self.load = Some(cx.spawn(async move |this, cx| {
            let data = cx.background_spawn(async move { parse(raw) }).await;
            cx.background_executor().timer(Duration::from_millis(30)).await;
            this.update(cx, |this, cx| {
                this.data = data;
                cx.notify();
            })
            .ok();
            let n: usize = cx.update(|cx| cx.global::<Store>().n); // AsyncApp::update -> R
        }));
        cx.spawn_in(window, async move |this, cx| {
            this.update_in(cx, |this, window, cx| this.refresh(window, cx)).ok();
            let r: Result<()> = cx.update(|window, cx| {}); // AsyncWindowContext::update -> Result
        })
        .detach();

        cx.emit(PanelEvent::Closed);
        cx.set_global(Store::default());
        let n = cx.global::<Store>().n;
        cx.global_mut::<Store>().n += 1;
        cx.update_global::<Store, _>(|store, cx| store.n += 1);

        window.focus(&self.focus_handle, cx);
        self.focus_handle.focus(window, cx);
        let focused = self.focus_handle.is_focused(window);

        cx.defer(|cx| {});
        cx.defer_in(window, |this, window, cx| {});
        cx.on_next_frame(window, |this, window, cx| {});

        let open = window.use_keyed_state(("menu", 7usize), cx, |_window, _cx| false);
        let units = (px(12.), rems(1.), relative(0.5), SharedString::from("label"));
        self.scroll.scroll_to_item(5, ScrollStrategy::Center);
    }
}

impl Render for Panel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let id: u64 = 3;
        div()
            .key_context("Workspace")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &NewTab, window, cx| this.new_tab(window, cx)))
            .child(
                div()
                    .id("save")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.save(window, cx))),
            )
            .child(div().id(("session", id)).when(true, |el| el.p_2()).when_some(Some(1), |el, _| el.gap_1()))
            .child(div().hover(|s| s.opacity(0.5)).on_mouse_down(MouseButton::Left, |_, _, _| {}))
            .child(
                uniform_list(
                    "rows",
                    self.items.len(),
                    cx.processor(|this, range: Range<usize>, _window, _cx| {
                        range.map(|ix| this.row(ix)).collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&self.scroll),
            )
            .child(list(self.state.clone(), move |ix, _window, cx| div().into_any_element()))
            .child(Input::new(&self.input))
            .child(Chip)
    }
}

#[derive(IntoElement)]
struct Chip;

impl RenderOnce for Chip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
    }
}

fn app_setup(cx: &mut App) {
    gpui_kit::init(cx);
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.bind_keys([KeyBinding::new("cmd-t", NewTab, Some("Workspace"))]);
    let view = cx.new(|_| Store::default());
    view.update(cx, |this, cx| {
        this.n += 1;
        cx.notify();
    });
    let weak = view.downgrade();
    let updated: Result<()> = weak.update(cx, |_, _| {});
}

mod headless {
    use gpui_kit::{AppContext as _, TestAppContext};

    struct Counter {
        n: usize,
    }

    #[gpui_kit::test]
    fn counts(cx: &mut TestAppContext) {
        let c = cx.new(|_| Counter { n: 0 });
        c.update(cx, |c, cx| {
            c.n += 1;
            cx.notify();
        });
        assert_eq!(c.read_with(cx, |c, _| c.n), 1);
    }
}
