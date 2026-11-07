import Foundation
import Metal
import Testing
import TerminalCore

@testable import TerminalMetal

/// The frame's placements section, the quad each placement becomes, and the pictures through the real Metal
/// pipeline (read back from a texture, no window).
@MainActor
@Suite struct InlineImageTests {
    private let cols = 20, rows = 8
    private let red = (r: UInt8(255), g: UInt8(0), b: UInt8(0))

    // MARK: the section

    private func writer(changed: Int = 0, placements: [ImagePlacement]? = nil) -> Data {
        var data = FrameWriter(
            header: FrameHeader(cols: cols, rows: rows, changedRows: changed, isFull: false, cursor: nil), rowCapacity: 0
        ).bytes
        if let placements { PlacementRecords.append(placements, to: &data) }
        return data
    }

    private func read(_ data: Data) throws -> [ImagePlacement]? {
        try data.withUnsafeBytes { bytes in try PlacementRecords.read(bytes, header: try FrameReader.header(bytes)) }
    }

    @Test func placementsRoundTripAndAFrameWithoutTheFlagSaysAsBefore() throws {
        let list = [
            ImagePlacement(key: 7, z: -2, line: -3, column: 4, cols: 5, rows: 2, source: ImageRect(x: 1, y: 2, width: 30, height: 40), offsetX: 6, offsetY: 7, extent: .pixels(width: 90, height: 50)),
            ImagePlacement(key: 8, line: 1, column: 0, cols: 2, rows: 1, extent: .fit),
            ImagePlacement(key: 9, z: -1_073_741_825, line: 0, column: 0, cols: 1, rows: 1),
        ]
        #expect(try read(writer(placements: list)) == list)
        #expect(try read(writer(placements: [])) == [])
        #expect(try read(writer()) == nil)
    }

    @Test func aSectionCutShortIsRefused() throws {
        var data = writer(placements: [ImagePlacement(key: 1, line: 0, column: 0, cols: 1, rows: 1)])
        data.removeLast(8)
        #expect(throws: FrameError.truncated) { try read(data) }
    }

    @Test func theBandsFollowKittysZ() {
        #expect(ImageLayer.band(of: -1_073_741_824) == .belowBackgrounds)
        #expect(ImageLayer.band(of: -1_073_741_823) == .belowText)
        #expect(ImageLayer.band(of: -1) == .belowText)
        #expect(ImageLayer.band(of: 0) == .aboveText)
    }

    // MARK: geometry (the cases of `render/images.rs`)

    private let surface = ImageSurface(cellWidth: 8, cellHeight: 16, scale: 2, width: 160, height: 128)

    private func quad(_ placement: ImagePlacement, offset: Int = 0, image: (Int, Int) = (16, 32)) -> ImageQuad? {
        ImageLayout.quad(placement, imageWidth: image.0, imageHeight: image.1, displayOffset: offset, surface: surface)
    }

    @Test func aFilledPictureCoversItsCells() throws {
        let q = try #require(quad(ImagePlacement(key: 1, line: 1, column: 2, cols: 2, rows: 2)))
        #expect((q.x, q.y, q.width, q.height) == (16, 16, 16, 32))
        #expect((q.u0, q.v0, q.u1, q.v1) == (0, 0, 1, 1))
    }

    @Test func aPictureScrolledPartlyUpIsCutAtTheGridAndKeepsItsPlace() throws {
        // Its first row is in the history: only the second shows, and only the lower half of the texture.
        let q = try #require(quad(ImagePlacement(key: 1, line: -1, column: 0, cols: 2, rows: 2)))
        #expect((q.x, q.y, q.width, q.height) == (0, 0, 16, 16))
        #expect((q.v0, q.v1) == (0.5, 1))
        #expect(quad(ImagePlacement(key: 1, line: -5, column: 0, cols: 2, rows: 2)) == nil)
        // The display offset brings it back into view.
        #expect(quad(ImagePlacement(key: 1, line: -5, column: 0, cols: 2, rows: 2), offset: 5) != nil)
        // Cut at the right and bottom edges too.
        let edge = try #require(quad(ImagePlacement(key: 1, line: 7, column: 19, cols: 2, rows: 2)))
        #expect((edge.width, edge.height, edge.u1, edge.v1) == (8, 16, 0.5, 0.5))
    }

    @Test func aPictureAtItsPixelSizeIsNotStretchedAndHonoursTheOffsetInPoints() throws {
        // 4 x 8 points at scale 2, nudged 2 and 3 points: 8 x 16 device pixels from (4, 6).
        let p = ImagePlacement(key: 1, line: 0, column: 0, cols: 1, rows: 1, offsetX: 2, offsetY: 3, extent: .pixels(width: 4, height: 8))
        let q = try #require(quad(p))
        #expect((q.x, q.y, q.width, q.height) == (4, 6, 8, 16))
    }

    @Test func aFittedPictureKeepsItsShapeCentredInTheCells() throws {
        // 4 x 2 cells are 32 x 32; a 64 x 16 picture fits the width at 32 x 8, centred vertically.
        let p = ImagePlacement(key: 1, line: 0, column: 0, cols: 4, rows: 2, extent: .fit)
        let q = try #require(quad(p, image: (64, 16)))
        #expect((q.x, q.y, q.width, q.height) == (0, 12, 32, 8))
    }

    @Test func aPartOfThePictureShowsOnlyThatPart() throws {
        let p = ImagePlacement(key: 1, line: 0, column: 0, cols: 2, rows: 2, source: ImageRect(x: 8, y: 0, width: 8, height: 32))
        let q = try #require(quad(p))
        #expect((q.u0, q.u1, q.v0, q.v1) == (0.5, 1, 0, 1))
    }

    // MARK: through the pipeline

    /// Bilinear sampling a pixel off a texel's centre blends a little of its neighbours: equal within a few levels.
    private func near(_ a: (r: UInt8, g: UInt8, b: UInt8), _ b: (r: UInt8, g: UInt8, b: UInt8)) -> Bool {
        abs(Int(a.r) - Int(b.r)) <= 12 && abs(Int(a.g) - Int(b.g)) <= 12 && abs(Int(a.b) - Int(b.b)) <= 12
    }

    private func renderer() throws -> MetalRenderer {
        try MetalRenderer(configuration: RendererConfiguration(font: .system(family: "Menlo"), pointSize: 13, scale: 2))
    }

    private func rowsFrame(background cells: [(row: Int, column: Int)] = [], placements: [ImagePlacement]) -> Data {
        var writer = FrameWriter(
            header: FrameHeader(cols: cols, rows: rows, changedRows: rows, isFull: true, cursor: nil), rowCapacity: rows)
        for row in 0..<rows {
            var line = Array(repeating: PackedCell(), count: cols)
            for cell in cells where cell.row == row { line[cell.column].bg = PackedColor.rgb(0, 0, 255) }
            line.withUnsafeBufferPointer { writer.appendRow(index: row, size: .single, cells: $0) }
        }
        var data = writer.bytes
        PlacementRecords.append(placements, to: &data)
        return data
    }

    /// A 2 x 2 picture: red, green / blue, half-transparent white.
    private let picture = InlineImage(
        width: 2, height: 2,
        rgba: Data([255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 0]))

    @Test func aPlacedPictureIsDrawnOverItsCellsAndOnlyThere() throws {
        let renderer = try renderer()
        renderer.images.provider = { _ in picture }
        try renderer.apply(frame: rowsFrame(placements: [ImagePlacement(key: 1, line: 1, column: 2, cols: 4, rows: 2)]))
        let image = renderer.renderToImage()
        let cell = renderer.cellPixels
        // The red texel is the top-left quarter of the 4 x 2 cells.
        #expect(near(image.pixel(2 * cell.width + cell.width, 1 * cell.height + cell.height / 2), red))
        // The green one the top-right, the blue the bottom-left.
        #expect(near(image.pixel(5 * cell.width, cell.height + cell.height / 2), (0, 255, 0)))
        #expect(near(image.pixel(3 * cell.width, 2 * cell.height + cell.height / 2), (0, 0, 255)))
        let background = TerminalPalette.spike.background
        #expect(image.pixel(cell.width, cell.height + cell.height / 2) == (background.x, background.y, background.z), "left of it")
        #expect(image.pixel(7 * cell.width, cell.height + cell.height / 2) == (background.x, background.y, background.z), "right of it")
    }

    @Test func aTransparentTexelShowsTheBackgroundAndTheBandsStackByZ() throws {
        let renderer = try renderer()
        renderer.images.provider = { _ in picture }
        let blue = [(row: 1, column: 2), (row: 1, column: 3)]
        let cell = renderer.cellPixels
        func pixel(z: Int32) throws -> (r: UInt8, g: UInt8, b: UInt8) {
            try renderer.apply(frame: rowsFrame(background: blue, placements: [ImagePlacement(key: 1, z: z, line: 1, column: 2, cols: 2, rows: 1)]))
            return renderer.renderToImage().pixel(2 * cell.width + cell.width / 2, cell.height + cell.height / 4)
        }
        #expect(near(try pixel(z: 0), red), "over the cells' colour")
        #expect(near(try pixel(z: -1), red), "between the backgrounds and the text")
        #expect(near(try pixel(z: -1_073_741_824), (0, 0, 255)), "below the backgrounds: covered by them")
        // The bottom-right texel is fully transparent: the cell's own background shows through it.
        renderer.images.provider = { _ in InlineImage(width: 1, height: 1, rgba: Data([255, 255, 255, 0])) }
        try renderer.apply(frame: rowsFrame(background: blue, placements: [ImagePlacement(key: 2, line: 1, column: 2, cols: 1, rows: 1)]))
        #expect(renderer.renderToImage().pixel(2 * cell.width + cell.width / 2, cell.height + cell.height / 2) == (0, 0, 255))
    }

    @Test func aPictureStillDecodingIsAskedForAgainOnTheNextFrameAndNotBefore() throws {
        let renderer = try renderer()
        var asked = 0
        var ready = false
        renderer.images.provider = { _ in
            asked += 1
            return ready ? picture : nil
        }
        let list = [ImagePlacement(key: 5, line: 0, column: 0, cols: 2, rows: 1)]
        try renderer.apply(frame: rowsFrame(placements: list))
        #expect(renderer.images.isWaiting && !renderer.images.textures.has(5))
        ready = true
        // A frame that says "as before" still looks again.
        var again = FrameWriter(header: FrameHeader(cols: cols, rows: rows, changedRows: 0, isFull: false, cursor: nil), rowCapacity: 0).bytes
        try renderer.apply(frame: again)
        again = Data()
        #expect(!renderer.images.isWaiting && renderer.images.textures.has(5))
        #expect(asked == 2)
        // Once it has a texture it is not asked for again.
        try renderer.apply(frame: rowsFrame(placements: list))
        #expect(asked == 2)
    }

    @Test func theLeastRecentlyUsedTextureGoesFirstAndAPlacedOneStays() throws {
        let device = try #require(MTLCreateSystemDefaultDevice())
        let textures = ImageTextures(device: device, budget: 8)
        let pixels = Data(count: 4)
        for key in 1...3 { #expect(textures.insert(InlineImage(width: 1, height: 1, rgba: pixels), key: UInt32(key))) }
        #expect(!textures.insert(InlineImage(width: 2, height: 2, rgba: pixels), key: 9), "pixels that do not match the size")
        _ = textures.texture(for: 1)
        textures.trim(keeping: [])
        #expect(textures.has(1) && textures.has(3) && !textures.has(2) && textures.bytes == 8)
        textures.insert(InlineImage(width: 1, height: 1, rgba: pixels), key: 4)
        textures.trim(keeping: [1, 3, 4])
        #expect(textures.count == 3, "over the budget, but every one of them is placed: none may go")
        textures.trim(keeping: [4])
        #expect(!textures.has(3) && textures.has(1) && textures.has(4), "3 is the least recently used of the unplaced")
    }
}
