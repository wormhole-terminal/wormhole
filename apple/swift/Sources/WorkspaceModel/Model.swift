// What the window shows, as plain values. No UI framework, no I/O: the Rust core's workspace fills it
// (`WorkspaceBridge` maps its snapshots), and `MockWorkspace` does for previews and tests. Every UI module reads
// only this and sends `WorkspaceAction`s; none of them decides an order, a number or a word.

import Foundation

public typealias TerminalID = Int
/// A group's id is core's: a number that is stable while the group exists.
public typealias GroupID = UInt32

/// What a terminal is doing, in the words of the sidebar's status dot (core's `RowState`).
public enum Activity: Sendable, Equatable {
    case idle
    case working
    case needsInput
    case finished
    case failed
    /// The shell ended; its screen stays readable.
    case exited
}

/// The right panel's tabs.
public enum PanelTab: String, Sendable, CaseIterable {
    case notes, changes, history, usage, project, files
}

/// What is laid over the window, at most one at a time.
public enum Overlay: Sendable, Equatable {
    case palette, shortcuts
}

/// The whole window's state: the sidebar as the core listed it, and the window's own switches.
public struct WorkspaceState: Sendable, Equatable {
    /// The list top to bottom (a search has already cut it down).
    public var sections: [SidebarSection]
    public var selected: TerminalID?
    /// Every terminal that exists, listed or folded away: what keeps a terminal's view alive.
    public var live: [TerminalID]
    public var sidebarVisible: Bool
    public var panelVisible: Bool
    public var panelTab: PanelTab
    public var search: String
    public var overlay: Overlay?
    /// Every terminal, folded ones too, in list order (core `TerminalsSnapshot`): for the palette, the status bar and menus.
    public var all: [TerminalSummary]
    public var counts: TerminalCounts
    /// The columns, tabs and layouts as the core fitted them; nil while the backend has no window width (the sample).
    public var layout: ChromeLayout?
    /// The selected terminal's title row (core `TitleSnapshot`); nil in the sample.
    public var title: TitleState?

    /// The terminals the list shows, in list order. Those folded away with their group are not here.
    public var terminals: [TerminalSummary] { sections.flatMap(\.rows).map(\.terminal) }
    public var groups: [GroupHead] { sections.compactMap(\.head) }

    public init(
        sections: [SidebarSection] = [], selected: TerminalID? = nil, live: [TerminalID]? = nil,
        sidebarVisible: Bool = true, panelVisible: Bool = true, panelTab: PanelTab = .changes, search: String = "",
        overlay: Overlay? = nil, all: [TerminalSummary]? = nil, counts: TerminalCounts = TerminalCounts(),
        layout: ChromeLayout? = nil, title: TitleState? = nil
    ) {
        self.all = all ?? sections.flatMap(\.rows).map(\.terminal)
        self.counts = counts
        self.layout = layout
        self.title = title
        self.sections = sections
        self.selected = selected
        self.live = live ?? sections.flatMap(\.rows).map(\.id)
        self.sidebarVisible = sidebarVisible
        self.panelVisible = panelVisible
        self.panelTab = panelTab
        self.search = search
        self.overlay = overlay
    }
}
