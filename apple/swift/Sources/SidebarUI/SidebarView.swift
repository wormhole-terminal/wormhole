// The sidebar: header, search and the grouped list of terminals. It draws `SidebarListing`'s sections and
// sends actions; it never edits state.

import DesignSystem
import SwiftUI
import WorkspaceModel

public struct SidebarView: View {
    public let state: WorkspaceState
    public let send: @MainActor (WorkspaceAction) -> Void
    @Namespace private var selection

    public init(state: WorkspaceState, send: @escaping @MainActor (WorkspaceAction) -> Void) {
        self.state = state
        self.send = send
    }

    public var body: some View {
        let sections = SidebarListing.sections(for: state)
        VStack(spacing: 0) {
            SidebarHeader(send: send)
            SidebarSearchField(text: Binding(get: { state.search }, set: { send(.search($0)) }))
                .padding(.bottom, Space._2)
            list(sections)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(Palette.sidebarBackground)
    }

    private func list(_ sections: [SidebarSection]) -> some View {
        let ids = RowNavigation.visibleIDs(sections)
        return ScrollView {
            LazyVStack(alignment: .leading, spacing: Space._2) {
                if ids.isEmpty && sections.allSatisfy({ $0.title == nil }) {
                    Text(state.search.isEmpty ? "No terminals" : "No terminals match")
                        .textRole(.caption).foregroundStyle(Palette.textSecondary)
                        .frame(maxWidth: .infinity).padding(.top, Space._6)
                        .motionTransition(.opacity)
                }
                ForEach(sections) { SidebarSectionView(section: $0, selected: state.selected, selection: selection, send: send) }
            }
            .padding(.horizontal, Space._2)
        }
        // Arrow keys move the selection itself, as in a native source list; the selected row is the focus mark.
        .focusable()
        .focusEffectDisabled()
        .onKeyPress(.upArrow) { move(-1, in: ids) }
        .onKeyPress(.downArrow) { move(1, in: ids) }
        .onKeyPress(.return) {
            guard let id = state.selected else { return .ignored }
            send(.select(id))
            return .handled
        }
        .motion(Motion.layout, value: ids.isEmpty)
        .accessibilityLabel("Terminals")
    }

    private func move(_ step: Int, in ids: [TerminalID]) -> KeyPress.Result {
        guard let id = RowNavigation.neighbor(of: state.selected, in: ids, step: step) else { return .ignored }
        send(.select(id))
        return .handled
    }
}

#Preview("Sidebar") {
    SidebarView(
        state: WorkspaceState(
            terminals: [
                TerminalSummary(id: 1, title: "wormhole", subtitle: "~/Projects/wormhole", branch: "main", activity: .working, isPinned: true),
                TerminalSummary(id: 2, title: "api", subtitle: "cargo test", activity: .needsInput, unread: 3),
                TerminalSummary(id: 3, title: "docs", subtitle: "~/Projects/docs", activity: .idle),
            ],
            selected: 1),
        send: { _ in }
    )
    .frame(width: 260, height: 420)
}
