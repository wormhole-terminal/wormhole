// What the terminal's state draws beside and over the grid: find's ticks, each command block's gutter mark and the hovered block's
// tint and readout, and the minimap along the right edge (which takes the scroll bar's place and keeps the grid out
// of its width). One view of its own so that the frame-by-frame reads (`overlay.revision`) redraw only these, not the
// whole pane.

import DesignSystem
import SwiftUI
import TerminalMetal
import TerminalUI

struct TerminalMarks: View {
    let overlay: TerminalOverlay
    let showsMinimap: Bool

    var body: some View {
        // Read after every frame that rebuilt rows: the spans and the minimap are the engine's, as of that frame.
        let _ = (overlay.revision, overlay.find.outcome)
        ZStack(alignment: .bottomTrailing) {
            // Find's ticks beside the grid, where the matches are along the scrollback; they stand left of the minimap.
            FindTicks(positions: overlay.find.ticks).padding(.trailing, minimapWidth)
            if let geometry = overlay.blockGeometry {
                let spans = overlay.blocks.spans
                BlockMarks(spans: spans, hovered: overlay.blocks.hovered, geometry: geometry)
                if let id = overlay.blocks.hovered, let span = spans.first(where: { $0.id == id }) {
                    BlockReadout(controller: overlay.blocks, id: id, state: span.state)
                        .padding(Space._2)
                        .padding(.trailing, minimapWidth + MetalRenderer.padding)
                }
            }
            if minimapShown {
                // The pane's padding above and below, as the GPUI minimap sits (`render/minimap.rs`: top and bottom PADDING_Y).
                MinimapView(source: overlay.minimap, revision: overlay.revision)
                    .padding(.vertical, MetalRenderer.padding)
                    .frame(maxHeight: .infinity)
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
        }
        .onChange(of: minimapShown, initial: true) { _, _ in overlay.view?.minimapWidth = minimapWidth }
    }

    private var minimapShown: Bool { showsMinimap && overlay.hasScrollback }
    private var minimapWidth: CGFloat { minimapShown ? MinimapGeometry.width : 0 }
}
