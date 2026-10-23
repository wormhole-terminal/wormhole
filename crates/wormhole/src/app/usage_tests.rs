//! A usage report arriving on the hook socket reaches the Usage tab of the open windows.

use gpui_kit::TestAppContext;
use wormhole_core::ipc;

use crate::features::usage::tests::{claude, drawn, on_usage};
use crate::window::test_harness::isolate_in;

#[gpui_kit::test]
fn a_usage_message_reaches_the_tab(cx: &mut TestAppContext) {
    if isolate_in(module_path!(), "a_usage_message_reaches_the_tab") {
        return;
    }
    let window = on_usage(cx);
    assert!(!drawn(cx, &window, "usage-card-Claude"));
    cx.update(|cx| {
        super::tick::dispatch_hook(ipc::Incoming::Message(ipc::Message::Usage { usage: claude(30.0, 5) }), cx)
    });
    cx.run_until_parked();
    assert!(drawn(cx, &window, "usage-card-Claude"), "the open window shows the report");
}
