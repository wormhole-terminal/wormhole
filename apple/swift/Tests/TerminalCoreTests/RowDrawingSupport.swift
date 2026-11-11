import Foundation
import TerminalCore

@testable import TerminalMetal

/// Frames and fonts the drawing tests share: the bundled JetBrains Mono (found from this file up to the
/// repository, as the app finds it), and a frame of rows with per-row sizes.
@MainActor
enum RowDrawing {
    static let size = GridSize(cols: 20, rows: 4)

    static var jetBrainsMono: FontSpec? {
        let relative = "crates/wormhole-core/assets/fonts/JetBrainsMono"
        var directory = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        for _ in 0..<12 {
            let candidate = directory.appendingPathComponent(relative)
            if FileManager.default.fileExists(atPath: candidate.appendingPathComponent("JetBrainsMono-Regular.ttf").path) {
                return .jetBrainsMono(directory: candidate)
            }
            directory.deleteLastPathComponent()
        }
        return nil
    }

    /// Without the FFI these tests draw blocks from a stand-in for the core's shapes: the full block and its halves.
    static let shapes: Void = CellShapes.install(TestShapes())

    private struct TestShapes: CellShapeSource {
        func shapes(_ scalar: UInt32, cellWidth: Float, height: Float, phase: Float, fontSize: Float, scale: Float) -> (shapes: [CellShape], alpha: Float)? {
            let right = (phase + cellWidth).rounded() - phase.rounded(), half = (height / 2).rounded()
            switch scalar {
            case 0x2588: return ([CellShape(rect: [0, 0, right, height])], 1)
            case 0x2580: return ([CellShape(rect: [0, 0, right, half])], 1)
            case 0x2584: return ([CellShape(rect: [0, half, right, height])], 1)
            default: return nil
            }
        }
    }

    static func renderer(_ fonts: TerminalFontSettings = TerminalFontSettings(pointSize: 13)) throws -> MetalRenderer {
        _ = shapes
        return try MetalRenderer(configuration: RendererConfiguration(fonts: fonts, scale: 2))
    }

    static func blank(cols: Int = size.cols, rows: Int = size.rows) -> [[PackedCell]] {
        Array(repeating: Array(repeating: PackedCell(), count: cols), count: rows)
    }

    static func frame(_ cells: [[PackedCell]], sizes: [LineSize] = [], cursor: FrameHeader.Cursor? = nil) -> Data {
        var writer = FrameWriter(
            header: FrameHeader(cols: cells[0].count, rows: cells.count, changedRows: cells.count, isFull: true, cursor: cursor),
            rowCapacity: cells.count)
        for (index, row) in cells.enumerated() {
            row.withUnsafeBufferPointer { writer.appendRow(index: index, size: index < sizes.count ? sizes[index] : .single, cells: $0) }
        }
        return writer.bytes
    }

    /// Whether any pixel of the box differs from the background.
    static func lit(_ image: RenderedImage, x: Range<Int>, y: Range<Int>, background: (UInt8, UInt8, UInt8)) -> Bool {
        y.contains { row in x.contains { column in image.pixel(column, row) != background } }
    }

    static var background: (UInt8, UInt8, UInt8) {
        let b = TerminalPalette.spike.background
        return (b.x, b.y, b.z)
    }
}
