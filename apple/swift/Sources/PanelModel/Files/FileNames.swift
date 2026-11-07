// How a name is renamed (`wormhole_core::view::file_tree::names::stem_len`): the field selects the name and not its
// extension, as Finder does, so typing replaces `main` and keeps `.rs`.

public enum FileNames {
    /// How many characters of `name` come before its extension. Only the last extension counts (`archive.tar` of
    /// `archive.tar.gz`); a name that starts with its only dot (`.gitignore`) or ends with one has none, and a
    /// folder's name is all name.
    public static func stemLength(_ name: String, isFolder: Bool) -> Int {
        guard !isFolder, let dot = name.lastIndex(of: "."), dot != name.startIndex, name.index(after: dot) != name.endIndex
        else { return name.count }
        return name.distance(from: name.startIndex, to: dot)
    }
}
