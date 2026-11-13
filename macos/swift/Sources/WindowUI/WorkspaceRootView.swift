// The window's content: sidebar | terminal | right panel over the window's blur, the optional status bar under
// them, the two toggles floating over the title rows and the overlays over everything. The columns collapse by
// clip, not by removal, so their content keeps its layout while it slides and a running terminal view is never
// rebuilt (`SlideGeometry`, `PaneFrames`). The widths live here and only in memory: hiding a column and showing
// it again brings back the width it had.

import DesignSystem
import PaletteUI
import PanelUI
import SwiftUI
import WorkspaceModel

public struct WorkspaceRootView: View {
    let hub: WindowHub
    let dress: WindowDress
    @State private var widths = PaneLayout()
    @State private var facts = WindowFacts()
    @State private var sidebarSlide: ColumnSlide
    @State private var panelSlide: ColumnSlide
    @State private var drag: DragState?
    @Environment(\.reduceMotion) private var reduceMotion

    /// A column being dragged: which, the pointer, what the readout says ("430 · max", "Close"), whether the column stays
    /// open at this pointer and how far the pointer is from the window's edge it hangs on.
    struct DragState: Equatable {
        var edge: PaneLayout.Edge
        var pointer: CGFloat
        /// Where the pointer is down the window: the readout stands beside it.
        var pointerY: CGFloat = 0
        var readout: String
        var open: Bool
        var extent: CGFloat
    }

    public init(hub: WindowHub, look: WindowLook = WindowLook()) { self.init(hub: hub, dress: WindowDress(look)) }

    /// A window whose look can change under it (it follows the profile the window is in).
    public init(hub: WindowHub, dress: WindowDress) {
        self.hub = hub
        self.dress = dress
        _sidebarSlide = State(initialValue: ColumnSlide(open: hub.state.sidebarVisible))
        _panelSlide = State(initialValue: ColumnSlide(open: hub.state.panelVisible))
    }

    public var body: some View {
        let look = dress.look
        WindowBody(
            hub: hub, widths: $widths, facts: facts,
            sidebarSlide: sidebarSlide, panelSlide: panelSlide, drag: $drag, animates: !reduceMotion
        )
        .background(WindowBackdrop())
        .background(WindowReader(facts: $facts))
        .background(FocusHolder(focus: hub.focus))
        // A new window starts with the keys in its terminal, as the GPUI app's does (SwiftUI would give them to the first list).
        .task { hub.focus.focusTerminal() }
        // The project commands window approves through a desk of its own: coming back to the window shows what it did.
        .onReceive(NotificationCenter.default.publisher(for: NSWindow.didBecomeKeyNotification)) { _ in hub.panel.project.reload() }
        // The panel's tabs follow the selected terminal's folder whether or not the panel is on screen: the worktree sheet and the
        // diff in the terminal's place need it with the panel shut.
        .onChange(of: hub.state.selectedTerminal?.folder, initial: true) { _, _ in followTerminal() }
        // What a tab keeps per terminal (compare branch, comments, open commit, tree) changes hands with the selection.
        .onChange(of: hub.state.selected) { _, _ in followTerminal() }
        // Nothing of a folder is listed or watched while the Files tab is hidden or its feature is off.
        .onChange(of: filesShown, initial: true) { _, now in hub.panel.files.setShown(now) }
        // ⇥ and Esc from the lists and fields of SwiftUI; the holder and the terminal take their own (`WindowFocus`).
        .onKeyPress(keys: [.tab, .escape], phases: .down) { hub.focus.listKey($0) }
        .coordinateSpace(name: PaneSpace.name)
        .environment(look)
        .environment(\.codeFont, look.codeFont)
        .environment(\.tabWidth, hub.panel.preferences.tabWidth)
        .environment(hub.focus)
        .environment(\.windowFront, hub.focus.front)
        .environment(\.cardGlass, look.glassOn)
        // The window's own profile's theme, which is the app's one store for a window of the profile Wormhole is in.
        .themed(look.themes)
        .worktreeSheets(hub.panel.worktrees)
        .ignoresSafeArea()
    }

    private var filesShown: Bool {
        hub.state.panelVisible && hub.state.panelTab == .files && hub.panelTabs.contains(.files)
    }

    private func followTerminal() {
        hub.panel.follow(folder: Self.expanded(hub.state.selectedTerminal?.folder), terminal: hub.state.selected)
    }

    /// The sidebar's folder is written with `~`; the panel reads real paths.
    static func expanded(_ folder: String?) -> String? {
        guard let folder else { return nil }
        return folder == "~" ? NSHomeDirectory() : folder.hasPrefix("~/") ? NSHomeDirectory() + folder.dropFirst(1) : folder
    }
}
