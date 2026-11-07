import AppKit
import Testing
import TerminalCore

@testable import TerminalMetal

@MainActor
@Suite struct KeyboardTests {
    private func row(_ harness: ViewHarness, _ row: Int = 0) -> String {
        harness.view.drawFrame(force: true)
        return harness.view.renderer.text(ofRow: row).trimmingCharacters(in: .whitespaces)
    }

    private func key(_ characters: String, code: UInt16, flags: NSEvent.ModifierFlags = []) -> NSEvent {
        NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: flags, timestamp: 0, windowNumber: 0, context: nil, characters: characters,
            charactersIgnoringModifiers: characters, isARepeat: false, keyCode: code)!
    }

    @Test func composingShowsTheTextAtTheCursorAndCommittingTypesIt() throws {
        let harness = try ViewHarness()
        let before = harness.view.renderer.renderToImage()
        harness.view.setMarkedText("にほん", selectedRange: NSRange(location: 3, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
        harness.view.drawFrame(force: true)
        #expect(harness.view.hasMarkedText() && harness.view.markedRange() == NSRange(location: 0, length: 3))
        #expect(harness.view.renderer.renderToImage().bgra != before.bgra, "the composed text is drawn")
        #expect(row(harness).isEmpty, "but it is not in the screen: nothing was typed")

        harness.view.insertText("日本", replacementRange: NSRange(location: NSNotFound, length: 0))
        #expect(!harness.view.hasMarkedText())
        #expect(row(harness) == "日本")
    }

    @Test func aDeadKeyComposesAnAccentThenCommitsIt() throws {
        let harness = try ViewHarness()
        harness.view.setMarkedText("´", selectedRange: NSRange(location: 1, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
        harness.view.insertText("é", replacementRange: NSRange(location: NSNotFound, length: 0))
        #expect(row(harness) == "é")
    }

    @Test func theSystemCanReadBackWhatIsComposedForReconversionAndDictation() throws {
        let harness = try ViewHarness()
        #expect(harness.view.selectedRange() == NSRange(location: 0, length: 0), "nothing composed: a valid empty selection")
        harness.view.setMarkedText(NSAttributedString(string: "かな"), selectedRange: NSRange(location: 0, length: 2), replacementRange: NSRange(location: NSNotFound, length: 0))
        var actual = NSRange()
        #expect(harness.view.attributedSubstring(forProposedRange: NSRange(location: 0, length: 9), actualRange: &actual)?.string == "かな")
        #expect(actual == NSRange(location: 0, length: 2))
        #expect(harness.view.selectedRange() == NSRange(location: 0, length: 2), "the clause being converted")
        harness.view.unmarkText()
        #expect(!harness.view.hasMarkedText())
    }

    @Test func theCandidateWindowOpensAtTheCursorsCell() throws {
        let harness = try ViewHarness(text: "ab")
        let rect = harness.view.firstRect(forCharacterRange: NSRange(location: 0, length: 0), actualRange: nil)
        #expect(rect.width == harness.cell.width && rect.height == harness.cell.height)
    }

    @Test func anOrdinaryKeyGoesThroughTheInputMethodAndIsTyped() throws {
        let harness = try ViewHarness()
        harness.view.keyDown(with: key("h", code: 4))
        harness.view.keyDown(with: key("i", code: 34))
        #expect(row(harness) == "hi")
    }

    @Test func aKeyThatTypesNothingGoesStraightToTheEngine() throws {
        let harness = try ViewHarness(text: "abc")
        harness.view.keyDown(with: key("\u{7F}", code: 51))
        #expect(row(harness) == "ab", "backspace")
    }

    @Test func theBellGlowsBrieflyAndOnlyAsAskedFor() async throws {
        let harness = try ViewHarness()
        harness.view.bell = BellStyle(sound: false, visual: true, dock: false)
        harness.view.ringBell()
        #expect(harness.view.glowLayer?.isHidden == false)
        #expect(harness.view.glowLayer?.borderWidth == 2)
        await withCheckedContinuation { done in DispatchQueue.main.asyncAfter(deadline: .now() + 0.4) { done.resume() } }
        #expect(harness.view.glowLayer?.isHidden == true, "gone after 150 ms")

        let quiet = try ViewHarness()
        quiet.view.bell = BellStyle(sound: false, visual: false, dock: false)
        quiet.view.ringBell()
        #expect(quiet.view.glowLayer == nil)
    }
}
