// The window's content: sidebar | centre | right panel. The columns collapse by width, not by removal, so
// their content keeps its layout while it slides and a running terminal view is never rebuilt. The widths
// live here and only in memory: hiding a column and showing it again brings back the width it had.

import DesignSystem
import SidebarUI
import SwiftUI
import WorkspaceModel

public struct WorkspaceRootView: View {
    let state: WorkspaceState
    let send: @MainActor (WorkspaceAction) -> Void
    let terminals: TerminalViews
    @State private var sidebarWidth = Chrome.sidebarWidth
    @State private var panelWidth = Chrome.panelWidth
    @State private var isFullScreen = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(
        state: WorkspaceState, send: @escaping @MainActor (WorkspaceAction) -> Void, terminals: TerminalViews
    ) {
        self.state = state
        self.send = send
        self.terminals = terminals
    }

    public var body: some View {
        GeometryReader { proxy in
            let widths = ColumnWidths(
                sidebar: sidebarWidth, panel: panelWidth, window: proxy.size.width,
                sidebarVisible: state.sidebarVisible, panelVisible: state.panelVisible)
            HStack(spacing: 0) {
                column(width: widths.sidebar, visible: state.sidebarVisible) {
                    VStack(spacing: 0) {
                        SidebarTitleRow(leadingInset: collapseInset, send: send)
                        SidebarView(state: state, send: send)
                    }
                    .background(Palette.sidebarBackground)
                }
                if state.sidebarVisible {
                    ColumnDivider(
                        width: $sidebarWidth, range: Chrome.sidebarRange, grows: 1, label: "Resize sidebar",
                        initial: Chrome.sidebarWidth)
                }
                centre
                if state.panelVisible {
                    ColumnDivider(
                        width: $panelWidth, range: Chrome.panelRange, grows: -1, label: "Resize panel",
                        initial: Chrome.panelWidth)
                }
                column(width: widths.panel, visible: state.panelVisible) {
                    RightPanel(tab: state.panelTab, send: send)
                }
            }
        }
        .ignoresSafeArea()
        .background(Chrome.surface)
        .background(FullScreenReader(isFullScreen: $isFullScreen))
    }

    private var collapseInset: CGFloat { isFullScreen ? Chrome.collapseXFullScreen : Chrome.collapseX }

    private var centre: some View {
        VStack(spacing: 0) {
            CentreTitleRow(state: state, leadingInset: collapseInset, send: send)
            TerminalHost(views: terminals, selected: state.selected, ids: state.terminals.map(\.id))
                .background(Chrome.terminalSurface)
            StatusBar(terminal: state.selectedTerminal)
        }
        .frame(minWidth: Chrome.terminalMin, maxWidth: .infinity)
        .background(Chrome.surface)
    }

    /// A column that keeps its layout width and is clipped to nothing when hidden, out of reach of keys and
    /// VoiceOver. The width follows the spring; the content only fades, which is all Reduce Motion keeps.
    private func column<Content: View>(
        width: CGFloat, visible: Bool, @ViewBuilder content: () -> Content
    ) -> some View {
        content()
            .frame(width: width)
            .opacity(visible ? 1 : 0)
            .animation(WindowMotion.fade, value: visible)
            .frame(width: visible ? width : 0, alignment: .leading)
            .animation(WindowMotion.column(reduceMotion: reduceMotion), value: visible)
            .clipped()
            .allowsHitTesting(visible)
            .accessibilityHidden(!visible)
    }
}
