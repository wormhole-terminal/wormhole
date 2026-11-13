// The command palette (`command_palette/view.rs`): 600 wide, 72 below the window's top, on the scrim: the search
// row, the scope row, the results (up to 560 in all), and the footer's keys. Type to filter, a prefix or Tab
// narrows to one kind, ↑↓ move, ↩ runs, Esc clears, then goes back to everything, then closes.

import DesignSystem
import PaletteModel
import SwiftUI
import WordsModel
import WorkspaceModel

public struct PaletteView: View {
    public let state: WorkspaceState
    public let send: @MainActor (WorkspaceAction) -> Void
    public let dismiss: @MainActor () -> Void
    /// What the window lists and does for it: the keymap's actions, settings, commits (`PaletteSources`).
    let sources: PaletteSources
    let startsPresented: Bool

    @State var scope: PaletteScope
    @State var text: String
    /// The selected result, by id (rows come and go while typing); nil is the first one.
    @State var chosen: String?
    @State var hovered: String?
    /// The actions as the window offered them when the palette opened (the core's list for this window).
    @State var actions: ActionList
    /// The profiles when it opened, and what the settings index found for the query typed: both are looked up once, not on
    /// every redraw (hovering a row redraws the palette).
    @State var profiles: [PaletteProfile]
    @State var hits: [PaletteSetting] = []
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    public init(
        state: WorkspaceState, send: @escaping @MainActor (WorkspaceAction) -> Void,
        sources: PaletteSources = PaletteSources(), dismiss: @escaping @MainActor () -> Void
    ) {
        self.init(state: state, send: send, sources: sources, dismiss: dismiss, startsPresented: false)
    }

    init(
        state: WorkspaceState, send: @escaping @MainActor (WorkspaceAction) -> Void,
        sources: PaletteSources = PaletteSources(), dismiss: @escaping @MainActor () -> Void, startsPresented: Bool, query: String = "",
        chosen: String? = nil, hovered: String? = nil
    ) {
        self.state = state
        self.send = send
        self.dismiss = dismiss
        self.sources = sources
        _actions = State(initialValue: sources.actions())
        _profiles = State(initialValue: sources.profiles())
        self.startsPresented = startsPresented
        let (scope, rest) = PaletteScope.parse(query)
        _scope = State(initialValue: scope)
        _text = State(initialValue: rest)
        _chosen = State(initialValue: chosen)
        _hovered = State(initialValue: hovered)
    }

    var motion: OverlayMotion { OverlayMotion(reduceMotion: reduceMotion) }

    /// The rows at `now`, which the palette's own clock moves on every second while it is open (core `gather`).
    func rows(now: Date) -> [PaletteRow] {
        let recent = RecentStore.shared.ids
        let commands = CommandCatalog.commands(
            for: state, sources: sources, actions: actions, profiles: profiles, hits: hits, scope: scope, query: text, recent: recent, now: now)
        return PaletteRows.rows(
            scope: scope, query: text, commands: commands, recent: recent, suggested: actions.suggested, project: sources.project()?.name)
    }

    public var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { clock in content(now: clock.date) }
            .onAppear { sources.opened() }
    }

    private func content(now: Date) -> some View {
        let rows = rows(now: now)
        let selection = PaletteSelection.current(chosen: chosen, in: rows)
        return OverlayChrome(
            style: .palette, top: OverlayMetrics.paletteTop, startsPresented: startsPresented, dismiss: dismiss
        ) { context in
            card(rows: rows, selection: selection, context: context)
        }
        .accessibilityLabel(Text(Words.text("palette-title", fallback: "Command palette")))
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
                hints: PaletteFooter.hints(scope: scope, selected: rows.compactMap(\.result).first { $0.id == selection }?.command),
                trailing: PaletteFooter.trailing(scope: scope, query: text, rows: rows))
        }
        .frame(width: width)
        .overlayCard()
        .animation(motion.resize, value: listHeight(rows, in: context.size))
        .onChange(of: text) { _, new in typed(new) }
        .onChange(of: scope) { findSettings() }
        .onAppear { findSettings() }
    }

    /// The list grows with its results up to what the window leaves under the search and scope rows.
    private func listHeight(_ rows: [PaletteRow], in window: CGSize) -> CGFloat {
        let room = min(OverlayMetrics.paletteMaxHeight, window.height - OverlayMetrics.paletteTop - OverlayMetrics.bottomMargin)
        let chrome = Metric.paletteSearchHeight + 2 * Metric.paletteRowHeight
        let content = rows.reduce(OverlayMetrics.listBottom) { $0 + $1.height(in: scope) }
        return min(content, max(room - chrome, Metric.paletteRowHeight))
    }
}
