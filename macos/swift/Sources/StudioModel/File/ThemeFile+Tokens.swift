// A token's value in a file, by name. In a pair of versions a value lives in exactly one of the top (both
// versions) and the shown version: the token's own kind decides where a new one goes, and moving it between
// them is one change (`studio/document/places.rs`). `group` is the token's catalogue group, which says
// whether it is shared by default (the primary is; surfaces and text are not).

import Foundation

extension ThemeFile {
    /// Where a value is written.
    public enum Place: Sendable, Equatable { case top, version }

    /// The value the file sets itself for `token` while `version` is edited: the version's, else the top's.
    public func own(_ token: String, in version: String? = nil) -> String? {
        let path = TokenPath(token)
        return versionIndex(version).flatMap { layer(at: $0).value(of: path) } ?? layer(at: nil).value(of: path)
    }

    /// The value as it is in the top or the version alone.
    public func own(_ token: String, at place: Place, version: String?) -> String? {
        let path = TokenPath(token)
        return layer(at: place == .top ? nil : versionIndex(version)).value(of: path)
    }

    /// Where `token`'s value is, or would go if it were set.
    public func place(of token: String, version: String?, group: String?) -> Place {
        guard isPair, let index = versionIndex(version) else { return .top }
        let path = TokenPath(token)
        if layer(at: index).value(of: path) != nil { return .version }
        if layer(at: nil).value(of: path) != nil || path.isShared(group: group) { return .top }
        return .version
    }

    /// Sets (or with `nil` removes) the value, where `place(of:)` says it lives.
    public mutating func set(_ token: String, _ value: String?, version: String? = nil, group: String? = nil) {
        let target = place(of: token, version: version, group: group)
        write(token, value, at: target, version: version)
    }

    /// Moves a value between "both versions" and "this version only", keeping what it is.
    public mutating func move(_ token: String, to place: Place, version: String?) {
        let current = self.place(of: token, version: version, group: nil)
        guard current != place, let value = own(token, at: current, version: version) else { return }
        write(token, nil, at: current, version: version)
        write(token, value, at: place, version: version)
    }

    private mutating func write(_ token: String, _ value: String?, at place: Place, version: String?) {
        let index = place == .top ? nil : versionIndex(version)
        var target = layer(at: index)
        target.set(TokenPath(token), value)
        store(target, at: index)
    }

    func versionIndex(_ version: String?) -> Int? { version.flatMap { versions.firstIndex(of: $0) } }

    /// The tokens whose own value differs between two files (for "primary, git.branch +2" in Versions).
    public static func changedTokens(_ old: ThemeFile, _ new: ThemeFile, among names: [String]) -> [String] {
        let shown = new.versions.first
        return names.filter { old.own($0, in: shown) != new.own($0, in: shown) }
    }
}
