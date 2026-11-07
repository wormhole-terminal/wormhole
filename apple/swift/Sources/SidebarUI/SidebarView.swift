// The list of terminals (`sidebar/list.rs`): cards from `SidebarListing`'s sections, under the window's own title row (the
// new-terminal button and the collapse button are the window's, as in the GPUI app). It draws and sends actions; it never
// edits state. Keys: the list is one tab stop and keeps a cursor of its own (`SidebarWalk`); Return opens a terminal.

import DesignSystem
import SwiftUI
import WorkspaceModel

public struct SidebarView: View {
    public let state: WorkspaceState
    public let send: @MainActor (WorkspaceAction) -> Void
    public let appearance: SidebarAppearance
    @State private var hold = CommandHoldMonitor()
    @State private var cursor: SidebarStop?
    @State private var keysLead = false
    @State private var typed = TypedPrefix()
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    public init(state: WorkspaceState, appearance: SidebarAppearance = SidebarAppearance(), send: @escaping @MainActor (WorkspaceAction) -> Void) {
        self.state = state
        self.appearance = appearance
        self.send = send
    }

    public var body: some View {
        let sections = SidebarListing.sections(for: state)
        let walk = SidebarWalk(sections)
        ScrollViewReader { proxy in
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(sections) { section in
                        SidebarSectionView(section: section, members: members(of: section), selected: state.selected,
                                           wash: appearance.groupWash[section.id], send: send)
                    }
                }
                .padding(.top, Space._1)
                .animation(SidebarMotion.list, value: sections.flatMap { $0.rows.map(\.id) })
            }
            .scrollIndicators(.automatic)
            .onChange(of: cursor) { _, stop in if let stop { withAnimation(SidebarMotion.state) { proxy.scrollTo(stop) } } }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(tokens.sidebarBackground)
        .environment(\.sidebarMetrics, SidebarMetrics(density: appearance.density))
        .environment(\.showsHints, appearance.forcesHints || hold.showing)
        .environment(\.keyedStop, focused && keysLead ? cursor : nil)
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onKeyPress(phases: [.down, .repeat]) { press in handle(press, walk) }
        .simultaneousGesture(TapGesture().onEnded { keysLead = false })
        .onChange(of: focused) { _, now in if now && cursor == nil { cursor = state.selected.map(SidebarStop.row) ?? walk.items.first?.stop } }
        .onAppear { hold.start() }
        .onDisappear { hold.stop() }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Terminals")
    }

    /// The terminals a group holds, which its head draws as tiles while it is folded.
    private func members(of section: SidebarSection) -> [TerminalSummary] {
        guard section.isCollapsed, let group = state.groups.first(where: { $0.id == section.id }) else { return [] }
        let known = Dictionary(state.terminals.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        return group.members.compactMap { known[$0] }.filter { matches($0) }
    }

    private func matches(_ terminal: TerminalSummary) -> Bool {
        let needle = state.search.trimmingCharacters(in: .whitespaces)
        return needle.isEmpty || [terminal.title, terminal.subtitle, terminal.branch].contains { $0?.range(of: needle, options: .caseInsensitive) != nil }
    }

    private func handle(_ press: KeyPress, _ walk: SidebarWalk) -> KeyPress.Result {
        keysLead = true
        let at = walk.index(of: cursor)
        let key: ListKey? = switch press.key {
        case .upArrow: press.modifiers.contains(.option) ? .first : .up
        case .downArrow: press.modifiers.contains(.option) ? .last : .down
        case .leftArrow: .left
        case .rightArrow: .right
        default: nil
        }
        if let key {
            switch walk.move(from: at, key) {
            case .to(let index): cursor = walk.items[index].stop
            case .open(let index), .fold(let index): if case .group(let id) = walk.items[index].stop { send(.toggleGroup(id)) }
            case .stay: break
            }
            return .handled
        }
        if press.key == .return || press.key == .space, let at {
            switch walk.items[at].stop {
            case .row(let id): send(.select(id))
            case .group(let id): if press.key == .space { send(.toggleGroup(id)) }
            }
            return .handled
        }
        guard press.modifiers.intersection([.command, .control]).isEmpty, let character = press.characters.first,
              !character.isWhitespace, !character.isNewline else { return .ignored }
        typed.add(character, at: Date())
        if let index = walk.typeAhead(from: at, typed.text) { cursor = walk.items[index].stop }
        return .handled
    }
}

/// Letters typed in the list: what was typed in the last second, so a name can be reached by its first letters.
struct TypedPrefix: Equatable {
    private(set) var text = ""
    private var last = Date.distantPast
    static let window: TimeInterval = 1

    mutating func add(_ character: Character, at now: Date) {
        if now.timeIntervalSince(last) > Self.window { text = "" }
        text.append(character)
        last = now
    }
}
