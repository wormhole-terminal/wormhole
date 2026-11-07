// What the control socket asks of the app, as plain values: what only the app can do. Rust answers the request
// (the caller's reply is already written); these are what is left to do on the screen.

import WormholeBindings

public enum HookAction: Equatable, Sendable {
    /// A second launch, or `wh focus --activate`: come to the front.
    case activate
    /// `wh notify`: show a notification about `terminal`, by the notification rules, in the caller's words.
    case notice(terminal: Int64, title: String, body: String)
    /// `wh close`: ask the person whether to close `terminal` (as closing it by hand does), then call
    /// `HookServerHandle.closeQuestionOver()`. Nothing closes unless they say yes.
    case closeAsked(terminal: Int64)
}

extension HookAction {
    init(_ event: HookEvent) {
        switch event {
        case .activate: self = .activate
        case .notice(let terminal, let title, let body): self = .notice(terminal: terminal, title: title, body: body)
        case .closeAsked(let terminal): self = .closeAsked(terminal: terminal)
        }
    }
}
