import Foundation
import PlatformModel
import Testing

@testable import PlatformUI

@MainActor @Suite struct QuickPanelModelTests {
    private func model() -> QuickPanelModel {
        let model = QuickPanelModel()
        let now = Date()
        model.now = now
        model.terminals = [
            AmbientTerminal(session: 1, name: "diff perf", state: .needsYou, since: now - 120, source: .agent, who: "Claude Code", message: "Approve cargo test?"),
            AmbientTerminal(session: 2, name: "prod", state: .needsYou, since: now - 300),
            AmbientTerminal(session: 3, name: "tests", state: .working, since: now - 72),
        ]
        model.items = [.newTerminal(cwd: "/tmp"), .openFolder]
        return model
    }

    @Test func theFirstSelectableRowIsSelectedAndArrowsStopAtTheEnds() {
        let model = model()
        #expect(model.selectedRow?.waiting?.name == "prod")
        model.step(1)
        #expect(model.selectedRow?.waiting?.name == "diff perf")
        model.step(1); model.step(1); model.step(1); model.step(1)
        #expect(model.selectedRow?.command == .openFolder)
        model.step(10)
        #expect(model.selectedRow?.command == .openFolder, "the last row holds")
        model.step(-100)
        #expect(model.selectedRow?.waiting?.name == "prod")
    }

    @Test func aRowIsFoundAgainByWhatItIsWhenTheListChanges() {
        let model = model()
        model.select(model.rows[2])
        model.terminals.insert(AmbientTerminal(session: 9, name: "new", state: .needsYou, since: Date() - 600), at: 0)
        #expect(model.selectedRow?.waiting?.name == "diff perf")
        model.query = "tests"
        #expect(model.selectedRow?.command == .openTab(3), "a new query selects the first match")
    }

    @Test func returnRunsTheRowAfterClosing() {
        let model = model()
        var order: [String] = []
        model.close = { order.append("close") }
        model.send = { order.append("\($0)") }
        model.run()
        #expect(order == ["close", "openTab(2)"])
    }

    @Test func commandReturnAnswersInPlaceAndReplySendsThenMovesOn() {
        let model = model()
        var sent: [AmbientCommand] = []
        model.send = { sent.append($0) }
        model.commandReturn()
        #expect(model.answering?.tab == 2 && model.answering?.question == "Waiting for your answer")
        let wasHeight = model.height
        #expect(model.footerHints.first?.label == "send reply")
        model.reply = "  "
        model.sendReply()
        #expect(model.answering != nil && sent.isEmpty, "a blank reply is not sent")
        model.reply = "yes"
        model.sendReply()
        #expect(sent == [.reply(tab: 2, text: "yes")] && model.answering == nil && model.reply.isEmpty)
        #expect(model.selectedRow?.waiting?.name == "diff perf", "on to the next terminal that waits")
        #expect(wasHeight > model.height, "the answer region closed")
    }

    @Test func commandReturnWhileAnsweringOpensTheTerminal() {
        let model = model()
        var order: [String] = []
        model.close = { order.append("close") }
        model.send = { order.append("\($0)") }
        model.commandReturn()
        model.commandReturn()
        #expect(order == ["close", "openTab(2)"])
    }

    @Test func onlyAWaitingRowTakesAReply() {
        let model = model()
        model.select(model.rows.first { $0.id == "terminal.3" }!)
        model.commandReturn()
        #expect(model.answering == nil)
    }

    @Test func theWindowIsAsTallAsItsRowsUpToTheMost() {
        let model = model()
        // search 48 + top 4 + footer 36, labels 28 each (Needs input, Working, Do), rows 36 each.
        let expected: Double = 48 + 4 + 36 + 84 + 180
        #expect(model.height == expected)
        model.terminals = (1...40).map { AmbientTerminal(session: Int64($0), name: "t\($0)", state: .working) }
        #expect(model.height == 560)
    }
}
