// The sidebar: header, search and the grouped list of terminals. First cut: the sidebar agent fills it in.

import DesignSystem
import SwiftUI
import WorkspaceModel

public struct SidebarView: View {
    public let state: WorkspaceState
    public let send: @MainActor (WorkspaceAction) -> Void

    public init(state: WorkspaceState, send: @escaping @MainActor (WorkspaceAction) -> Void) {
        self.state = state
        self.send = send
    }

    public var body: some View {
        Text("Sidebar").frame(maxWidth: .infinity, maxHeight: .infinity).background(Palette.sidebarBackground)
    }
}
