// What the window is told of its own state (key, on screen): the workspace's rule, and the panel's git reads, which
// wait for a window nobody sees (GPUI: `window/repo/state.rs` `Sight`).

extension WindowHub {
    /// The window is in front or behind, and on screen or not (not minimised, not covered).
    public func windowState(active: Bool, seen: Bool) {
        store.windowState(active: active, seen: seen)
        panel.windowSeen(seen)
    }
}
