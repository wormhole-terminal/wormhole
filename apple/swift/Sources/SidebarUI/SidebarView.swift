// The list of terminals (`sidebar/list.rs`): cards from `SidebarListing`'s sections, under the window's own title row (the
// new-terminal button and the collapse button are the window's, as in the GPUI app). It draws and sends actions; it never
// edits state. Keys: the list is one tab stop and keeps a cursor of its own (`SidebarWalk`); Return opens a terminal.
// It paints no fill of its own: the column's surface is the window's, at the window's opacity over the blur
// (painting `sidebar.background` here as well hid the blur and read lighter than the GPUI app's).
// It holds only what a view must: the keys' cursor, the name being typed, the drag in flight and the one timer.

import DesignSystem
import SwiftUI
import WorkspaceModel

public struct SidebarView: View {
    public let state: WorkspaceState
    public let send: @MainActor (WorkspaceAction) -> Void
    public let appearance: SidebarAppearance
    public let groupEdits: GroupEdits?
    @State var hold = CommandHoldMonitor()
    @State var cursor: SidebarStop?
    @State var keysLead = false
    @State var typed = TypedPrefix()
    @State var now = Date()
    @State var renaming: TerminalID?
    @State var popover: String?
    @State var frames: [DropSlot: CGRect] = [:]
    @State var drag: LiveDrag?
    @State var shifts: [TerminalID: Int] = [:]
    @FocusState var focused: Bool
    public init(
        state: WorkspaceState, appearance: SidebarAppearance = SidebarAppearance(), groupEdits: GroupEdits? = nil,
        send: @escaping @MainActor (WorkspaceAction) -> Void
    ) {
        self.init(state: state, appearance: appearance, groupEdits: groupEdits, caught: nil, send: send)
    }

    /// A list caught mid-edit, for pictures and tests: a name half typed, a drag in flight, a group's popover open.
    /// The window starts every list at rest.
    init(
        state: WorkspaceState, appearance: SidebarAppearance, groupEdits: GroupEdits?, caught: Caught?,
        send: @escaping @MainActor (WorkspaceAction) -> Void
    ) {
        self.state = state
        self.appearance = appearance
        self.groupEdits = groupEdits
        self.send = send
        _renaming = State(initialValue: caught?.renaming)
        _drag = State(initialValue: caught?.drag)
        _popover = State(initialValue: caught?.popover)
    }

    struct Caught {
        var renaming: TerminalID?
        var drag: LiveDrag?
        var popover: String?
    }

    public var body: some View {
        let sections = SidebarListing.sections(for: state)
        let walk = SidebarWalk(sections)
        let context = context(sections)
        ScrollViewReader { proxy in
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(sections) { section in
                        SidebarSectionView(section: section, members: members(of: section), context: context)
                    }
                }
                .padding(.top, Space._1)
                .animation(SidebarMotion.list, value: sections.flatMap { $0.rows.map(\.id) })
                .coordinateSpace(.named(DragLift.space))
                .onPreferenceChange(DropFrames.self) { frames = $0 }
                .overlay(alignment: .topLeading) { dragLayer(sections) }
            }
            .scrollIndicators(.automatic)
            .onChange(of: cursor) { _, stop in if let stop { withAnimation(SidebarMotion.state) { proxy.scrollTo(stop) } } }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .environment(\.sidebarMetrics, SidebarMetrics(density: appearance.density, cards: appearance.cards))
        .environment(\.sidebarLook, appearance)
        .environment(\.sidebarNow, appearance.now ?? now)
        .environment(\.showsHints, appearance.forcesHints || hold.showing)
        .environment(\.keyedStop, focused && keysLead ? cursor : nil)
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onKeyPress(phases: [.down, .repeat]) { press in handle(press, walk) }
        .simultaneousGesture(TapGesture().onEnded { keysLead = false })
        .onChange(of: focused) { _, now in if now && cursor == nil { cursor = state.selected.map(SidebarStop.row) ?? walk.items.first?.stop } }
        .onChange(of: statuses) { old, new in noteStatusChanges(from: old, to: new) }
        .onChange(of: state.terminals) { now = Date() }
        .task(id: nextChange) { await waitForNextChange() }
        .onAppear { hold.start() }
        .onDisappear { hold.stop() }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Terminals")
    }

    /// The terminals a group holds, which its head draws as tiles while it is folded.
    private func members(of section: SidebarSection) -> [TerminalSummary] {
        guard section.isCollapsed, let group = section.group else { return [] }
        let known = Dictionary(state.terminals.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        return group.members.compactMap { known[$0] }.filter(matches)
    }

    private func matches(_ terminal: TerminalSummary) -> Bool {
        let needle = state.search.trimmingCharacters(in: .whitespaces)
        return needle.isEmpty || [terminal.title, terminal.subtitle, terminal.branch].contains { $0?.range(of: needle, options: .caseInsensitive) != nil }
    }

    private func context(_ sections: [SidebarSection]) -> SidebarContext {
        SidebarContext(
            selected: state.selected,
            groups: state.groups.map { SidebarContext.GroupChoice(id: $0.id, name: $0.name, members: $0.members) },
            send: send, renaming: renaming, rename: { renaming = $0 }, shifts: shifts,
            drag: DragHooks(item: drag?.item, moved: { dragMoved($0, $1, sections) }, ended: { dragEnded($0, $1, sections) }),
            groupEdits: groupEdits, popover: popover, openPopover: { popover = $0 },
            nextTerminalID: (state.terminals.map(\.id).max() ?? 0) + 1,
            nextGroupID: (1...).lazy.map { "group-\($0)" }.first { id in !state.groups.contains { $0.id == id } } ?? "group-1")
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
