//! Shared lifecycle for AppKit controls hosted in a GPUI window.
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use gpui_kit::{App, Bounds, ForegroundExecutor, Pixels, Window};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSAppearanceCustomization as _, NSView};
use objc2_foundation::NSRect;
use wormhole_core::theme::tokens::Appearance;

use super::{appearance, appkit};
use crate::platform::controls::{generation_visible, visible};

struct Entry<T> {
    container: Retained<NSView>,
    value: T,
    generation: u64,
}

/// Each value owns the control and its action target. The clipping view is common to every kind.
pub(super) struct NativeViews<T: 'static> {
    pub mtm: MainThreadMarker,
    parent: Retained<NSView>,
    entries: RefCell<HashMap<String, Entry<T>>>,
    generation: Cell<u64>,
    suspended: Cell<bool>,
    executor: ForegroundExecutor,
    window_id: gpui_kit::WindowId,
}

impl<T: 'static> NativeViews<T> {
    pub fn new(window: &Window, cx: &App) -> Option<Self> {
        if crate::platform::background_mode::on() {
            return None;
        }
        let mtm = MainThreadMarker::new()?;
        let window_id = window.window_handle().window_id();
        let parent = appkit::ns_view(window)?;
        crate::style::native_focus::register(window_id);
        Some(Self {
            mtm,
            parent,
            entries: RefCell::new(HashMap::new()),
            generation: Cell::new(0),
            suspended: Cell::new(false),
            executor: cx.foreground_executor().clone(),
            window_id,
        })
    }

    pub fn begin(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
    }

    pub fn place(
        &self,
        id: &str,
        bounds: Bounds<Pixels>,
        clip: Bounds<Pixels>,
        appearance: Appearance,
        create: impl FnOnce(&NSView) -> T,
        update: impl FnOnce(&mut T, &NSView, NSRect),
    ) {
        let Some(clipped) = visible(bounds, clip) else { return };
        let mut entries = self.entries.borrow_mut();
        let entry = entries.entry(id.to_owned()).or_insert_with(|| {
            let container = NSView::initWithFrame(self.mtm.alloc(), NSRect::ZERO);
            container.setClipsToBounds(true);
            let value = create(&container);
            self.parent.addSubview(&container);
            Entry { container, value, generation: 0 }
        });
        entry.generation = self.generation.get();
        entry.container.setAppearance(appearance::ns_appearance(appearance).as_deref());
        let full = appkit::rect(&self.parent, bounds);
        entry.container.setFrame(appkit::rect(&self.parent, clipped));
        let local = entry.container.convertRect_fromView(full, Some(&self.parent));
        update(&mut entry.value, &entry.container, local);
        entry.container.setHidden(self.suspended.get());
    }

    pub fn finish(&self) {
        for entry in self.entries.borrow().values() {
            entry.container.setHidden(!generation_visible(
                self.suspended.get(),
                entry.generation,
                self.generation.get(),
            ));
        }
    }

    /// Temporarily cover a cached surface without discarding its current geometry. Only views
    /// placed in the most recent generation may reappear when the cover is removed.
    pub fn set_suspended(&self, suspended: bool) {
        if self.suspended.replace(suspended) == suspended {
            return;
        }
        for entry in self.entries.borrow().values() {
            entry.container.setHidden(!generation_visible(
                suspended,
                entry.generation,
                self.generation.get(),
            ));
        }
    }

    pub fn accepts(&self, id: &str, enabled: impl FnOnce(&T) -> bool) -> bool {
        self.entries.borrow().get(id).is_some_and(|entry| {
            entry.generation == self.generation.get() && !entry.container.isHidden() && enabled(&entry.value)
        })
    }

    pub fn forget_prefix(&self, prefix: &str) {
        self.forget_prefix_except(prefix, None);
    }

    /// Release controls from earlier transient owners while retaining the active owner's views.
    pub fn forget_prefix_except(&self, prefix: &str, keep: Option<&str>) {
        let mut entries = self.entries.borrow_mut();
        let ids: Vec<_> = entries
            .keys()
            .filter(|id| id.starts_with(prefix) && !keep.is_some_and(|keep| id.starts_with(keep)))
            .cloned()
            .collect();
        let removed: Vec<_> = ids.into_iter().filter_map(|id| entries.remove(&id)).collect();
        drop(entries);
        if removed.is_empty() {
            return;
        }
        // Detaching is deferred, but the old controls must stop drawing and taking input now.
        for entry in &removed {
            entry.container.setHidden(true);
        }
        self.executor
            .spawn(async move {
                for entry in removed {
                    entry.container.removeFromSuperview();
                }
            })
            .detach();
    }
}

impl<T: 'static> Drop for NativeViews<T> {
    fn drop(&mut self) {
        crate::style::native_focus::forget(self.window_id);
        let entries = std::mem::take(self.entries.get_mut());
        // Defer AppKit notifications until GPUI has finished releasing the owner.
        self.executor
            .spawn(async move {
                for entry in entries.into_values() {
                    entry.container.removeFromSuperview();
                }
            })
            .detach();
    }
}
