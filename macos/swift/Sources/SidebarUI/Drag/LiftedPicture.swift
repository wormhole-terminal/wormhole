// What follows the pointer while a terminal's row is dragged (`sidebar/motion/lift.rs`): the row itself, lifted out of the list
// as a card of its own with a shadow, held where it was picked up. Over the middle of another row, where the drop makes a
// split, it crossfades (120 ms, ease out) to the compact chip, which snaps on the quick spring below that row's name, and goes
// back the same way when the pointer leaves. Under Reduce Motion the crossfade is a cut.

import DesignSystem
import SwiftUI
import WorkspaceModel

struct LiftedPicture: View {
    let row: SidebarRow
    let context: SidebarContext
    let drag: LiveDrag
    let width: CGFloat
    /// The row the drop would join: where the chip lands.
    let target: CGRect?
    @State private var chipSize = CGSize(width: 0, height: 24)
    @Environment(\.sidebarMetrics) private var metrics
    @Environment(\.reduceMotion) private var reduceMotion

    private var onto: Bool { target != nil }

    var body: some View {
        let origin = CGPoint(x: drag.point.x - drag.grab.x, y: drag.point.y - drag.grab.y)
        ZStack(alignment: .topLeading) {
            TerminalRowView(row: row, isSelected: row.id == context.selected, solo: true, context: context, lifted: true)
                .frame(width: width)
                .opacity(onto ? 0 : 1)
                .animation(crossfade, value: onto)
                .offset(x: origin.x, y: origin.y)
            DragGhost(terminal: row.terminal, title: row.terminal.title)
                .fixedSize()
                .onGeometryChange(for: CGSize.self) { $0.size } action: { chipSize = $0 }
                .opacity(onto ? 1 : 0)
                .animation(crossfade, value: onto)
                .offset(x: chip.x, y: chip.y)
                .animation(reduceMotion ? nil : SidebarMotion.list, value: onto)
        }
    }

    private var crossfade: Animation? { reduceMotion ? nil : SidebarMotion.hint }

    /// The chip's top left: beside the pointer, or on the target row's bottom at the x its name starts, centred on that line.
    private var chip: CGPoint {
        if let target {
            let name = target.minX + metrics.rowInset + metrics.cardPad + metrics.tile + metrics.tileGap
            return CGPoint(x: name, y: target.maxY - chipSize.height / 2)
        }
        let rise = max(drag.grab.y - DragLift.chipRise, 0)
        return CGPoint(x: drag.point.x + DragLift.chipGap, y: drag.point.y - drag.grab.y + rise)
    }
}
