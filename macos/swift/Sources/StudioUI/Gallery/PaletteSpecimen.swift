// The command palette and the shortcut sheet, on their scrim, and the menu bar's quick panel on its card
// (`gallery/ambient.rs`), over the terminals the status item's own samples list. All take the keyboard the moment they appear (a
// palette is a modal question), so the page holds them back until asked: showing them here must not take the
// focus from the field being typed in.

import DesignSystem
import PaletteUI
import PlatformModel
import PlatformUI
import SwiftUI
import WorkspaceModel

struct PaletteSpecimen: View {
    enum Shown { case none, palette, sheet, quick }
    @State var shown = Shown.none
    /// The quick panel's state while it shows: kept, so typing in its field narrows the rows.
    @State private var quick: QuickPanelModel?
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._2) {
            HStack(spacing: Space._2) {
                Button(StudioWords.showPalette) { shown = shown == .palette ? .none : .palette }
                Button(StudioWords.showSheet) { shown = shown == .sheet ? .none : .sheet }
                Button(StudioWords.showQuickPanel) {
                    quick = shown == .quick ? nil : Self.quick { shown = .none }
                    shown = quick == nil ? .none : .quick
                }
                Spacer(minLength: 0)
            }
            .buttonStyle(.bordered).controlSize(.small)
            switch shown {
            case .none: EmptyView()
            case .palette: PaletteView(state: MockWorkspace.state, send: { _ in }, dismiss: { shown = .none }).stage
            case .sheet: ShortcutSheetView(dismiss: { shown = .none }).stage
            case .quick: if let quick { QuickPanelView(model: quick).transition(.opacity) }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    /// The quick panel over a terminal that asks, one that works, one that failed and one at rest; choosing a row only closes it.
    private static func quick(close: @escaping () -> Void) -> QuickPanelModel {
        let model = QuickPanelModel()
        let now = Date()
        model.now = now
        model.terminals = [
            AmbientTerminal(session: 4, name: "diff perf", state: .needsYou, since: now - 125, source: .agent, who: "Claude Code", message: "Approve cargo test?"), // text: sample, as the GPUI gallery's
            AmbientTerminal(session: 3, name: "tests", state: .working, since: now - 72, command: "cargo test -p wormhole-core"), // text: sample, as the GPUI gallery's
            AmbientTerminal(session: 7, name: "build", state: .failed, since: now - 120, command: "cargo build", took: 38, exit: 101), // text: sample, as the GPUI gallery's
            AmbientTerminal(session: 1, name: "zsh", state: .idle, folder: "terminal"),
        ]
        model.items = [.newTerminal(cwd: "~/Projects/terminal"), .openFolder]
        model.hotkey = "⇧⌘Space"
        model.close = close
        return model
    }
}

private extension View {
    /// A window-sized stage for an overlay that fills what it is given.
    var stage: some View {
        frame(height: 480).clipShape(RoundedRectangle(cornerRadius: 10)).transition(.opacity)
    }
}
