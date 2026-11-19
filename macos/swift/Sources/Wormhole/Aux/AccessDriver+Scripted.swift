// What a test instance's control folder does about programs that ask to control it (GPUI's `agent_access/scripted.rs` and
// `ask.rs` `answer_pending`): answer the question that is up as a button does (`agent-ask`), or say what answers every
// question from now on (`agent-script`). Nothing here is reachable from a request over the control socket; only the
// control folder calls it, and the real app (no control folder) never does.

import AuxModel

extension AccessDriver {
    /// What answers the programs' questions from now on without a sheet; nil holds them for `agent-ask` (in a test
    /// instance the sheet is held for the control folder, never shown).
    static var script: AccessButton?

    /// The button a wish comes to on a question that offers `buttons`: itself when offered, else the longest allowance
    /// the question has that is no longer than the wish (`ask.rs` `offered`).
    static func offered(_ buttons: [AccessButton], wish: AccessButton) -> AccessButton {
        if buttons.contains(wish) { return wish }
        switch wish {
        case .session, .always, .keep: return buttons.contains(.quit) ? .quit : .deny
        case .deny, .dontKeep, .quit: return .deny
        }
    }

    /// Answers the question that is up as `wish`'s button does; for the question about a changed mode, deny is Don't Keep
    /// and anything else is Keep. Whether there was one. Its held sheet goes when the desk says the question is settled.
    func answerPending(_ wish: AccessButton) -> Bool {
        if let change = askedMode {
            answers.answerMode(change.token, keep: wish != .deny)
            return true
        }
        guard let question = asked else { return false }
        answers.answer(question.id, Self.offered(AccessDisplay.of(question.request).buttons, wish: wish))
        return true
    }
}
