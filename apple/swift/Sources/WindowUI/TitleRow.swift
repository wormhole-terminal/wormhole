// The 38 pt rows across the top. The sidebar's row leaves room for the traffic lights and holds the
// collapse button; the centre's names the selected terminal and holds the panel toggle. Each toggle shows
// in the column that is open and falls back to the centre row when its column is hidden, at the same spot.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct SidebarTitleRow: View {
    let leadingInset: CGFloat
    let send: @MainActor (WorkspaceAction) -> Void

    var body: some View {
        HStack(spacing: 0) {
            ChromeButton(systemImage: "sidebar.left", label: "Hide sidebar") { send(.toggleSidebar) }
                .padding(.leading, leadingInset)
            Spacer(minLength: 0)
        }
        .frame(height: Chrome.titleHeight)
        .background(WindowDragArea())
    }
}

struct CentreTitleRow: View {
    let state: WorkspaceState
    let leadingInset: CGFloat
    let send: @MainActor (WorkspaceAction) -> Void
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        HStack(spacing: Space._2) {
            if !state.sidebarVisible {
                ChromeButton(systemImage: "sidebar.left", label: "Show sidebar") { send(.toggleSidebar) }
                    .transition(WindowMotion.appear(reduceMotion: reduceMotion))
            }
            if let terminal = state.selectedTerminal {
                Text(terminal.title)
                    .font(.headline)
                    .lineLimit(1)
                    .contentTransition(.interpolate)
                    .animation(WindowMotion.fade, value: terminal.title)
                if let branch = terminal.branch {
                    BranchChip(name: branch)
                        .transition(WindowMotion.appear(reduceMotion: reduceMotion))
                }
            }
            Spacer(minLength: 0)
            if !state.panelVisible {
                ChromeButton(systemImage: "sidebar.right", label: "Show panel") { send(.togglePanel) }
                    .transition(WindowMotion.appear(reduceMotion: reduceMotion))
            }
        }
        .padding(.leading, state.sidebarVisible ? Space._3 : leadingInset)
        .padding(.trailing, Space._2)
        .frame(height: Chrome.titleHeight)
        .animation(WindowMotion.column(reduceMotion: reduceMotion), value: state.sidebarVisible)
        .animation(WindowMotion.column(reduceMotion: reduceMotion), value: state.panelVisible)
        .animation(WindowMotion.fade, value: state.selectedTerminal?.branch)
        .background(WindowDragArea())
    }
}

struct BranchChip: View {
    let name: String

    var body: some View {
        Label(name, systemImage: "arrow.triangle.branch")
            .font(.caption)
            .foregroundStyle(Palette.textSecondary)
            .lineLimit(1)
            .padding(.horizontal, Space._2)
            .padding(.vertical, Space._1)
            .background(Palette.hover, in: RoundedRectangle(cornerRadius: Radius.chip))
            .contentTransition(.interpolate)
    }
}
