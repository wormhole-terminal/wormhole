// Every piece of the module over its mock, wired as a terminal would wire it, so the gallery renders them in the
// states the GPUI app shows them in. The lead's window wires the same pieces to the real FFI instead.

import Foundation

@MainActor
public final class GalleryModel {
    public let searching = MockSearching()
    public let find: FindModel
    public let blocks = MockBlocks()
    public let controller: BlockController
    public let notices = NoticeCenter()
    public let size = SizeReadoutModel()
    public let minimap = MockMinimap()
    public let actions = MockPaneActions()

    public init() {
        find = FindModel(searcher: searching)
        let notices = notices
        controller = BlockController(source: blocks, say: { notices.post($0) })
    }

    /// Puts every piece in the state a screenshot shows: find open with matches, a block hovered, a copy on the chip,
    /// the grid's size after a live resize.
    public func stage() async {
        find.open(seed: "Compiling")
        find.step(1)
        controller.hover(row: 9)
        controller.perform(.copyOutput, on: blocks.block(id: 2))
        size.noteSize(cols: 100, rows: 30, live: true)
        size.noteSize(cols: 112, rows: 33, live: true)
        try? await Task.sleep(for: .milliseconds(50))
    }
}
