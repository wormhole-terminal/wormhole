// The list of terminals (`sidebar/list.rs`): cards from the core's sections (`state.sections`), under the window's own title row (the
// new-terminal button and the collapse button are the window's, as in the GPUI app). It draws and sends actions; it never
// edits state. Keys: the list is one tab stop and keeps a cursor of its own (`SidebarWalk`); Return opens a terminal and
// gives it the keyboard, Space only shows it. There is no search field: the GPUI list has none (⌘F is the terminal's Find).
// It paints no fill of its own: the column's surface is the window's, at the window's opacity over the blur
// (painting `sidebar.background` here as well hid the blur and read lighter than the GPUI app's).
// It holds only what a view must: the keys' cursor, the name being typed and the drag in flight. The second lines' words
// are the core's, read again by the store's one timer.

import DesignSystem
import SwiftUI
import UniformTypeIdentifiers
import WordsModel
import WorkspaceModel

public struct SidebarView: View {
    public let state: WorkspaceState
    public let send: @MainActor (WorkspaceAction) -> Void
    public let appearance: SidebarAppearance
    /// What the window answers: the close questions, the keyboard going back to the terminal.
    public let hooks: SidebarHooks
    /// What the window and the list say to each other (who has the keyboard, the keyboard's menus).
    public let link: SidebarLink
    /// The terminal welcome's example band, after the list's own cards while the program's sidebar step is up.
    public let band: ExampleBand?
    @State var hold = CommandHoldMonitor()
    @State var cursor: SidebarStop?
    @Environment(\.windowFront) private var windowFront
    @State var keysLead = false
    @State var typed = TypedPrefix()
    @State var renaming: SidebarRename?
    @State var flash: (id: TerminalID, generation: Int)?
    @State var rowFrames = RowFrames()
    @State var popover: GroupID?
    @State var frames: [DropSlot: CGRect] = [:]
    @State var drag: LiveDrag?
    /// Where a pane header dragged over the list would land (the list draws its mark), and which terminal it carries.
    @State var paneAim: DropAim?
    /// When this list was first drawn (a split's bracket drawn in later is one just made).
    @State var shownAt = Date()
    @State var paneDragReader = PaneHeaderDragReader()
    @State var shifts: [TerminalID: StatusShift] = [:]
    /// The list's frame in the window, and where a drag's pointer stands in it: what tells a drag over the list from one
    /// that has left it for the panes.
    @State var viewport: CGRect = .zero
    @State var windowPoint: CGPoint = .zero
    @State var outside = false
    @State var escape = DragEscape()
    @FocusState var focused: Bool
    @Environment(\.dropPlanner) var dropPlanner
    @Environment(\.terminalDrag) var terminalDrag
    @Environment(\.reduceMotion) var reduce
    public init(
        state: WorkspaceState, appearance: SidebarAppearance = SidebarAppearance(),
        hooks: SidebarHooks = SidebarHooks(), link: SidebarLink = SidebarLink(),
        band: ExampleBand? = nil, send: @escaping @MainActor (WorkspaceAction) -> Void
    ) {
        self.init(state: state, appearance: appearance, hooks: hooks, link: link, band: band, caught: nil, send: send)
    }

    /// A list caught mid-edit, for pictures and tests: a name half typed, a drag in flight, a group's popover open.
    /// The window starts every list at rest.
    init(
        state: WorkspaceState, appearance: SidebarAppearance, hooks: SidebarHooks,
        link: SidebarLink = SidebarLink(), band: ExampleBand? = nil, caught: Caught?,
        send: @escaping @MainActor (WorkspaceAction) -> Void
    ) {
        self.state = state
        self.link = link
        self.band = band
        self.appearance = appearance
        self.hooks = hooks
        self.send = send
        _renaming = State(initialValue: caught?.renaming)
        _drag = State(initialValue: caught?.drag)
        _popover = State(initialValue: caught?.popover)
    }

    struct Caught {
        var renaming: SidebarRename?
        var drag: LiveDrag?
        var popover: GroupID?
    }

    public var body: some View {
        let sections = state.sections
        let walk = SidebarWalk(sections)
        let context = context(sections)
        list(sections, walk, context)
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { viewport = $0 }
            .onChange(of: link.requests) { answer(link.request) }
            .onChange(of: link.keyboardRequests) { focused = true; keysLead = true }
            .onChange(of: focused, initial: true) { _, has in link.hasKeyboard = has }
    }

    private func list(_ sections: [SidebarSection], _ walk: SidebarWalk, _ context: SidebarContext) -> some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(spacing: 0) {
                    ForEach(sections) { section in
                        SidebarSectionView(section: section, context: context)
                    }
                    if let band { ExampleBandView(band: band, context: context) }
                }
                .padding(.top, Space._1)
                // At least the list's height, so a pane header can be let go below the rows (it leaves its split).
                .frame(maxWidth: .infinity, minHeight: viewport.height, alignment: .top)
                // Rows changing order slide on the core's spring; under Reduce Motion they stand in their new places at once
                // (the row plan measures nothing then).
                .motionTurn(SidebarMotion.list, value: sections.flatMap { $0.rows.map(\.id) })
                .coordinateSpace(.named(DragLift.space))
                .onPreferenceChange(DropFrames.self) { frames = $0 }
                .onDrop(of: [.plainText], delegate: paneHeaderDrop(sections))
                .overlay(alignment: .topLeading) { dragLayer(sections) }
            }
            .scrollIndicators(.automatic)
            .scrollEdge()
            .onChange(of: cursor) { _, stop in if let stop { withAnimation(reduce ? nil : SidebarMotion.state) { proxy.scrollTo(stop) } } }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .environment(\.sidebarMetrics, SidebarMetrics(density: appearance.density, cards: appearance.cards))
        .environment(\.sidebarLook, appearance)
        .environment(\.listRoom, drag?.aim?.room ?? ListRoom())
        .environment(\.listShownAt, shownAt)
        .environment(\.rowChanges, RowChanges.counts(sections, scope: appearance.changesScope))
        .environment(\.showsHints, appearance.forcesHints || (appearance.showsCommandHints && hold.showing))
        .environment(\.keyedStop, focused && keysLead && windowFront ? cursor : nil)
        .focusable()
        .focusEffectDisabled()
        .focused($focused)
        .onKeyPress(phases: [.down, .repeat]) { press in handle(press, walk) }
        .simultaneousGesture(TapGesture().onEnded { keysLead = false })
        .onChange(of: focused) { _, now in if now { enter(walk) } }
        .onChange(of: statuses) { old, new in noteStatusChanges(from: old, to: new) }
        .onAppear { hold.start() }
        .onDisappear { hold.stop() }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Words.text("sidebar-list", fallback: "Terminals"))
    }

    /// Coming into the list (F6, ⇥): the keys go back where they were, or onto the selected terminal (its group's head while it is
    /// folded away, which the walk does not hold), or the first item (`sidebar/keys.rs` `enter`).
    func enter(_ walk: SidebarWalk) {
        let stops = Set(walk.items.map(\.stop))
        if let cursor, stops.contains(cursor) { return }
        let selected = state.selected
        let group = selected.flatMap { id in state.all.first { $0.id == id }?.group }
        cursor = [selected.map(SidebarStop.row), group.map(SidebarStop.group)].compactMap { $0 }.first { stops.contains($0) }
            ?? walk.items.first?.stop
    }

    func context(_ sections: [SidebarSection]) -> SidebarContext {
        SidebarContext(
            selected: state.selected,
            groups: sections.compactMap { section in
                section.head.map { SidebarContext.GroupChoice(id: $0.id, name: $0.name, members: section.rows.map(\.id)) }
            },
            send: { act($0) }, open: { openTerminal($0) }, renaming: renaming, rename: { renaming = $0 }, shifts: shifts,
            drag: DragHooks(
                item: drag?.item, moved: { dragMoved($0, $1, sections) }, ended: { dragEnded($0, $1, sections) },
                window: { dragWindow($0, $1) }),
            everyTerminal: state.all, newGroup: { newGroup(for: $0) }, newInGroup: { newInGroup($0) },
            hooks: hooks, flash: flash,
            admits: { group, id in dropPlanner?.groupAdmits(group, terminal: id) ?? true },
            hasRoom: { other, beside in dropPlanner?.paneDropRefusal(dragged: other, target: beside, zone: .right) == nil },
            popover: popover, openPopover: { popover = $0 }, frames: rowFrames)
    }

    /// An action of the list: a row that moves to another group flashes once, as it lands (`sidebar/groups.rs` `flash`).
    func act(_ action: WorkspaceAction) {
        switch action {
        case .addToGroup(let id, _), .moveTerminal(let id, _, _, _): flashRow(id)
        case .dropTerminal(let id, let over): if case .move? = dropPlanner?.dropPlan(dragged: id, over: over) { flashRow(id) }
        default: break
        }
        send(action)
    }

    private func flashRow(_ id: TerminalID) { flash = (id, (flash?.generation ?? 0) + 1) }
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
