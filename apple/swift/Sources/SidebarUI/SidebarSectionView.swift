// A run of rows. A titled section (a group) has a heading that folds it; the ungrouped run has none.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SidebarSectionView: View {
    let section: SidebarSection
    let selected: TerminalID?
    let selection: Namespace.ID
    let send: @MainActor (WorkspaceAction) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let title = section.title {
                Button { send(.toggleGroup(section.id)) } label: {
                    SectionLabel(title, isCollapsed: section.isCollapsed).padding(.horizontal, Space._2)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(title)
                .accessibilityValue(section.isCollapsed ? "collapsed" : "expanded")
                .accessibilityHint("Folds or unfolds the group")
            }
            if !section.isCollapsed {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(section.rows) { row in
                        TerminalRowView(row: row, isSelected: row.id == selected, selection: selection, send: send)
                            .motionTransition(.opacity.combined(with: .scale(scale: 0.96, anchor: .top)))
                    }
                }
                .motionTransition(.opacity.combined(with: .move(edge: .top)))
            }
        }
        // Rows arriving, leaving or changing order, and the group folding, share one smooth spring.
        .motion(Motion.layout, value: section.rows.map(\.id))
        .motion(Motion.layout, value: section.isCollapsed)
        .clipped()
    }
}
