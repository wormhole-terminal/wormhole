// What the diff card shows for a submodule in place of lines (`features/git/diff/submodule.rs`): the commits between
// the two commits the parent records, newest first; the commits the pointer left behind when it went back; or core's
// sentence for why there is nothing to list (not checked out, not fetched). The lists are capped by the core and the
// totals say how many there are.

public struct SubmoduleCommits: Hashable, Sendable {
    /// The short hashes the pointer was at and is at, when it moved.
    public var old: String?
    public var new: String?
    /// Why no commits are listed, and what is inside the folder besides ("uncommitted changes"), in core's words.
    public var nothing: String?
    public var inside: String?
    public var ahead: [Commit]
    public var behind: [Commit]
    public var aheadTotal: Int
    public var behindTotal: Int

    public init(
        old: String? = nil, new: String? = nil, nothing: String? = nil, inside: String? = nil, ahead: [Commit] = [],
        behind: [Commit] = [], aheadTotal: Int = 0, behindTotal: Int = 0
    ) {
        (self.old, self.new, self.nothing, self.inside) = (old, new, nothing, inside)
        (self.ahead, self.behind, self.aheadTotal, self.behindTotal) = (ahead, behind, aheadTotal, behindTotal)
    }
}
