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
    /// `wh new-terminal --worktree <branch>`: put the new-worktree question up for the repository `folder` is in,
    /// with `branch` filled in. Nothing is made until the person confirms.
    case worktreeAsked(folder: String, branch: String)
    /// The status line of `terminal`'s `agent` (its command, `claude`) says which model it runs and how much of its
    /// context window is used (0 to 100).
    case gauge(terminal: Int64, agent: String, model: String?, context: Double?)
    /// `wh new-window`: open window `window`, whose first terminal `terminal` is planned already, from the plan the core
    /// holds for it. Behind the other windows and without the keys unless `activate`.
    case newWindow(window: Int64, terminal: Int64, folder: String?, activate: Bool)
}

extension HookAction {
    init(_ event: HookEvent) {
        switch event {
        case .activate: self = .activate
        case .notice(let terminal, let title, let body): self = .notice(terminal: terminal, title: title, body: body)
        case .closeAsked(let terminal): self = .closeAsked(terminal: terminal)
        case .worktreeAsked(let folder, let branch): self = .worktreeAsked(folder: folder, branch: branch)
        case .gauge(let terminal, let agent, let model, let context):
            self = .gauge(terminal: terminal, agent: agent, model: model, context: context.map(Double.init))
        case .newWindow(let window, let terminal, let folder, let activate):
            self = .newWindow(window: window, terminal: terminal, folder: folder, activate: activate)
        }
    }
}
