// Sample data for the UI modules' previews, screenshots and tests, and the `--mock` window: a window as a working day
// leaves it, in the shapes of the core's snapshot. The listing here is the least that makes them: items in the order
// given, the first nine open rows numbered, brackets along terminals that share a split. No pin, fold or search rule lives
// here; the core's does (`WorkspaceBridge`).

import Foundation

/// A terminal with the words its row reads.
public struct MockTerminal: Sendable, Equatable {
    public var terminal: TerminalSummary
    public var secondLine: SecondLine

    public init(_ terminal: TerminalSummary, _ secondLine: SecondLine? = nil) {
        self.terminal = terminal
        self.secondLine = secondLine ?? SecondLine(text: terminal.message ?? terminal.folder ?? "Shell")
    }
}

public enum MockItem: Sendable, Equatable {
    case loose(MockTerminal)
    case group(GroupHead, [MockTerminal])
}

public enum MockWorkspace {
    /// The listing of `items` as a snapshot's sections.
    public static func sections(_ items: [MockItem], selected: TerminalID?) -> [SidebarSection] {
        var number = 0
        let selectedSplit = items.flatMap(terminals).first { $0.terminal.id == selected }?.terminal.splitID
        func rows(_ list: [MockTerminal], open: Bool) -> [SidebarRow] {
            guard open else { return [] }
            return list.enumerated().map { at, mock in
                number += 1
                let split = mock.terminal.splitID
                let mates = list.indices.filter { list[$0].terminal.splitID == split && split != nil }
                let bracket: Bracket? = mates.count < 2 || split == nil ? nil : at == mates.first ? .start : at == mates.last ? .end : .mid
                return SidebarRow(
                    terminal: mock.terminal, number: number <= 9 ? number : nil, keycap: number <= 9 ? "⌘\(number)" : nil,
                    splitMarks: SplitMarks(
                        bracket: bracket, onScreen: split != nil && split == selectedSplit && mock.terminal.id != selected,
                        members: split == nil ? 0 : mates.count),
                    secondLine: mock.secondLine, position: at + 1, of: list.count,
                    spoken: "\(mock.terminal.title), \(mock.terminal.activity)")
            }
        }
        // A run of loose terminals is one section, as the core cuts the list.
        var sections: [SidebarSection] = []
        var run: [MockTerminal] = []
        func flush() {
            guard let first = run.first else { return }
            sections.append(SidebarSection(id: "loose-\(first.terminal.id)", rows: rows(run, open: true), alertCount: run.filter(alert).count))
            run = []
        }
        for item in items {
            switch item {
            case .loose(let mock): run.append(mock)
            case .group(var head, let members):
                flush()
                if head.spoken.isEmpty { head.spoken = "\(head.name), \(members.count) terminals, \(head.isFolded ? "collapsed" : "expanded")" }
                sections.append(
                    SidebarSection(
                        id: "group-\(head.id)", head: head, rows: rows(members, open: !head.isFolded), terminalCount: members.count,
                        alertCount: members.filter(alert).count))
            }
        }
        flush()
        return sections
    }

    private static func terminals(_ item: MockItem) -> [MockTerminal] {
        switch item {
        case .loose(let mock): [mock]
        case .group(_, let members): members
        }
    }

    private static func alert(_ mock: MockTerminal) -> Bool { mock.terminal.activity == .needsInput || mock.terminal.activity == .failed }

    public static let items: [MockItem] = {
        let app = "~/Projects/terminal/wormhole"
        let web = "~/Projects/terminal/web"
        func line(_ text: String, _ tone: SecondLineTone = .plain, command: Bool = false, trail: String? = nil) -> SecondLine {
            SecondLine(text: text, tone: tone, isCommand: command, trail: trail)
        }
        return [
            .loose(MockTerminal(TerminalSummary(id: 10, title: "htop", folder: "~", isPinned: true))),
            .loose(
                MockTerminal(
                    TerminalSummary(
                        id: 12, title: "release 1.7", folder: app, branch: "release/1.7", activity: .needsInput, isPinned: true,
                        unread: true, agent: .claude, message: "Run the release script?", noteCount: 1),
                    line("Needs input · Run the release script?", .attention))),
            .group(
                GroupHead(id: 1, name: "Wormhole", color: .blue, folder: app),
                [
                    MockTerminal(
                        TerminalSummary(
                            id: 1, title: "sidebar polish", folder: app, branch: "sidebar-polish", activity: .working,
                            agent: .claude, noteCount: 2, changeCount: 7), line("Working · 3m")),
                    MockTerminal(
                        TerminalSummary(id: 2, title: "cargo test", folder: app, branch: "main", activity: .working),
                        line("cargo test -p wormhole-term", command: true, trail: "1:12")),
                    MockTerminal(
                        TerminalSummary(
                            id: 3, title: "diff perf", folder: "\(app)/crates/wormhole-term", branch: "diff-perf",
                            activity: .needsInput, isPinned: true, unread: true, agent: .claude, message: "Approve cargo test?",
                            changeCount: 12), line("Needs input · Approve cargo test?", .attention)),
                    MockTerminal(TerminalSummary(id: 4, title: "zsh", folder: app, branch: "main")),
                ]),
            .group(
                GroupHead(id: 2, name: "Website", color: .purple, folder: web),
                [
                    MockTerminal(
                        TerminalSummary(
                            id: 5, title: "landing hero", folder: web, branch: "hero-copy", activity: .working, agent: .codex,
                            message: "Editing hero.tsx…", splitID: 1, changeCount: 4), line("Editing hero.tsx…")),
                    MockTerminal(
                        TerminalSummary(
                            id: 6, title: "pnpm build", folder: web, branch: "hero-copy", activity: .working, splitID: 1,
                            changeCount: 4), line("pnpm build", command: true, trail: "0:18")),
                    MockTerminal(
                        TerminalSummary(
                            id: 7, title: "deploy preview", folder: web, branch: "hero-copy", activity: .failed, unread: true,
                            changeCount: 4), line("pnpm deploy · exit 1", .failed, command: true, trail: "2m ago")),
                ]),
            .loose(MockTerminal(TerminalSummary(id: 11, title: "docs pass", folder: "~/Documents/notes", activity: .finished, agent: .codex, noteCount: 3), line("Finished · 12m ago", .done))),
            .group(
                GroupHead(id: 3, name: "Infra", color: .cyan, isFolded: true, folder: "~/infra"),
                [
                    MockTerminal(TerminalSummary(id: 8, title: "ssh build-mac", folder: "~/infra")),
                    MockTerminal(TerminalSummary(id: 9, title: "terraform plan", folder: "~/infra", branch: "network-split", activity: .finished, unread: true)),
                ]),
            .loose(MockTerminal(TerminalSummary(id: 13, title: "scratch", folder: "/tmp"))),
        ]
    }()

    public static let state: WorkspaceState = {
        WorkspaceState(
            sections: Self.sections(items, selected: 1), selected: 1, live: items.flatMap(terminals).map(\.terminal.id))
    }()
}
