// Quitting asks first while an agent or a command runs in any window; a quit with nothing running just goes. It is
// the app delegate's `applicationShouldTerminate`, so the Dock's Quit and logging out ask too (the GPUI app's Dock
// limitation disappears): the answer arrives later through `reply(toApplicationShouldTerminate:)`.

import AppKit
import AuxModel

@MainActor
public enum QuitFlow {
    /// Returns `.terminateNow` when no question is needed, else `.terminateLater` and asks on `window`.
    /// `decide(endHosts)` runs when the person confirms, before the app goes; `endAll` is "Quit and End All Terminals".
    /// It returns whether the app goes now: false when the caller first ends the session hosts off the main thread and
    /// then answers AppKit itself (`reply(toApplicationShouldTerminate: true)`).
    public static func shouldTerminate(
        facts: QuitFacts, endAll: Bool = false, window: NSWindow?, decide: @escaping (_ endHosts: Bool) -> Bool
    ) -> NSApplication.TerminateReply {
        guard let question = facts.question(endAll: endAll) else {
            return decide(endAll) ? .terminateNow : .terminateLater
        }
        let sheet = QuestionSheet(question)
        Task {
            let answer = await sheet.ask(in: window)
            guard case .confirm = answer else { return NSApp.reply(toApplicationShouldTerminate: false) }
            if decide(QuitFacts.endsHosts(endAll: endAll, answer: answer)) { NSApp.reply(toApplicationShouldTerminate: true) }
        }
        return .terminateLater
    }
}
