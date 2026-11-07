#if WITH_UNIFFI
import AppKit
import Foundation
import Testing
import TerminalCore

@testable import TerminalMetal

/// kitty, iTerm2 and Sixel pictures from a real shell through the real engine, drawn by the Metal view and read back.
/// With `WORMHOLE_VISUAL_DIR` set the frame is also written there as a PNG, to be looked at.
@MainActor
@Suite(.serialized) struct LiveImageTests {
    /// A 4 x 2 picture: red, green, blue, yellow / white, black, cyan, magenta; raw RGB and as a PNG.
    private static let raw = "/wAAAP8AAAD///8A////AAAAAP///wD/"
    private static let png = "iVBORw0KGgoAAAANSUhEUgAAAAQAAAACCAIAAADwyuo0AAAAFUlEQVR4nGP4z8DAAMb//0MJCPUfAIuOC/Wv5XZ8AAAAAElFTkSuQmCC"
    private static let colours: [[(UInt8, UInt8, UInt8)]] = [
        [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0)],
        [(255, 255, 255), (0, 0, 0), (0, 255, 255), (255, 0, 255)],
    ]

    /// Bilinear sampling a pixel off a texel's centre blends a little of its neighbours: equal within a few levels.
    private func near(_ a: (r: UInt8, g: UInt8, b: UInt8), _ b: (UInt8, UInt8, UInt8)) -> Bool {
        abs(Int(a.r) - Int(b.0)) <= 12 && abs(Int(a.g) - Int(b.1)) <= 12 && abs(Int(a.b) - Int(b.2)) <= 12
    }

    private func script() -> String {
        // kitty: 8 x 2 cells (both given, so it fills them). iTerm2: the same size in cells, shape not kept.
        // Sixel: eight columns of a red band.
        [
            "printf '\\033_Ga=T,f=24,s=4,v=2,i=1,c=8,r=2,q=2;\(Self.raw)\\033\\\\'",
            "printf '\\r\\n'",
            "printf '\\033]1337;File=inline=1;width=8;height=2;preserveAspectRatio=0:\(Self.png)\\a'",
            "printf '\\r\\n'",
            "printf '\\033Pq#1;2;100;0;0!16~\\033\\\\'",
            "printf '\\r\\ndone\\r\\n'",
        ].joined(separator: "; ")
    }

    @Test func threeProtocolsDrawTheirPictures() async throws {
        let live = try LiveHarness(script(), cols: 40, rows: 12, dataDirectory: "/tmp/Wormhole-swift-n-images")
        #expect(await live.wait(for: "done"))
        // The pictures are decoded off the PTY thread: draw until the three have textures.
        for _ in 0..<250 where live.view.renderer.images.textures.count < 3 {
            live.view.drawFrame(force: true)
            await withCheckedContinuation { done in DispatchQueue.main.asyncAfter(deadline: .now() + 0.02) { done.resume() } }
        }
        let placements = live.view.renderer.images.placements
        #expect(placements.count == 3, "\(placements)")
        live.view.drawFrame(force: true)
        let image = live.view.renderer.renderToImage()
        if let directory = ProcessInfo.processInfo.environment["WORMHOLE_VISUAL_DIR"] {
            try FileManager.default.createDirectory(atPath: directory, withIntermediateDirectories: true)
            try image.writePNG(to: URL(fileURLWithPath: directory).appendingPathComponent("three-protocols.png"))
        }
        let cell = live.view.renderer.cellPixels
        // The two pictures with both sizes given: the texel at (column, row) of the 4 x 2 picture is the colour at
        // the middle of its two cells.
        for placement in placements.filter({ $0.cols == 8 && $0.rows == 2 }) {
            for (row, line) in Self.colours.enumerated() {
                for (column, expected) in line.enumerated() {
                    let x = placement.column * cell.width + (2 * column + 1) * cell.width
                    let y = (placement.line * cell.height) + row * cell.height + cell.height / 2
                    let got = image.pixel(x, y)
                    #expect(near(got, expected), "texel \(column),\(row) of the picture at line \(placement.line): \(got)")
                }
            }
        }
        #expect(placements.filter({ $0.cols == 8 && $0.rows == 2 }).count == 2)
        // The Sixel: red where a band was drawn, the background beside it.
        let sixel = try #require(placements.first { $0.rows < 2 || $0.cols != 8 })
        let red = image.pixel(sixel.column * cell.width + 2, sixel.line * cell.height + 2)
        #expect(near(red, (255, 0, 0)))
    }

    @Test func aPictureFollowsTheTextIntoTheHistory() async throws {
        let script = "printf '\\033_Ga=T,f=24,s=4,v=2,i=1,c=8,r=2,q=2;\(Self.raw)\\033\\\\'; printf '\\r\\n\\r\\n'; i=0; while [ $i -lt 14 ]; do printf \"row $i\\r\\n\"; i=$((i+1)); done; printf 'done\\r\\n'"
        let live = try LiveHarness(script, cols: 40, rows: 12, dataDirectory: "/tmp/Wormhole-swift-n-images")
        #expect(await live.wait(for: "done"))
        // Print and Export read the engine's own text: the screen without the scrolled-off lines, the history with them.
        #expect(live.view.source.plainText(history: false)?.contains("row 0\n") == false)
        #expect(live.view.source.plainText(history: true)?.contains("row 0\nrow 1\n") == true)
        // 17 lines on 12 rows: the picture is above the live screen, so no placement reaches the view.
        live.view.drawFrame(force: true)
        #expect(live.view.renderer.images.placements.isEmpty)
        // Scrolled back to the top of the history it is placed again, on the first rows.
        live.view.source.scroll(lines: 100)
        for _ in 0..<50 where live.view.renderer.images.placements.isEmpty {
            live.view.drawFrame(force: true)
            await withCheckedContinuation { done in DispatchQueue.main.asyncAfter(deadline: .now() + 0.02) { done.resume() } }
        }
        let placed = try #require(live.view.renderer.images.placements.first)
        let image = live.view.renderer.renderToImage()
        let cell = live.view.renderer.cellPixels
        // The history's top row is the picture's own: red where the first texel is.
        let red = image.pixel(cell.width, cell.height / 2)
        #expect(placed.line + live.view.renderer.images.displayOffset == 0)
        #expect(near(red, (255, 0, 0)))
    }
}
#endif
