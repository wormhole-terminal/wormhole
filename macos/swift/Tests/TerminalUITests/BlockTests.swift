import Foundation
import Testing

@testable import TerminalUI

@MainActor @Suite struct BlockTests {
    private func make() -> (BlockController, MockBlocks, Recorder) {
        let blocks = MockBlocks()
        let recorder = Recorder()
        let controller = BlockController(
            source: blocks, say: { recorder.notices.append($0) }, announce: { recorder.spoken.append($0) },
            write: { recorder.written.append($0) })
        return (controller, blocks, recorder)
    }

    @MainActor final class Recorder {
        var notices: [Notice] = []
        var spoken: [String] = []
        var written: [String] = []
    }

    @Test func tookIsWordedAsTheReadoutSaysIt() {
        #expect(BlockWording.took(.milliseconds(400)) == "0.4s")
        #expect(BlockWording.took(.milliseconds(3500)) == "3.5s")
        #expect(BlockWording.took(.seconds(12)) == "12s")
        #expect(BlockWording.took(.seconds(65)) == "1m 5s")
        #expect(BlockWording.took(.seconds(7380)) == "2h 3m")
    }

    @Test func theReadoutNamesTheCodeAndTheTime() {
        #expect(BlockWording.readout(.finished(code: 101, took: .milliseconds(3500))) == "exit 101 · 3.5s")
        #expect(BlockWording.readout(.finished(code: nil, took: .milliseconds(3500))) == "3.5s")
        #expect(BlockWording.readout(.finished(code: nil, took: nil)) == nil)
        #expect(BlockWording.readout(.running) == nil)
    }

    @Test func voiceOverHearsTheBlockLandedOn() {
        let failed = CommandBlock(id: 9, start: 0, end: 3, command: "cargo build\nsecond line",
                                  state: .finished(code: 101, took: .seconds(42)))
        #expect(BlockWording.spoken(failed).hasPrefix("cargo build, failed with exit code 101, "))
        let empty = CommandBlock(id: 9, start: 0, end: 0, command: "", state: .finished(code: nil, took: nil))
        #expect(BlockWording.spoken(empty) == "empty prompt")
    }

    @Test func aCommandThatRanIsOneWithACodeOrATime() {
        #expect(!BlockState.prompt.ran)
        #expect(BlockState.running.ran)
        #expect(!BlockState.finished(code: nil, took: nil).ran)
        #expect(BlockState.finished(code: 0, took: nil).ran)
    }

    @Test func onlyAFinishedBlockThatRanIsHovered() {
        let (controller, _, _) = make()
        controller.hover(row: 9)
        #expect(controller.hovered == 2)
        controller.hover(row: 19)
        #expect(controller.hovered == nil, "a prompt nothing ran at")
        controller.hover(row: 3)
        controller.readoutHovered = true
        controller.hover(row: 9)
        #expect(controller.hovered == 1, "the readout keeps the block")
        controller.readoutHovered = false
        #expect(controller.hovered == nil)
    }

    @Test func upFromTheLiveScreenGoesToThePromptBeforeTheCurrentOne() {
        let (controller, blocks, recorder) = make()
        controller.jumpPrompt(.up)
        #expect(blocks.topLine == 14)
        #expect(recorder.spoken.count == 1)
        controller.jumpPrompt(.up)
        #expect(blocks.topLine == 6)
        controller.jumpPrompt(.down)
        #expect(blocks.topLine == 14)
    }

    @Test func downPastTheLastPromptReturnsToTheLiveScreen() {
        let (controller, blocks, _) = make()
        controller.jumpPrompt(.up)
        controller.jumpPrompt(.down)
        #expect(blocks.topLine == 19)
        controller.jumpPrompt(.down)
        #expect(blocks.displayOffset == 0 && blocks.topLine == 0)
    }

    @Test func copyOutputPutsTheLinesOnTheClipboardAndSaysOnlyHowMany() {
        let (controller, blocks, recorder) = make()
        controller.perform(.copyOutput, on: blocks.block(id: 2))
        #expect(recorder.written == ["output line 7\noutput line 8\noutput line 9\noutput line 10\noutput line 11\noutput line 12\noutput line 13"])
        #expect(recorder.notices == [.copied(.output(lines: 7))])
    }

    @Test func actionsActOnTheNewestBlockThatRan() {
        let (controller, _, recorder) = make()
        controller.perform(.copyCommand)
        #expect(recorder.written == ["ls"])
        #expect(recorder.notices == [.copied(.command)])
    }

    @Test func aRefusedPutAtPromptSaysWhyOnTheChip() {
        let (controller, blocks, recorder) = make()
        blocks.putOutcome = .refused(.concealed)
        controller.perform(.putAtPrompt, on: blocks.block(id: 1))
        #expect(recorder.notices == [.failed(.putAtPrompt(.concealed))])
        blocks.putOutcome = .typed
        controller.perform(.putAtPrompt, on: blocks.block(id: 1))
        #expect(blocks.typed == ["git status"])
    }

    @Test func theGeometryFindsTheRowUnderThePointerAndNothingOutsideTheGrid() {
        let geometry = BlockGeometry(rows: 24, cellHeight: 18, top: 4, gutter: 4)
        #expect(geometry.row(atY: 4) == 0)
        #expect(geometry.row(atY: 4 + 18 * 3 + 1) == 3)
        #expect(geometry.row(atY: 3) == nil)
        #expect(geometry.row(atY: 4 + 18 * 24) == nil)
    }
}
