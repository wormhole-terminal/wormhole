// The families picked lately, newest first, kept per slot. The store is the host's (user defaults in the app);
// the rule (a handful, no repeats) is here.

public protocol FontRecents: AnyObject, Sendable {
    func families(for slot: String) -> [String]
    func store(_ families: [String], for slot: String)
}

extension FontRecents {
    /// The most recent families kept.
    public static var limit: Int { 4 }

    func remember(_ family: String, for slot: String) {
        let kept = families(for: slot).filter { $0 != family }
        store(Array(([family] + kept).prefix(Self.limit)), for: slot)
    }
}

/// Recents that live as long as the window: tests, previews.
public final class MemoryFontRecents: FontRecents, @unchecked Sendable {
    private var byslot: [String: [String]] = [:]
    public init() {}
    public func families(for slot: String) -> [String] { byslot[slot] ?? [] }
    public func store(_ families: [String], for slot: String) { byslot[slot] = families }
}
