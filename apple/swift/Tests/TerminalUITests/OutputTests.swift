import AppKit
import Foundation
import Testing

@testable import TerminalUI

@MainActor @Suite struct OutputTests {
    @Test func theFileIsNamedAfterTheTerminalWithoutPathSeparators() {
        #expect(TextExporter.suggestedName(title: "api: ~/code/server") == "api- ~-code-server.txt")
        #expect(TextExporter.suggestedName(title: "  ") == "Terminal.txt")
    }

    @Test func theFileEndsItsLastLine() {
        #expect(TextExporter.data(for: "a\nb") == Data("a\nb\n".utf8))
        #expect(TextExporter.data(for: "a\n") == Data("a\n".utf8))
    }

    @Test func thePrintViewWrapsAtThePageAndKeepsTheText() {
        let info = NSPrintInfo()
        let view = PrintableText.view(text: String(repeating: "x", count: 400) + "\nsecond", font: .monospacedSystemFont(ofSize: 10, weight: .regular), info: info)
        #expect(view.string.hasSuffix("second"))
        #expect(view.frame.width == info.paperSize.width - info.leftMargin - info.rightMargin)
        #expect(view.frame.height > 20, "the long line wrapped")
    }

    @Test func theJobIsNamedAfterTheTerminal() {
        let operation = PrintableText.operation(text: "hi", title: "api", font: .systemFont(ofSize: 10), info: NSPrintInfo())
        #expect(operation.jobTitle == "api")
    }
}
