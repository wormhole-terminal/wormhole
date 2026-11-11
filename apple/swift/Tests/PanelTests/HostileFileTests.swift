import CoreGraphics
import ImageIO
import PanelModel
import XCTest

@testable import PanelUI

/// A repository's files are hostile input: how the viewer cuts text into lines and what it will decode of a picture.
final class HostileFileTests: XCTestCase {
    func testLinesAreCutAtLineFeedsWhateverComesBefore() {
        XCTAssertEqual(ViewedLines.cut("a\r\nb\r\n"), ["a", "b"], "CRLF: Swift reads \\r\\n as one Character")
        XCTAssertEqual(ViewedLines.cut("a\nb\n"), ["a", "b"], "a last line break adds no line")
        XCTAssertEqual(ViewedLines.cut("a\n\nb"), ["a", "", "b"])
        XCTAssertEqual(ViewedLines.cut("a\rb"), ["a\rb"], "a bare carriage return is not a line end")
        XCTAssertEqual(ViewedLines.cut("e\u{301}\r\n"), ["e\u{301}"])
        XCTAssertEqual(ViewedLines.cut(""), [""])
    }

    private func png(width: Int, height: Int) throws -> Data {
        let context = try XCTUnwrap(
            CGContext(
                data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue))
        let data = NSMutableData()
        let out = try XCTUnwrap(CGImageDestinationCreateWithData(data, "public.png" as CFString, 1, nil))
        CGImageDestinationAddImage(out, try XCTUnwrap(context.makeImage()), nil)
        XCTAssertTrue(CGImageDestinationFinalize(out))
        return data as Data
    }

    func testAPictureIsScaledDownAndOneThatDeclaresAnAbsurdSizeIsRefused() throws {
        guard case let .shown(small) = FilePicture.decode(try png(width: 8, height: 6)) else { return XCTFail("a small picture shows") }
        XCTAssertEqual([small.width, small.height], [8, 6])
        // The same file with 60,000 x 60,000 written into its header (IHDR's width and height, big-endian, at 16 and 20):
        // the declared size is what is judged, before any pixel is decoded.
        var huge = try png(width: 8, height: 6)
        huge.replaceSubrange(16..<24, with: [0, 0, 0xEA, 0x60, 0, 0, 0xEA, 0x60])
        if case .shown = FilePicture.decode(huge) { XCTFail("3.6 gigapixels must not be decoded") }
        if case .shown = FilePicture.decode(Data("not a picture".utf8)) { XCTFail("bytes that are no picture show nothing") }
    }
}
