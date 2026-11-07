// The command palette (`command_palette/view.rs`): 600 wide, 72 below the window's top, on the scrim: the search
// row, the scope row, the results (up to 560 in all), and the footer's keys. Type to filter, a prefix or Tab
// narrows to one kind, ↑↓ move, ↩ runs, Esc clears, then goes back to everything, then closes.

import DesignSystem
import PaletteModel
import SwiftUI
import WorkspaceModel

public struct PaletteView: View {
    public let state: WorkspaceState
    public let send: @MainActor (WorkspaceAction) -> Void
    public let dismiss: @MainActor () -> Void
    /// Carries out a keymap action the workspace has no action for (a split, find, a window).
    let perform: @MainActor (String) -> Void
    let startsPresented: Bool

    @State var scope: PaletteScope
    @State var text: String
    /// The selected result, by id (rows come and go while typing); nil is the first one.
    @State var chosen: String?
    @State var hovered: String?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(
        state: WorkspaceState, send: @escaping @MainActor (WorkspaceAction) -> Void,
        perform: @escaping @MainActor (String) -> Void = { _ in }, dismiss: @escaping @MainActor () -> Void
    ) {
        self.init(state: state, send: send, perform: perform, dismiss: dismiss, startsPresented: false)
    }

    init(
        state: WorkspaceState, send: @escaping @MainActor (WorkspaceAction) -> Void,
        perform: @escaping @MainActor (String) -> Void = { _ in }, dismiss: @escaping @MainActor () -> Void, startsPresented: Bool, query: String = "",
        chosen: String? = nil, hovered: String? = nil
    ) {
        self.state = state
        self.send = send
        self.dismiss = dismiss
        self.perform = perform
        self.startsPresented = startsPresented
        let (scope, rest) = PaletteScope.parse(query)
        _scope = State(initialValue: scope)
        _text = State(initialValue: rest)
        _chosen = State(initialValue: chosen)
        _hovered = State(initialValue: hovered)
    }

    var motion: OverlayMotion { OverlayMotion(reduceMotion: reduceMotion) }

    var rows: [PaletteRow] {
        PaletteRows.rows(
            scope: scope, query: text, commands: CommandCatalog.commands(for: state), recent: RecentStore.shared.ids)
    }

    public var body: some View {
        let rows = rows
        let selection = PaletteSelection.current(chosen: chosen, in: rows)
        OverlayChrome(
            style: .palette, top: OverlayMetrics.paletteTop, startsPresented: startsPresented, dismiss: dismiss
        ) { context in
            card(rows: rows, selection: selection, context: context)
        }
        .accessibilityLabel(Text("Command palette"))
    }

    private func card(rows: [PaletteRow], selection: String?, context: OverlayContext) -> some View {
        let width = min(Metric.paletteWidth, context.size.width - 2 * OverlayMetrics.sideMargin)
        return VStack(spacing: 0) {
            SearchRow(scope: scope, text: $text) { key in
                handle(key, rows: rows, selection: selection, context: context)
            }
            ScopeRow(scope: scope) { setScope($0) }
            ResultList(
                rows: rows, scope: scope, selection: selection, hovered: $hovered, motion: motion,
                run: { run($0.command, context: context) }
            )
            .frame(height: listHeight(rows, in: context.size) - OverlayMetrics.listBottom)
            .padding(.bottom, OverlayMetrics.listBottom)
            PaletteFooterView(
                hints: PaletteFooter.hints(scope: scope),
                trailing: PaletteFooter.trailing(scope: scope, query: text, rows: rows))
        }
        .frame(width: width)
        .overlayCard()
        .animation(motion.resize, value: listHeight(rows, in: context.size))
        .onChange(of: text) { _, new in typed(new) }
    }

    /// The list grows with its results up to what the window leaves under the search and scope rows.
    private func listHeight(_ rows: [PaletteRow], in window: CGSize) -> CGFloat {
        let room = min(OverlayMetrics.paletteMaxHeight, window.height - OverlayMetrics.paletteTop - OverlayMetrics.bottomMargin)
        let chrome = Metric.paletteSearchHeight + 2 * Metric.paletteRowHeight
        let content = rows.reduce(OverlayMetrics.listBottom) { $0 + $1.height }
        return min(content, max(room - chrome, Metric.paletteRowHeight))
    }
}
