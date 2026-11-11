// The folders a path menu lists: the folder first and then each parent down to the volume, as the menu of a
// Finder window's title does (`view/layout/path_menu.rs`). Nothing is read from disk: the names are the path's.

import Foundation

public struct PathFolder: Sendable, Equatable {
    public var path: String
    /// The folder's own name, or the volume's name for the top of the tree.
    public var name: String
}

public enum PathChain {
    /// `folder` and its parents, innermost first. The volume is `/` (named `bootVolume`, "Macintosh HD") or,
    /// under `/Volumes/<name>`, that volume's folder, where the list ends as Finder's does.
    public static func of(_ folder: String, bootVolume: String) -> [PathFolder] {
        let parts = folder.split(separator: "/").map(String.init)
        let rooted = folder.hasPrefix("/")
        let volumeDepth = rooted && parts.count >= 2 && parts[0] == "Volumes" ? 2 : nil // text: the /Volumes folder's own name
        var out: [PathFolder] = []
        var depth = parts.count
        while depth > 0 {
            let path = (rooted ? "/" : "") + parts[..<depth].joined(separator: "/")
            out.append(PathFolder(path: path, name: parts[depth - 1]))
            if depth == volumeDepth { return out }
            depth -= 1
        }
        if rooted { out.append(PathFolder(path: "/", name: bootVolume)) }
        return out
    }
}
