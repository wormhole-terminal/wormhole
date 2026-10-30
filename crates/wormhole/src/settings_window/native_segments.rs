//! Native segmented controls with GPUI keyboard and accessibility proxies.
use gpui_kit::assets::IconName;
use gpui_kit::*;
use std::rc::Rc;
use wormhole_text::text;

use super::{SettingsWindow, registry::Segment};
use crate::platform::controls::{SegmentOption, Segments};
use crate::settings::SettingsStore;
use crate::style::native_segments::NativeSegments;

pub(super) type Change = crate::style::native_segments::Change;
pub(super) type Hover = crate::style::native_segments::Hover;

pub(super) fn begin(native: &Rc<Segments>) -> AnyElement {
    let native = native.clone();
    canvas(move |_, _, _| native.begin(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}
pub(super) fn finish(native: &Rc<Segments>) -> AnyElement {
    let native = native.clone();
    canvas(move |_, _, _| native.finish(), |_, _, _, _| {}).absolute().size_0().into_any_element()
}

fn symbol(icon: IconName) -> Option<&'static str> {
    match icon {
        IconName::Moon => Some("moon"),
        IconName::Sun => Some("sun.max"),
        IconName::Monitor => Some("display"),
        IconName::Columns2 => Some("rectangle.split.2x1"),
        IconName::Rows2 => Some("rectangle.split.1x2"),
        _ => None,
    }
}

pub(super) fn control(
    owner: &SettingsWindow,
    native: Rc<Segments>,
    id: impl Into<SharedString>,
    options: &'static [Segment],
    selected: usize,
    enabled: bool,
    cx: &mut Context<SettingsWindow>,
    change: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let options: Vec<_> = options
        .iter()
        .map(|option| SegmentOption {
            label: text(option.label).to_string(),
            symbol: option.icon.and_then(symbol),
        })
        .collect();
    control_options(owner, native, id, options, selected, enabled, None, cx, change)
}

pub(super) fn control_options(
    owner: &SettingsWindow,
    native: Rc<Segments>,
    id: impl Into<SharedString>,
    options: Vec<SegmentOption>,
    selected: usize,
    enabled: bool,
    hover: Option<Hover>,
    cx: &mut Context<SettingsWindow>,
    change: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let id = id.into();
    let metrics = native.metrics(&options);
    let change: Change = Rc::new(change);
    owner.native_segment_handlers.borrow_mut().insert(id.to_string(), change.clone());
    let focus: Vec<_> = (0..options.len())
        .map(|ix| {
            owner
                .native_segment_focus
                .borrow_mut()
                .entry((id.to_string(), ix))
                .or_insert_with(|| cx.focus_handle())
                .clone()
        })
        .collect();
    let appearance = cx.global::<SettingsStore>().tokens.appearance;
    NativeSegments {
        native,
        id,
        options,
        accessibility_labels: None,
        selected,
        enabled,
        hover,
        focus,
        appearance,
        metrics,
        change,
    }
    .into_any_element()
}
