// Quitting asks first while an agent or a command runs in any window; a quit with nothing running just goes. It is
// the app delegate's `applicationShouldTerminate`, so the Dock's Quit and logging out ask too (the GPUI app's Dock
// limitation disappears): the answer arrives later through `reply(toApplicationShouldTerminate:)`.

import AppKit
import AuxModel

@MainActor
public enum QuitFlow {
    /// Returns `.terminateNow` when no question is needed, else `.terminateLater` and asks on `window`.
    /// `decide(endHosts)` runs when the person confirms, before the app goes; `endAll` is "Quit and End All Terminals".
    public static func shouldTerminate(
        facts: QuitFacts, endAll: Bool = false, window: NSWindow?, decide: @escaping (_ endHosts: Bool) -> Void
    ) -> NSApplication.TerminateReply {
        guard let question = facts.question(endAll: endAll) else {
            decide(endAll)
            return .terminateNow
        }
        let sheet = QuestionSheet(question)
        Task {
            let answer = await sheet.ask(in: window)
            if case .confirm = answer { decide(QuitFacts.endsHosts(endAll: endAll, answer: answer)) }
            NSApp.reply(toApplicationShouldTerminate: { if case .confirm = answer { true } else { false } }())
        }
        return .terminateLater
    }
}
