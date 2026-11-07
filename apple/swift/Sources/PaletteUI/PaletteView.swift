// The command palette: a centred card over a dimmed window. Type to filter, ↑↓ to move, ↩ to run, esc or a
// click outside to close. Run sends the command's action, then closes; the window only shows or removes it.

import DesignSystem
import PaletteModel
import SwiftUI
import WorkspaceModel

public struct PaletteView: View {
    public let state: WorkspaceState
    public let send: @MainActor (WorkspaceAction) -> Void
    public let dismiss: @MainActor () -> Void
    let startsPresented: Bool

    @State private var query = ""
    @State private var selectedID: String?
    @State private var hoveredID: String?
    @FocusState private var focused: Bool
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(
        state: WorkspaceState, send: @escaping @MainActor (WorkspaceAction) -> Void,
        dismiss: @escaping @MainActor () -> Void
    ) {
        self.init(state: state, send: send, dismiss: dismiss, startsPresented: false)
    }

    init(
        state: WorkspaceState, send: @escaping @MainActor (WorkspaceAction) -> Void,
        dismiss: @escaping @MainActor () -> Void, startsPresented: Bool, query: String = ""
    ) {
        self.state = state
        self.send = send
        self.dismiss = dismiss
        self.startsPresented = startsPresented
        _query = State(initialValue: query)
    }

    private var motion: PaletteMotion { PaletteMotion(reduceMotion: reduceMotion) }

    public var body: some View {
        let results = PaletteSearch.results(for: query, in: CommandCatalog.commands(for: state))
        let selection = results.contains { $0.id == selectedID } ? selectedID : results.first?.id
        OverlayChrome(width: 600, startsPresented: startsPresented, dismiss: dismiss) { close in
            VStack(spacing: 0) {
                SearchField(
                    prompt: "Search commands and terminals…", text: $query,
                    scope: PaletteScope.parse(query).scope.title, focus: $focused
                )
                .animation(motion.results, value: PaletteScope.parse(query).scope)
                Divider().overlay(Palette.divider)
                if results.isEmpty {
                    empty
                } else {
                    PaletteList(
                        results: results, selection: selection, hovered: $hoveredID,
                        grouped: query.allSatisfy(\.isWhitespace), motion: motion,
                        run: { run($0, close: close) })
                }
                Divider().overlay(Palette.divider)
                hints
            }
            .onKeyPress(.upArrow) { move(-1, in: results, from: selection) }
            .onKeyPress(.downArrow) { move(1, in: results, from: selection) }
            .onKeyPress(.escape) { close(); return .handled }
            .onKeyPress(.return) {
                guard let result = results.first(where: { $0.id == selection }) else { return .ignored }
                run(result, close: close)
                return .handled
            }
            // ⌘K closes as it opens, so the key is a toggle.
            .onKeyPress(characters: ["k"], phases: .down) { press in
                guard press.modifiers == .command else { return .ignored }
                close()
                return .handled
            }
            .task { focused = true }
            .onChange(of: query) { selectedID = nil }
        }
        .accessibilityLabel(Text("Command palette"))
    }

    private var empty: some View {
        Text("No matches for “\(query)”")
            .font(.system(size: 14))
            .foregroundStyle(Palette.textSecondary)
            .frame(maxWidth: .infinity)
            .frame(height: PaletteItem.rowHeight * 2)
            .accessibilityLabel(Text("No results"))
    }

    private var hints: some View {
        HStack(spacing: Space._4) {
            hint("↑↓", "Navigate")
            hint("↩", "Run")
            hint("esc", "Close")
            Spacer()
        }
        .padding(.horizontal, Space._4)
        .frame(height: Space._6 + Space._2)
        .accessibilityHidden(true)
    }

    private func hint(_ key: String, _ label: String) -> some View {
        HStack(spacing: Space._1) {
            Text(key).font(.system(size: 11, weight: .medium, design: .rounded))
            Text(label).font(.system(size: 11))
        }
        .foregroundStyle(Palette.textSecondary)
    }

    private func run(_ result: PaletteResult, close: @MainActor () -> Void) {
        if let action = result.command.action { send(action) }
        close()
    }

    private func move(_ step: Int, in results: [PaletteResult], from selection: String?) -> KeyPress.Result {
        guard !results.isEmpty else { return .handled }
        let at = results.firstIndex { $0.id == selection } ?? 0
        selectedID = results[(at + step + results.count) % results.count].id
        return .handled
    }
}
