// What the app says beside the pointer or over a terminal (`gallery/ambient.rs`): a column's resize handle with the width
// beside it, the grid's size in a terminal's middle, and a hovered command block's exit code, time and Copy output. The
// handle and its readout are the window's (`GalleryExtras`); the other two the terminal's own views over its samples.
// text-file: sample content of the Theme Studio gallery; the GPUI gallery shows its specimens untranslated too (studio/gallery)

import DesignSystem
import SwiftUI
import TerminalUI

struct ReadoutsSpecimen: View {
    @State private var size = SizeReadoutModel()
    @State private var blocks = BlockController(source: MockBlocks(), say: { _ in })
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: Space._4) {
            line("← the width beside a dragged handle: rest the pointer on the line") { ExtraSpecimen(id: "resize").fixedSize() }
            line("← the grid's size, in a terminal's middle") { SizeReadoutChip(model: size, held: true).fixedSize() }
            line("← a hovered command block") {
                ForEach(MockBlocks.sample, id: \.id) { BlockReadout(controller: blocks, id: $0.id, state: $0.state) }
            }
            .allowsHitTesting(false)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .task {
            // The chip says a size only after a live resize changed it: the first size is never said.
            size.noteSize(cols: 100, rows: 30, live: true)
            size.noteSize(cols: 120, rows: 40, live: true)
        }
    }

    private func line<Content: View>(_ note: String, @ViewBuilder _ content: () -> Content) -> some View {
        HStack(spacing: Space._3) {
            content()
            Text(note).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
        }
    }
}
