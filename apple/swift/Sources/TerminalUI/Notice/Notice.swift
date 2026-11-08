// What a notice says, as data (`wormhole-core` `view::notice::message`): its kind and its numbers, never what
// was copied, so a password or a shared screen cannot leak through the chip. The words are `NoticeWords`.

import Foundation

public enum Notice: Hashable, Sendable {
    case copied(Copied)
    case driven(Driven)
    case failed(Failed)
    case warned(Warning)

    public enum Copied: Hashable, Sendable {
        case lines(Int), characters(Int)
        case programLines(Int), programCharacters(Int)
        case output(lines: Int)
        case command, path, error
    }

    /// A program that controls the terminal is doing something to it.
    public enum Driven: Hashable, Sendable {
        case typing(Driver), reading(Driver)
    }

    public enum Driver: Hashable, Sendable {
        case agent(String)
        case scripts
    }

    public enum Failed: Hashable, Sendable {
        case copyPath
        case historyKeptByHost
        case putAtPrompt(PutRefusal)
        /// A file a Probe named is not in the terminal's folder; the words come from Rust with the name cleaned and cut.
        case fileNotFound(words: String)
    }

    /// Stays on the chip until it is clicked.
    public enum Warning: Hashable, Sendable {
        case hostFallback, moveToApplications, dataFolder
        case updateReady(version: String)
        /// A Probe's hook offers a file in `terminal`'s folder: a click opens it, never anything by itself.
        case probeOffer(terminal: Int, text: String, words: String)
        /// A hook failed too often in a minute and was paused until the Probes are loaded again.
        case probePaused(words: String)
    }

    /// The picture beside the words (`Glyph`).
    var symbol: String {
        switch self {
        case .failed(.putAtPrompt), .failed(.fileNotFound), .warned: "exclamationmark.triangle"
        case .copied, .failed: "doc.on.doc"
        case .driven(.reading): "eye"
        case .driven(.typing): "keyboard"
        }
    }

    var isSticky: Bool { if case .warned = self { true } else { false } }

    /// A warning with an explanation behind an ⓘ.
    var hasInfo: Bool {
        switch self {
        case .warned(.hostFallback), .warned(.moveToApplications), .warned(.dataFolder): true
        default: false
        }
    }

    /// The capsule itself is the button ("Restart").
    var hasAction: Bool {
        switch self {
        case .warned(.updateReady), .warned(.probeOffer): true
        default: false
        }
    }

    /// A copy of `text`: lines when it has several, else characters (`Copied::text`).
    public static func copied(text: String, byProgram: Bool = false) -> Notice {
        let lines = text.split(omittingEmptySubsequences: false, whereSeparator: \.isNewline).count
        let trailing = text.hasSuffix("\n") ? 1 : 0
        let count = max(lines - trailing, 0)
        if count <= 1 { return .copied(byProgram ? .programCharacters(text.count) : .characters(text.count)) }
        return .copied(byProgram ? .programLines(count) : .lines(count))
    }

    public static func copiedOutput(_ text: String) -> Notice {
        .copied(.output(lines: max(text.split(whereSeparator: \.isNewline).count, 1)))
    }
}
