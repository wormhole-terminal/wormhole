import CoreGraphics
import Foundation
import ImageIO
import Testing
import UniformTypeIdentifiers

/// The Files viewer read with `files-viewer [from] [count]` (`Readout`'s shape) after `files-open <path>`: the lines a text
/// is cut into, a picture's size or the card that refuses it, a link out of the folder, the colours that follow the text.
/// TO-VERIFY.md lines 244, 245, 418, 583.
extension EndToEnd {
    /// The viewer once it shows `path` with a kind other than `reading` and `done` holds.
    @MainActor static func viewer(_ app: Instance, _ path: String, _ done: @escaping (Readout) -> Bool = { _ in true }) async -> Readout {
        await app.send(["files-open \(path)"])
        return Readout(await app.until("files-viewer 0 400") { said in
            let head = Readout(said).first("files-viewer")
            return head?.label == path && head?["kind"] != "reading" && done(Readout(said))
        })
    }

    /// A PNG that only declares its size (a header and an end, with their checksums): enough for its facts to be read.
    static func declaredPNG(width: UInt32, height: UInt32) -> Data {
        func crc(_ bytes: [UInt8]) -> UInt32 {
            var c: UInt32 = 0xFFFF_FFFF
            for byte in bytes {
                c ^= UInt32(byte)
                for _ in 0..<8 { c = (c & 1) != 0 ? 0xEDB8_8320 ^ (c >> 1) : c >> 1 }
            }
            return ~c
        }
        func be(_ v: UInt32) -> [UInt8] { [UInt8(v >> 24 & 0xFF), UInt8(v >> 16 & 0xFF), UInt8(v >> 8 & 0xFF), UInt8(v & 0xFF)] }
        func chunk(_ type: String, _ data: [UInt8]) -> [UInt8] { be(UInt32(data.count)) + Array(type.utf8) + data + be(crc(Array(type.utf8) + data)) }
        let header = be(width) + be(height) + [8, 2, 0, 0, 0]
        return Data([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] + chunk("IHDR", header) + chunk("IEND", []))
    }

    /// A real picture `width` wide, as ImageIO writes a PNG (`gray`: one flat channel, which keeps a very large one small).
    static func realPNG(width: Int, height: Int, gray: Bool = false) -> Data? {
        let space = gray ? CGColorSpaceCreateDeviceGray() : CGColorSpaceCreateDeviceRGB()
        let alpha = gray ? CGImageAlphaInfo.none : CGImageAlphaInfo.premultipliedLast
        guard let context = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0, space: space, bitmapInfo: alpha.rawValue),
            let image = { context.setFillColor(red: 0.2, green: 0.4, blue: 0.8, alpha: 1); context.fill(CGRect(x: 0, y: 0, width: width, height: height)); return context.makeImage() }()
        else { return nil }
        let out = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(out, UTType.png.identifier as CFString, 1, nil) else { return nil }
        CGImageDestinationAddImage(destination, image, nil)
        return CGImageDestinationFinalize(destination) ? out as Data : nil
    }

    @Test("a CRLF file is one numbered line per line with no carriage return; an LF file ending in a newline has no empty last line (244)")
    @MainActor func viewerLineEnds() async throws {
        try await Instance.with { app in
            // The panel's tabs are features behind their flags, which a window follows: on for what this scenario shows.
            await app.flags(["files": true])
            let repo = try app.scratch.repository("ends-repo", files: ["crlf.txt": "one\r\ntwo\r\nthree\r\n", "lf.txt": "a\nb\n"])
            await app.terminal(in: repo)
            await app.send(["tab files"])
            await app.until("files") { $0.contains("crlf.txt") }
            let crlf = await Self.viewer(app, "crlf.txt")
            app.expect(crlf.first("files-viewer")?["lines"] == "3", "three lines:\n\(crlf.text)")
            app.expect(crlf.all("line").allSatisfy { !($0["text"] ?? "").contains("\r") }, "no carriage return in a line")
            let lf = await Self.viewer(app, "lf.txt")
            app.expect(lf.first("files-viewer")?["lines"] == "2", "no empty last line:\n\(lf.text)")
        }
    }

    @Test("a large picture is drawn at most 4,096 px; one over a hundred million pixels is too large; a broken one can't be shown (245)")
    @MainActor func viewerPictures() async throws {
        try await Instance.with { app in
            // The panel's tabs are features behind their flags, which a window follows: on for what this scenario shows.
            await app.flags(["files": true])
            let repo = try app.scratch.repository("pics-repo")
            // The size is read from the header ImageIO accepts, so the picture is a real one, as in the GPUI app's own test
            // (`platform/macos/image/tests.rs`): 100,010,000 pixels of one grey. A header alone is no picture to ImageIO.
            let large = try #require(Self.realPNG(width: 10_001, height: 10_000, gray: true))
            try large.write(to: repo.appendingPathComponent("huge.png"))
            try Self.declaredPNG(width: 60_000, height: 60_000).write(to: repo.appendingPathComponent("declared.png"))
            try Data([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4]).write(to: repo.appendingPathComponent("broken.png"))
            if let wide = Self.realPNG(width: 5_000, height: 40) { try wide.write(to: repo.appendingPathComponent("wide.png")) }
            await app.terminal(in: repo)
            await app.send(["tab files"])
            await app.until("files") { $0.contains("wide.png") }
            let huge = await Self.viewer(app, "huge.png") { $0.first("note")?.label != "Reading…" }
            app.expect(huge.first("note")?.label == "This picture is too large to show.", "refused:\n\(huge.text)")
            app.expect(huge.all("button").count == 2, "with Open in… and Reveal:\n\(huge.text)")
            let broken = await Self.viewer(app, "broken.png") { $0.first("note")?.label != "Reading…" }
            app.expect(broken.first("note")?.label.contains("can't be shown") == true || broken.first("files-viewer")?["kind"] == "binary", "not shown:\n\(broken.text)")
            let declared = await Self.viewer(app, "declared.png") { $0.first("note")?.label != "Reading…" }
            app.expect(declared.first("note")?.label.contains("can't be shown") == true && declared.first("picture") == nil, "a header alone is not drawn:\n\(declared.text)")
            let wide = await Self.viewer(app, "wide.png") { $0.first("picture") != nil }
            app.expect(Int(wide.first("picture")?["width"] ?? "") ?? 0 <= 4_096, "scaled down:\n\(wide.text)")
        }
    }

    @Test("a link that leads out of the folder is not shown and offers neither Open in nor Reveal (418)")
    @MainActor func viewerLinkOut() async throws {
        try await Instance.with { app in
            // The panel's tabs are features behind their flags, which a window follows: on for what this scenario shows.
            await app.flags(["files": true])
            let repo = try app.scratch.repository("link-repo")
            let outside = app.scratch.root.appendingPathComponent("outside.txt")
            try Data("secret\n".utf8).write(to: outside)
            try FileManager.default.createSymbolicLink(at: repo.appendingPathComponent("out.txt"), withDestinationURL: outside)
            await app.terminal(in: repo)
            await app.send(["tab files"])
            await app.until("files") { $0.contains("out.txt") }
            let shown = await Self.viewer(app, "out.txt")
            app.expect(shown.first("files-viewer")?["kind"] == "unavailable-outside", "refused as a link out:\n\(shown.text)")
            app.expect(shown.all("button").isEmpty && !shown.text.contains("secret"), "nothing to open, nothing shown:\n\(shown.text)")
        }
    }

    @Test("a Rust file's text shows with its colours after it; a 50,000-line file shows every line (583)")
    @MainActor func viewerColoursAndLength() async throws {
        try await Instance.with { app in
            // The panel's tabs are features behind their flags, which a window follows: on for what this scenario shows.
            await app.flags(["files": true])
            let long = (1...50_000).map { "line \($0)" }.joined(separator: "\n") + "\n"
            let repo = try app.scratch.repository("view-repo", files: ["lib.rs": "fn main() {\n    let x = \"hi\";\n}\n", "long.txt": long])
            await app.terminal(in: repo)
            await app.send(["tab files"])
            await app.until("files") { $0.contains("lib.rs") }
            let rust = await Self.viewer(app, "lib.rs") { $0.all("line").contains { !($0["spans"] ?? "").isEmpty } }
            app.expect(rust.all("line").first?["spans"]?.contains("keyword") == true, "fn is a keyword:\n\(rust.text)")
            let text = await Self.viewer(app, "long.txt")
            app.expect(text.first("files-viewer")?["lines"] == "50000" && text.first("files-viewer")?["digits"] == "5", "every line, five digits:\n\(text.first("files-viewer")?.text ?? "")")
        }
    }
}
