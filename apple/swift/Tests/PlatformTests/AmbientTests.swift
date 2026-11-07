import Foundation
import PlatformModel
import Testing

/// The ambient surfaces' rules (`notify::listing`, `menubar::model`, `quick_panel::rows`).
@Suite struct AmbientTests {
    static let now = Date(timeIntervalSince1970: 1_000_000)

    static func terminal(
        _ tab: Int64, _ name: String, _ state: AmbientState, ago: TimeInterval = 0, source: AmbientSource = .plain
    ) -> AmbientTerminal {
        AmbientTerminal(session: tab, name: name, state: state, since: now - ago, source: source)
    }

    private var sample: [AmbientTerminal] {
        var diff = Self.terminal(1, "diff perf", .needsYou, ago: 120, source: .agent)
        diff.who = "Claude Code"
        diff.message = "Approve cargo test?"
        var build = Self.terminal(4, "build", .done, ago: 120)
        build.command = "cargo build"
        build.took = 38
        return [
            diff, Self.terminal(2, "prod", .needsYou, ago: 300), Self.terminal(3, "tests", .working, ago: 72), build,
            Self.terminal(5, "idle", .idle),
        ]
    }

    @Test func sectionsSortByUrgencyAndAge() {
        let sections = AmbientSections(sample)
        #expect(sections.needsYou == [1, 0], "the longest waiting first")
        #expect(sections.working == [2] && sections.finished == [3] && sections.other == [4])
        let same = [Self.terminal(1, "a", .working), Self.terminal(2, "b", .working)]
        #expect(AmbientSections(same).working == [0, 1], "equal times keep the sidebar order")
        let finished = [Self.terminal(1, "old", .done, ago: 600), Self.terminal(2, "new", .failed, ago: 5)]
        #expect(AmbientSections(finished).finished == [1, 0], "finished and failed, newest first")
    }

    @Test func countsStateAndTooltip() {
        let counts = AmbientCounts(of: sample)
        #expect(counts.needsYou == 2 && counts.working == 1 && counts.finished == 1 && counts.quiet == 1)
        #expect(counts.itemState == .needsYou)
        #expect(counts.tooltip == "Wormhole — 2 need input · 1 working · 1 finished")
        #expect(AmbientCounts(of: []).tooltip == "Wormhole — no terminals open")
        #expect(AmbientCounts(of: [Self.terminal(1, "a", .idle)]).tooltip == "Wormhole — 1 terminal, all quiet")
        #expect(AmbientCounts(of: [Self.terminal(1, "a", .failed), Self.terminal(2, "b", .done)]).itemState == .failed)
        #expect(AmbientCounts.badge(0) == nil && AmbientCounts.badge(4) == "4" && AmbientCounts.badge(10) == "9+")
    }

    @Test func theStatusItemsIcon() {
        let waiting = StatusIcon(sample, mode: .statusCount, animate: true)
        #expect(waiting.mark == .needsYou && waiting.count == "2" && !waiting.animate)
        #expect(StatusIcon(sample, mode: .status, animate: false).count == nil)
        #expect(StatusIcon(sample, mode: .plain, animate: false).mark == .none)
        let working = StatusIcon([Self.terminal(1, "a", .working)], mode: .statusCount, animate: true)
        #expect(working.state == .working && working.animate && working.mark == .none)
    }

    @Test func theWordsOfARow() {
        #expect(AmbientWording.elapsed(.needsYou, since: 30) == "now" && AmbientWording.elapsed(.needsYou, since: 125) == "2m")
        #expect(AmbientWording.elapsed(.working, since: 252) == "4:12" && AmbientWording.elapsed(.done, since: 720) == "12m ago")
        #expect(AmbientWording.elapsed(.idle, since: 5) == nil && AmbientWording.ago(10) == "just now")
        #expect(AmbientWording.took(3700) == "1h" && AmbientWording.took(90_000) == "1d")
        #expect(AmbientWording.clock(3723) == "1:02:03" && AmbientWording.duration(252) == "4m 12s")
        let diff = sample[0]
        #expect(diff.detail(withAgent: true) == "Claude Code · Approve cargo test?")
        #expect(diff.detail(withAgent: false) == "Approve cargo test?")
        #expect(sample[3].detail(withAgent: false) == "cargo build · after 38s")
        #expect(sample[1].isBell && sample[1].detail(withAgent: true) == "Bell")
        var failed = Self.terminal(9, "x", .failed)
        failed.command = "cargo build"
        failed.exit = 101
        failed.took = 38
        #expect(failed.detail(withAgent: true) == "cargo build · exit 101 · after 38s")
    }

    @Test func theStatusMenuListsTheSections() {
        var snapshot = StatusSnapshot()
        snapshot.terminals = sample
        snapshot.hotkey = KeySpec([.shift, .command], "space")
        snapshot.jumpKey = KeySpec([.option, .command], "j")
        snapshot.recent = [.init(name: "terminal", path: "/Users/ben/Projects/terminal")]
        snapshot.actions = [.init(project: "api", root: "/src/api", id: "test", title: "test")]
        let model = StatusMenu.build(snapshot, now: Self.now)
        let text = MenuDump.lines(model.menu, depth: 0)
        #expect(text == """
            # Needs input
            prod  ⇥ 5m
                Bell
            diff perf  ⇥ 2m
                Claude Code · Approve cargo test?
            # Working
            tests  ⇥ 1:12
            # Finished
            build  ⇥ 2m ago
                cargo build · after 38s
            ---
            Other Terminals  ⇥ 1 ▸
              idle
            ---
            New Terminal  ⌘T
            Open Folder…  ⌘O
            Projects ▸
              # Recent
              terminal
                  /Users/ben/Projects/terminal
              # Actions · api
              test
              ---
              Open Folder…  ⌘O
            ---
            Show Wormhole  ⇧⌘Space
            Jump to Next Waiting  ⌥⌘J
            Settings…  ⌘,
            Check for Updates…
            ---
            Quit Wormhole  ⌘Q

            """)
        #expect(model.icon.tooltip.hasPrefix("Wormhole — 2 need input"))
    }

    @Test func anEmptyMenuSaysSo() {
        let text = MenuDump.lines(StatusMenu.build(StatusSnapshot(), now: Self.now).menu, depth: 0)
        #expect(text.hasPrefix("No terminals open  (disabled)\n---\nNew Terminal  ⌘T"))
        #expect(text.contains("Jump to Next Waiting  (disabled)"))
    }

    @Test func manyTerminalsFoldIntoMoreRows() {
        var snapshot = StatusSnapshot()
        snapshot.terminals = (1...25).map { Self.terminal(Int64($0), "t\($0)", .working, ago: Double($0)) }
        let text = MenuDump.lines(StatusMenu.build(snapshot, now: Self.now).menu, depth: 0)
        #expect(text.contains("5 more…") && !text.contains("t5 "), "the 20 longest running are listed")
    }

    @Test func theDockMenuAndBadge() {
        let text = MenuDump.lines(DockMenu.build(sample, now: Self.now), depth: 0)
        #expect(text == "# Needs input\nprod   5m\ndiff perf   2m\n# Working\ntests   1m\n---\nNew Terminal\nNew Window\n")
        let counts = AmbientCounts(of: sample + [Self.terminal(7, "f", .failed)])
        #expect(DockBadgeMode.off.label(for: counts) == nil)
        #expect(DockBadgeMode.needsYou.label(for: counts) == "2")
        #expect(DockBadgeMode.needsYouDone.label(for: counts) == "4")
        #expect(DockBadgeMode.needsYou.label(for: AmbientCounts(of: [Self.terminal(1, "a", .working)])) == nil)
        #expect(DockBadgeMode.needsYou.label(for: AmbientCounts(of: (1...1200).map { Self.terminal(Int64($0), "n", .needsYou) })) == "999+")
    }

    @Test func nextWaitingGoesRoundInWaitingOrder() {
        #expect(AmbientSections.nextWaiting([4, 7, 9], after: nil) == 4)
        #expect(AmbientSections.nextWaiting([4, 7, 9], after: 4) == 7)
        #expect(AmbientSections.nextWaiting([4, 7, 9], after: 9) == 4)
        #expect(AmbientSections.nextWaiting([4, 7, 9], after: 5) == 4)
        #expect(AmbientSections.nextWaiting([], after: nil) == nil)
    }

    // MARK: the quick panel's rows

    private func shape(_ rows: [QuickPanelRow]) -> [String] {
        rows.map { row in
            switch row {
            case .label(let title, let count): "[\(title)\(count.map { " \($0)" } ?? "")]"
            case .note(let text): "(\(text))"
            case .terminal(let terminal): terminal.name
            case .action(let item): item.title
            }
        }
    }

    private let items: [QuickPanelDo] = [
        .newTerminal(cwd: nil), .openFolder, .action(.init(project: "api-server", root: "/src/api-server", id: "test", title: "test")),
    ]

    @Test func withoutAQueryItShowsTheMenusSectionsThenDo() {
        #expect(shape(QuickPanelRows.rows(sample, items, query: "")) == [
            "[Needs input 2]", "prod", "diff perf", "[Working 1]", "tests", "[Recent 1]", "build", "[Do]", "New terminal",
            "Open folder…", "Run test",
        ])
    }

    @Test func nothingWaitingSaysSo() {
        let shown = shape(QuickPanelRows.rows([Self.terminal(1, "idle", .idle)], items, query: " "))
        #expect(Array(shown.prefix(3)) == ["[Needs input 0]", "(Nothing needs input)", "[Do]"])
    }

    @Test func aQueryFindsTerminalsOfAnyStateAndThingsToDo() {
        let terminals = [Self.terminal(1, "diff perf", .needsYou), Self.terminal(2, "diff idle", .idle)]
        #expect(shape(QuickPanelRows.rows(terminals, items, query: "diff")) == ["[Terminals 2]", "diff perf", "diff idle"])
        #expect(shape(QuickPanelRows.rows(terminals, items, query: "api")) == ["[Do]", "Run test"])
        #expect(shape(QuickPanelRows.rows(terminals, items, query: "zebra")) == ["(Nothing matches)"])
    }

    @Test func onlyTerminalsAndActionsTakeTheSelectionAndOnlyWaitingOnesTakeAReply() {
        let waiting = QuickPanelRow.terminal(Self.terminal(1, "a", .needsYou))
        let working = QuickPanelRow.terminal(Self.terminal(2, "b", .working))
        #expect(waiting.selectable && working.selectable && QuickPanelRow.action(.openFolder).selectable)
        #expect(!QuickPanelRow.label("Do", count: nil).selectable && !QuickPanelRow.note("x").selectable)
        #expect(waiting.waiting != nil && working.waiting == nil)
    }

    @Test func doItemsCarryTheKeysOfTheApp() {
        #expect(items[0].keys(.bundled) == "⇧⌘N" && items[1].keys(.bundled) == "⌘O" && items[2].keys(.bundled) == nil)
        #expect(items[2].detail == "action · api-server" && items[0].command == .newTerminal)
    }
}
