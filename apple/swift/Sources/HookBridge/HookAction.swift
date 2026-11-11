// What the control socket asks of the app, as plain values: what only the app can do. Rust answers the request
// (the caller's reply is already written); these are what is left to do on the screen.

import WormholeBindings

public enum HookAction: Equatable, Sendable {
    /// A second launch, or `wh focus --activate`: come to the front.
    case activate
    /// `wh focus --activate`: bring the window that holds `terminal` to the front (it is selected there already).
    case front(terminal: Int64)
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
    /// A Probe's hook asks for a notification: its name is the subtitle ("<name> (extension)").
    case probeNotice(probe: String, subtitle: String, title: String, body: String)
    /// A Probe's hook offers a file in `terminal`'s folder, in the chip's words; a click opens it (`resolvePath`).
    case probeOffer(terminal: Int64, words: String, text: String)
    /// A hook failed too often and was paused.
    case probePaused(words: String)
    /// A program is typing at `terminal` (else only reading it): the chip of the window that shows it says who. `agent` is the
    /// coding agent's name as the sentence has it; none is a script.
    case driven(terminal: Int64, typing: Bool, agent: String?)
    /// The rules, a status item, a tinted block or a fold changed: every terminal takes a frame, the status bar reads again.
    case probesChanged
    /// The welcome program's sidebar step: the example band goes up in the sidebar of the window that holds `terminal` (`show`)
    /// or comes down. It is never a terminal.
    case welcomeDemo(terminal: Int64, show: Bool)
    /// The welcome in `terminal` is over: the band goes and a start animation still playing is cut short.
    case welcomeEnded(terminal: Int64)
    /// A press turned a banner on, or said Allow, and macOS has not been asked: ask, then `setNotificationPermission`.
    case welcomeAskPermission
}

extension HookAction {
    init(_ event: HookEvent) {
        switch event {
        case .activate: self = .activate
        case .front(let terminal): self = .front(terminal: terminal)
        case .notice(let terminal, let title, let body): self = .notice(terminal: terminal, title: title, body: body)
        case .closeAsked(let terminal): self = .closeAsked(terminal: terminal)
        case .worktreeAsked(let folder, let branch): self = .worktreeAsked(folder: folder, branch: branch)
        case .gauge(let terminal, let agent, let model, let context):
            self = .gauge(terminal: terminal, agent: agent, model: model, context: context.map(Double.init))
        case .newWindow(let window, let terminal, let folder, let activate):
            self = .newWindow(window: window, terminal: terminal, folder: folder, activate: activate)
        case .probeNotice(let probe, let subtitle, let title, let body):
            self = .probeNotice(probe: probe, subtitle: subtitle, title: title, body: body)
        case .probeOffer(let terminal, let words, let text): self = .probeOffer(terminal: terminal, words: words, text: text)
        case .probePaused(let words): self = .probePaused(words: words)
        case .driven(let terminal, let typing, let agent): self = .driven(terminal: terminal, typing: typing, agent: agent)
        case .probesChanged: self = .probesChanged
        case .welcomeDemo(let terminal, let show): self = .welcomeDemo(terminal: terminal, show: show)
        case .welcomeEnded(let terminal): self = .welcomeEnded(terminal: terminal)
        case .welcomeAskPermission: self = .welcomeAskPermission
        }
    }
}
