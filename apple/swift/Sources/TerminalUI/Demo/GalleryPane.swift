// A pane the way it is stacked in the window: a terminal-coloured grid of sample lines with the blocks' marks, find's
// ticks, the minimap, and over them the find bar, the block readout, the chip and the size readout.

import DesignSystem
import SwiftUI

struct GalleryPane: View {
    let model: GalleryModel
    @Environment(\.tokens) private var tokens

    /// The sample grid: 24 rows of 18 pt, 4 pt from the top and left (`TERMINAL_PAD`).
    private static let cellHeight: CGFloat = 18
    private static let pad: CGFloat = 4

    var body: some View {
        let geometry = BlockGeometry(rows: 24, cellHeight: Self.cellHeight, top: Self.pad, gutter: Self.pad)
        ZStack(alignment: .topLeading) {
            tokens.terminal.background.color
            VStack(alignment: .leading, spacing: 0) {
                ForEach(0..<24, id: \.self) { row in
                    Text(row < model.searching.lines.count ? model.searching.lines[row] : " ")
                        .font(.system(size: 13, design: .monospaced))
                        .foregroundStyle(tokens.terminal.foreground.color)
                        .frame(height: Self.cellHeight, alignment: .leading)
                }
            }
            .padding(.leading, Self.pad * 2).padding(.top, Self.pad)
            BlockMarks(spans: model.controller.spans, hovered: model.controller.hovered, geometry: geometry)
            FindTicks(positions: model.find.ticks)
            HStack(spacing: 0) {
                Spacer()
                MinimapView(source: model.minimap, revision: 0).padding(.vertical, Self.pad)
            }
            if model.find.isOpen {
                FindBar(model: model.find, onClose: model.find.close).padding(Space._2)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing).padding(.trailing, MinimapGeometry.width)
            }
            if let id = model.controller.hovered, let span = model.controller.spans.first(where: { $0.id == id }) {
                BlockReadout(controller: model.controller, id: id, state: span.state)
                    .padding(Space._2).padding(.trailing, MinimapGeometry.width)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
            }
            SizeReadoutChip(model: model.size)
            NoticeChipHost(center: model.notices)
        }
        .clipShape(RoundedRectangle(cornerRadius: tokens.metrics.radiusLg))
    }
}
