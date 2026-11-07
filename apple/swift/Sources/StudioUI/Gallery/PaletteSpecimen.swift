// The command palette and the shortcut sheet, on their scrim. Both take the keyboard the moment they appear (a
// palette is a modal question), so the page holds them back until asked: showing them here must not take the
// focus from the field being typed in.

import DesignSystem
import PaletteUI
import SwiftUI
import WorkspaceModel

struct PaletteSpecimen: View {
    enum Shown { case none, palette, sheet }
    @State var shown = Shown.none
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._2) {
                Button("Show the palette") { shown = shown == .palette ? .none : .palette }
                Button("Show the shortcut sheet") { shown = shown == .sheet ? .none : .sheet }
                Spacer(minLength: 0)
            }
            .buttonStyle(.bordered).controlSize(.small)
            switch shown {
            case .none: EmptyView()
            case .palette: PaletteView(state: MockWorkspace.state, send: { _ in }, dismiss: { shown = .none }).stage
            case .sheet: ShortcutSheetView(dismiss: { shown = .none }).stage
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private extension View {
    /// A window-sized stage for an overlay that fills what it is given.
    var stage: some View {
        frame(height: 480).clipShape(RoundedRectangle(cornerRadius: 10)).transition(.opacity)
    }
}
