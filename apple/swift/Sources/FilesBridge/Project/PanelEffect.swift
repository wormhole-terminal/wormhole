// What a project action, a prompt or the layout asks the window to do. Rust decides what trust allows and says
// it as effects; the window performs them (it owns the terminals), so the bridge never types into one itself.

import Foundation
import WormholeBindings

/// Which terminal an effect is for.
public enum EffectPlace: Equatable, Sendable {
    case current
    /// A new terminal, in `cwd`, selected.
    case new(cwd: String?)
    /// The terminal called `name`, made in `cwd` when there is none, and selected.
    case named(String, cwd: String?)
}

public struct NewTerminal: Equatable, Sendable {
    public var name: String
    public var cwd: String
    /// Typed in once it starts, then Return.
    public var run: String?
    public var focus: Bool
}

public enum PanelEffect: Equatable, Sendable {
    /// Type `command` into the terminal, then Return.
    case type(String, in: EffectPlace)
    /// Paste `text`; Return a moment later with `submit`.
    case paste(String, submit: Bool, in: EffectPlace)
    /// An http(s) URL, for the default browser.
    case open(URL)
    /// The layout's terminals (reusing ones with the same name).
    case openTerminals([NewTerminal])
}

/// What the window does with the effects and the problems the panel's backends report. Implemented by the host.
public protocol PanelEffectPerforming: Sendable {
    func perform(_ effects: [PanelEffect]) async
    /// A call that did nothing, in words for the notice chip ("Not allowed: the project is not trusted").
    func problem(_ text: String) async
}

extension EffectPlace {
    init(_ place: TerminalPlace) {
        switch place {
        case .current: self = .current
        case let .new(cwd): self = .new(cwd: cwd)
        case let .named(name, cwd): self = .named(name, cwd: cwd)
        }
    }
}

extension PanelEffect {
    /// An effect as Rust says it; a URL that is not http(s) is dropped.
    init?(_ effect: ProjectEffect) {
        switch effect {
        case let .command(place, command): self = .type(command, in: EffectPlace(place))
        case let .paste(place, text, submit): self = .paste(text, submit: submit, in: EffectPlace(place))
        case let .openUrl(url):
            // The loaders accept http(s) only; anything else (the system is lenient about what a URL is) is dropped.
            guard let url = URL(string: url), ["http", "https"].contains(url.scheme?.lowercased()) else { return nil }
            self = .open(url)
        case let .openTerminals(terminals):
            self = .openTerminals(terminals.map { NewTerminal(name: $0.name, cwd: $0.cwd, run: $0.run, focus: $0.focus) })
        }
    }
}
