// The seam for the interface's words. The GPUI app's words are `wormhole-text`'s Fluent catalogs; Swift modules
// ask for a message by its Fluent id and give the English as the fallback, so a view reads the same in a build
// without the Rust core. Linked to it, `WordsBridge` installs a source that holds a language's whole table (one
// fetch, then a dictionary lookup, never a call per word).

import Foundation
import Synchronization

/// A variable of a message: text, or a whole number (what a plural selects on).
public enum WordValue: Sendable, Equatable {
    case text(String)
    case whole(Int)
}

public protocol WordSource: Sendable {
    /// The text of the message called `id` in the language in use; nil when this source has none.
    func text(for id: String) -> String?
    /// The message called `id` with its variables filled in; nil when this source cannot.
    func format(_ id: String, _ args: [String: WordValue]) -> String?
}

public extension WordSource {
    func format(_ id: String, _ args: [String: WordValue]) -> String? { nil }
}

public enum Words {
    private static let installed = Mutex<(any WordSource)?>(nil)

    /// Makes `source` the words of the interface; replacing it (a language change) is what views re-read.
    public static func install(_ source: (any WordSource)?) { installed.withLock { $0 = source } }

    /// The message called `id`, or `fallback` (the English the caller wrote) when no source has it.
    public static func text(_ id: StaticString, fallback: @autoclosure () -> String) -> String {
        let source = installed.withLock { $0 }
        return source?.text(for: String(describing: id)) ?? fallback()
    }

    /// A message with variables, named `args` as in its Fluent text (`{ $folder }` is `"folder"`); `fallback` is the
    /// English with the values already in it.
    public static func text(_ id: StaticString, _ args: [String: WordValue], fallback: @autoclosure () -> String) -> String {
        let source = installed.withLock { $0 }
        return source?.format(String(describing: id), args) ?? fallback()
    }
}
