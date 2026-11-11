// What the app knows of the fonts folder after registering its files: the families they gave, and a count that moves
// each time the files changed. Whatever drew text with a family set by the person (the terminals, the interface, the
// code font) compares the count to know it must make its fonts again, because a family that was missing drew in a
// fallback face and an equal setting would not otherwise be looked at twice.

public struct FontFolderState: Equatable, Sendable {
    public var revision = 0
    /// The custom families, as the font files name them.
    public var families: Set<String> = []

    public init(revision: Int = 0, families: Set<String> = []) { (self.revision, self.families) = (revision, families) }
}
