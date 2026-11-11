// `--render-png`: no window, no display link, no drawable. The mock is stepped by hand, each step's frame goes
// through the renderer exactly as a view would feed it (so damage and the caches are exercised), and the scene
// is read back from a texture. The same bytes every run for a seed: a PNG is a regression check.

import Foundation
import TerminalCore
import TerminalMetal

@MainActor
enum RenderPNG {
    static func run(options: Options, prefix: String, frames: Int, every: Int) throws {
        guard options.program == nil else {
            throw Options.UsageError(description: "--render-png renders the mock; it takes no --shell")
        }
        let (configuration, note) = options.rendererConfiguration()
        if let note { FileHandle.standardError.write(Data((note + "\n").utf8)) }
        let renderer = try MetalRenderer(configuration: configuration)
        let grid = options.grid ?? (100, 36)
        let cell = renderer.cellPixels
        let size = GridSize(
            cols: grid.cols, rows: grid.rows, cellWidth: Float(Double(cell.width) / renderer.scale),
            cellHeight: Float(Double(cell.height) / renderer.scale))
        let source = MockFrameSource(
            .init(scenario: options.scenario, seed: options.seed, hertz: 0, manual: true), size: size)
        try source.start(size: size, wake: {})
        try renderer.apply(frame: source.frame())

        let directory = URL(fileURLWithPath: prefix).deletingLastPathComponent()
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        var written: [String] = []
        for step in 1...frames {
            source.step()
            let before = renderer.counters
            let rebuilt = try renderer.apply(frame: source.frame())
            if step % every == 0 || step == frames {
                let url = URL(fileURLWithPath: "\(prefix)-\(String(format: "%04d", step)).png")
                try renderer.renderToImage().writePNG(to: url)
                written.append(url.path)
                let delta = renderer.counters - before
                print("step \(step): \(rebuilt) rows rebuilt, \(delta.wordsShaped) words shaped, \(delta.glyphsRasterised) glyphs rasterised -> \(url.lastPathComponent)")
            }
        }
        let cells = renderer.cellPixels
        print("grid \(grid.cols)x\(grid.rows), cell \(cells.width)x\(cells.height) px at \(renderer.scale)x, \(written.count) PNG(s)")
        source.stop()
    }
}
