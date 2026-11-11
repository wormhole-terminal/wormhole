// The example band the terminal welcome's sidebar step puts in the list (`sidebar/demo.rs`): a card named "Example" with three
// rows whose statuses cycle working, needs input, done, so the program's page can point at them. What the rows say and when they
// change is the core's (`welcome::demo`, handed in as closures); this draws them with the list's own card and rows.
// They are not terminals: not in the workspace's state, so never listed, saved, numbered, selectable or reachable by a key; drawn
// without a click, a hover, a menu or a drag, and VoiceOver reads them as examples.

import SwiftUI
import WorkspaceModel

public struct ExampleBand {
    public struct Row: Sendable, Equatable {
        public var name: String
        public var state: Activity
        public var detail: String
        public var spoken: String

        public init(name: String, state: Activity, detail: String, spoken: String) {
            (self.name, self.state, self.detail, self.spoken) = (name, state, detail, spoken)
        }
    }

    /// The welcome's terminal that asked for it: the band goes with it.
    public var owner: TerminalID
    public var title: String
    public var label: String
    /// The three rows `seconds` after the band went up; the flag is Reduce Motion, which holds them as at the start.
    public var rows: @MainActor (_ seconds: Double, _ still: Bool) -> [Row]
    /// Seconds until the rows next change; nil when they never do.
    public var next: @MainActor (_ seconds: Double, _ still: Bool) -> Double?
    public let since = Date()

    public init(
        owner: TerminalID, title: String, label: String, rows: @escaping @MainActor (Double, Bool) -> [Row],
        next: @escaping @MainActor (Double, Bool) -> Double?
    ) {
        (self.owner, self.title, self.label, self.rows, self.next) = (owner, title, label, rows, next)
    }
}

struct ExampleBandView: View {
    let band: ExampleBand
    let context: SidebarContext
    @State private var seconds = 0.0
    @Environment(\.accessibilityReduceMotion) private var still

    var body: some View {
        SidebarSectionView(section: section, context: context)
            .allowsHitTesting(false)
            .task(id: still) {
                while !Task.isCancelled {
                    seconds = Date().timeIntervalSince(band.since)
                    // Under Reduce Motion nothing changes; a slow look catches the setting being turned off.
                    let wait = band.next(seconds, still) ?? 1
                    try? await Task.sleep(for: .seconds(max(wait, 0.05)))
                }
            }
    }

    private var section: SidebarSection {
        let rows = band.rows(seconds, still).enumerated().map { index, row in
            SidebarRow(
                terminal: TerminalSummary(id: -(index + 1), title: row.name, activity: row.state), secondLine: SecondLine(text: row.detail, tone: tone(row.state)), position: index + 1,
                of: 3, spoken: row.spoken)
        }
        let head = GroupHead(id: .max, name: band.title, spoken: band.label)
        return SidebarSection(id: "welcome-example", head: head, rows: rows, terminalCount: rows.count)
    }

    private func tone(_ state: Activity) -> SecondLineTone {
        switch state {
        case .needsInput: .attention
        case .finished: .done
        default: .plain
        }
    }
}
