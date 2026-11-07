// The seam between the window's state and whatever keeps it: the Rust workspace through the FFI (`WorkspaceBridge`) or
// the in-memory sample (`MockWorkspaceBackend`). A backend answers in this module's values; the store pulls a snapshot
// only when its revision moved, so a window with nothing happening does nothing.

import Foundation

/// What a backend counts: a part's snapshot changed when its number did.
public struct Revisions: Sendable, Equatable {
    public var sidebar: UInt64
    public var chrome: UInt64

    public init(sidebar: UInt64, chrome: UInt64) {
        self.sidebar = sidebar
        self.chrome = chrome
    }
}

/// The window's own switches (core `ChromeSnapshot`).
public struct ChromeState: Sendable, Equatable {
    public var sidebarVisible: Bool
    public var panelVisible: Bool
    public var panelTab: PanelTab
    public var search: String
    public var overlay: Overlay?

    public init(
        sidebarVisible: Bool = true, panelVisible: Bool = true, panelTab: PanelTab = .changes, search: String = "",
        overlay: Overlay? = nil
    ) {
        self.sidebarVisible = sidebarVisible
        self.panelVisible = panelVisible
        self.panelTab = panelTab
        self.search = search
        self.overlay = overlay
    }
}

/// Why an action changed nothing (core `Refusal`): data, not an error.
public enum Refusal: String, Sendable, Equatable {
    case unknown, tooSmall, unchanged
}

@MainActor
public protocol WorkspaceBackend: AnyObject {
    /// Set by the store: called on the main actor, once, when something changed away from a call of the store's
    /// (a terminal's title, folder or bell). The store answers with `drain()`.
    var onWake: (@MainActor () -> Void)? { get set }
    /// Every terminal that exists, in the order they were made.
    var live: [TerminalID] { get }
    /// Do what the person did; why it changed nothing, if it did not.
    func apply(_ action: WorkspaceAction) -> Refusal?
    /// Take what other threads caused since the last drain; this re-arms the wake.
    func drain()
    func revisions() -> Revisions
    func sidebar() -> SidebarSnapshot
    func chrome() -> ChromeState
    /// What dropping `dragged` on `over` would do; nil when nothing would change. Pure: nothing moves.
    func dropPlan(dragged: TerminalID, over: DropOver) -> DropPlan?
}
