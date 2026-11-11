// The rows inside a group's card, and what they do as the group folds and opens. The core's sections list no rows while a
// group is folded, so a fold has nothing to move: this keeps the rows the card last showed open and plays them out from that
// copy, each by the core's pose for its place (`RowMotion.tables.foldPose`: Drawer closes the height while the rows fade, Tuck
// lifts and fades them from the bottom up and then closes the height); opening plays the rows the core now lists in. One linear
// run per change, the pose of each row asked of the core every frame. Classic's fold is a cut, and Reduce Motion cuts them all.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct GroupRows<Row: View>: View {
    let section: SidebarSection
    let head: GroupHead
    @ViewBuilder let row: (SidebarRow) -> Row
    @Environment(\.sidebarLook) private var look
    @Environment(\.reduceMotion) private var reduceMotion
    /// The rows of the card while it was open: what a fold plays out.
    @State private var kept: [SidebarRow] = []
    /// Whether the card was folded when its last motion ended; a head that says otherwise is mid-motion.
    @State private var settled: Bool
    /// Where the run stands: 0...1 opening (1 is the rows at rest), 1...2 folding (the pose's going phase).
    @State private var progress: Double
    @State private var run = 0

    init(section: SidebarSection, head: GroupHead, @ViewBuilder row: @escaping (SidebarRow) -> Row) {
        (self.section, self.head, self.row) = (section, head, row)
        _settled = State(initialValue: head.isFolded)
        _progress = State(initialValue: head.isFolded ? 0 : 1)
    }

    var body: some View {
        let moving = settled != head.isFolded
        // Folding plays the rows kept from when it was open; opening and rest show the core's.
        let rows = moving && head.isFolded ? kept : (head.isFolded ? [] : section.rows)
        let variant = look.foldMotion, count = rows.count
        VStack(spacing: 0) {
            ForEach(Array(rows.enumerated()), id: \.element.id) { index, item in
                if moving, let tables = RowMotion.tables {
                    row(item).modifier(RowPoseEffect(progress: progress) { coming, going in
                        coming.map { tables.foldPose(variant, true, $0, index, count) }
                            ?? tables.foldPose(variant, false, going ?? 1, index, count)
                    })
                } else {
                    row(item)
                }
            }
        }
        .onChange(of: section.rows, initial: true) { _, rows in if !head.isFolded { kept = rows } }
        .onChange(of: head.isFolded) { _, folded in start(folding: folded) }
    }

    private func start(folding: Bool) {
        let count = folding ? kept.count : section.rows.count
        guard !reduceMotion, count > 0, let tables = RowMotion.tables,
            case let seconds = tables.foldPlan(look.foldMotion, !folding, count).seconds, seconds > 0
        else { return rest(folded: folding) }
        run += 1
        let mine = run
        progress = folding ? 1 : 0
        // The first frame of the run is seen before the run is: the next turn of the main loop.
        Task { @MainActor in
            withAnimation(.linear(duration: seconds)) { progress = folding ? 2 : 1 }
            try? await Task.sleep(for: .seconds(seconds))
            if run == mine { rest(folded: folding) }
        }
    }

    /// The motion is over (or none played): the rows are at rest as the core lists them.
    private func rest(folded: Bool) {
        var cut = Transaction()
        cut.disablesAnimations = true
        withTransaction(cut) {
            settled = folded
            progress = folded ? 0 : 1
        }
    }
}
