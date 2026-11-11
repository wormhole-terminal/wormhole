// The menu of the folder in the title row (`window/title/path_menu.rs`), like the one a Finder window's title
// opens: the path as shown heads it; Open in Finder, Copy path and Insert into terminal follow; then the path's
// folders, innermost first, down to the volume, each with Finder's icon. Choosing a folder opens it in Finder.

import Foundation
import WordsModel

public enum PathMenu {
    public enum Kind: String, Sendable {
        case openInFinder, copyPath, insertIntoTerminal, openFolder
        public var key: String { "path.\(rawValue)" }
    }

    /// `folder` as shown (`~/Projects/tube`), `volume` the name of the boot volume, `exited`: no terminal runs to
    /// type into.
    public static func nodes(folder: String, shown: String, volume: String, exited: Bool) -> [MenuNode] {
        var menu = ContextMenuList()
        menu.add(.header(shown))
        menu.separator()
        menu.item(Words.text("title-row-path-open-in-finder", fallback: "Open in Finder"), Kind.openInFinder.key, argument: folder)
        menu.item(Words.text("title-row-path-copy", fallback: "Copy path"), Kind.copyPath.key, argument: folder)
        menu.item(Words.text("title-row-path-insert", fallback: "Insert into terminal"), Kind.insertIntoTerminal.key, argument: folder, enabled: !exited)
        menu.separator()
        for step in PathChain.of(folder, volume: volume) {
            menu.item(step.name, Kind.openFolder.key, argument: step.path, icon: .file(path: step.path))
        }
        return menu.finished
    }
}

/// The folders a path menu lists (`view::layout::path_menu::chain`): the folder and its parents, innermost first,
/// down to the volume. Nothing is read from disk: the names are the path's own.
public enum PathChain {
    public struct Step: Equatable, Sendable {
        public var path: String
        public var name: String
    }

    /// The volume is `/` (named `volume`, "Macintosh HD") or, for a path under `/Volumes/<name>`, that volume's
    /// folder, where the list ends as Finder's does.
    public static func of(_ folder: String, volume: String) -> [Step] {
        let parts = folder.split(separator: "/", omittingEmptySubsequences: true).map(String.init)
        let absolute = folder.hasPrefix("/")
        let onVolume = absolute && parts.count >= 2 && parts[0] == "Volumes"  // text: the folder macOS mounts volumes in
        let stop = onVolume ? "/Volumes/\(parts[1])" : nil
        var steps: [Step] = []
        for count in stride(from: parts.count, through: 0, by: -1) {
            let path = (absolute ? "/" : "") + parts.prefix(count).joined(separator: "/")
            if count == 0 {
                if absolute { steps.append(Step(path: "/", name: volume)) }
                break
            }
            steps.append(Step(path: path, name: parts[count - 1]))
            if path == stop { break }
        }
        return steps
    }
}
