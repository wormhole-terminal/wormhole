// The list of terminals (`sidebar/list.rs`): cards from the core's sections (`state.sections`), under the window's own title row (the
// new-terminal button and the collapse button are the window's, as in the GPUI app). It draws and sends actions; it never
// edits state. Keys: the list is one tab stop and keeps a cursor of its own (`SidebarWalk`); Return opens a terminal.
// It paints no fill of its own: the column's surface is the window's, at the window's opacity over the blur
// (painting `sidebar.background` here as well hid the blur and read lighter than the GPUI app's).
// It holds only what a view must: the keys' cursor, the name being typed and the drag in flight. The second lines' words
// are the core's, read again by the store's one timer.

import DesignSystem
import SwiftUI
import WorkspaceModel

public struct SidebarView: View {
    public let state: WorkspaceState
    public let send: @MainActor (WorkspaceAction) -> Void
    public let appearance: SidebarAppearance
    /// How the window answers closing a group (it asks first when something works in it); nil sends the close at once.
    public let closeGroup: (@MainActor (GroupID) -> Void)?
    /// What ⌘F asks of the list; the window keeps it, so it can tell whether the sidebar has the keyboard.
    public let search: SidebarSearch
    @State var searchText: String
    @State var searchOpen = false
    @FocusState var searchFocused: Bool
    @State var hold = CommandHoldMonitor()
    @State var cursor: SidebarStop?
    @State var keysLead = false
    @State var typed = TypedPrefix()
    @State var renaming: TerminalID?
    @State var popover: GroupID?
    @State var frames: [DropSlot: CGRect] = [:]
    @State var drag: LiveDrag?
    @State var shifts: [TerminalID: Int] = [:]
    /// The list's frame in the window, and where a drag's pointer stands in it: what tells a drag over the list from one
    /// that has left it for the panes.
    @State var viewport: CGRect = .zero
    @State var windowPoint: CGPoint = .zero
    @State var outside = false
    @FocusState var focused: Bool
    @Environment(\.dropPlanner) var dropPlanner
    @Environment(\.terminalDrag) var terminalDrag
    @Environment(\.accessibilityReduceMotion) var reduce
    public init(
        state: WorkspaceState, appearance: SidebarAppearance = SidebarAppearance(),
        closeGroup: (@MainActor (GroupID) -> Void)? = nil, search: SidebarSearch = SidebarSearch(),
        send: @escaping @MainActor (WorkspaceAction) -> Void
    ) {
        self.init(state: state, appearance: appearance, closeGroup: closeGroup, search: search, caught: nil, send: send)
    }

    /// A list caught mid-edit, for pictures and tests: a name half typed, a drag in flight, a group's popover open.
    /// The window starts every list at rest.
    init(
        state: WorkspaceState, appearance: SidebarAppearance, closeGroup: (@MainActor (GroupID) -> Void)?,
        search: SidebarSearch = SidebarSearch(), caught: Caught?, send: @escaping @MainActor (WorkspaceAction) -> Void
    ) {
        self.state = state
        self.search = search
        _searchText = State(initialValue: state.search)
        _searchOpen = State(initialValue: caught?.searchOpen ?? false)
        self.appearance = appearance
        self.closeGroup = closeGroup
        self.send = send
        _renaming = State(initialValue: caught?.renaming)
        _drag = State(initialValue: caught?.drag)
        _popover = State(initialValue: caught?.popover)
    }

    struct Caught {
        var renaming: TerminalID?
        var drag: LiveDrag?
        var popover: GroupID?
        var searchOpen = false
    }

    public var body: some View {
        let sections = state.sections
        let walk = SidebarWalk(sections)
        let context = context(sections)
        VStack(spacing: 0) {
            searchBar
            list(sections, walk, context)
        }
        .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { viewport = $0 }
        .onChange(of: search.requests) { revealSearch() }
        .onChange(of: state.search) { _, text in if searchText != text { searchText = text } }
        .onChange(of: searchText) { _, text in if text != state.search { send(.search(text)) } }
        .onChange(of: focused || searchFocused, initial: true) { _, has in search.hasKeyboard = has }
    }

    private func list(_ sections: [SidebarSection], _ walk: SidebarWalk, _ context: SidebarContext) -> some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(sections) { section in
                        SidebarSectionView(section: section, context: context)
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
        .overlay(alignment: .top) { if sections.isEmpty && searching { NothingFound() } }
        .environment(\.sidebarMetrics, SidebarMetrics(density: appearance.density, cards: appearance.cards))
        .environment(\.sidebarLook, appearance)
        .environment(\.showsHints, appearance.forcesHints || hold.showing)
        .environment(\.keyedStop, focused && keysLead ? cursor : nil)
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onKeyPress(phases: [.down, .repeat]) { press in handle(press, walk) }
        .simultaneousGesture(TapGesture().onEnded { keysLead = false })
        .onChange(of: focused) { _, now in if now && cursor == nil { cursor = state.selected.map(SidebarStop.row) ?? walk.items.first?.stop } }
        .onChange(of: statuses) { old, new in noteStatusChanges(from: old, to: new) }
        .onAppear { hold.start() }
        .onDisappear { hold.stop() }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Terminals")
        .accessibilityAction(named: SidebarText.searchPlaceholder) { search.reveal() }
    }

    private func context(_ sections: [SidebarSection]) -> SidebarContext {
        SidebarContext(
            selected: state.selected,
            groups: sections.compactMap { section in
                section.head.map { SidebarContext.GroupChoice(id: $0.id, name: $0.name, members: section.rows.map(\.id)) }
            },
            send: send, renaming: renaming, rename: { renaming = $0 }, shifts: shifts,
            drag: DragHooks(
                item: drag?.item, moved: { dragMoved($0, $1, sections) }, ended: { dragEnded($0, $1, sections) },
                window: { dragWindow($0, $1) }),
            everyTerminal: state.all, newInGroup: { newInGroup($0) },
            closeGroup: { closeGroup?($0) ?? send(.closeGroup($0)) }, popover: popover, openPopover: { popover = $0 })
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
