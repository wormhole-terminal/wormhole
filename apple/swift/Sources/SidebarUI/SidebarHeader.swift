// The sidebar's own heading and its new-terminal button. Collapsing the sidebar is the window title row's job.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SidebarHeader: View {
    let send: @MainActor (WorkspaceAction) -> Void

    var body: some View {
        HStack(spacing: Space._2) {
            Text("Terminals").textRole(.heading).accessibilityAddTraits(.isHeader)
            Spacer(minLength: 0)
            IconButton(symbol: "plus", label: "New terminal") { send(.newTerminal) }
        }
        .padding(.leading, Metric.edgeInset)
        .padding(.trailing, Space._2)
        .frame(minHeight: Metric.control + Space._2)
    }
}
