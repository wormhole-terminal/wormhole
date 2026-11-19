// What the Files tab's path menu lists, like the menu a Finder window's title opens (`wormhole_core::view::file_tree::
// menu`): the folder shown and each folder above it, the pinned folders, and the commands for pinning and for going
// back to the terminal's folder. Only the list: the view words it and acts on a choice.

public struct PathMenuModel: Equatable, Sendable {
    public struct Folder: Equatable, Sendable, Identifiable {
        public var path: String
        public var name: String
        public var id: String { path }
    }

    /// The shown folder first (it carries the check), then each parent down to the volume.
    public var folders: [Folder]
    public var pinned: [Folder]
    public var isPinned: Bool
    /// A folder was chosen by hand, so "Follow Terminal" has something to undo.
    public var canFollow: Bool

    /// `volume` names `/` (the disk), `pins` are the pinned folders in the order they were made. Two pins of one
    /// name say where each is.
    public init(shown: String, volume: String, pins: [String], chosen: Bool, home: String? = nil) {
        func name(_ path: String) -> String { path == "/" ? volume : path.split(separator: "/").last.map { DiskName.shown(String($0)) } ?? volume }
        var chain: [Folder] = []
        var path = shown
        while true {
            chain.append(Folder(path: path, name: name(path)))
            guard path != "/", let slash = path.lastIndex(of: "/") else { break }
            path = slash == path.startIndex ? "/" : String(path[..<slash])
        }
        folders = chain
        pinned = pins.map { pin in
            let twin = pins.contains { $0 != pin && name($0) == name(pin) }
            let parent = pin.lastIndex(of: "/").map { String(pin[..<$0]) }.flatMap { $0.isEmpty ? nil : $0 }
            let shownParent = parent.map { folder in DiskName.shown(home.map { folder.replacingHome(with: $0) } ?? folder) }
            return Folder(path: pin, name: twin ? "\(name(pin)) \u{2014} \(shownParent ?? "")" : name(pin)) // text: a name and its parent folder, joined by a dash
        }
        isPinned = pins.contains(shown)
        canFollow = chosen
    }

    /// The shown folder's name, for "Pin “name”".
    public var shownName: String { folders.first?.name ?? "" }

    /// Whether `path` is on the disk the system starts from, as far as its name says: not under `/Volumes` or an
    /// automount, since asking the system for the icon of a folder on a share would touch the share.
    public static func onBootVolume(_ path: String) -> Bool {
        !["/Volumes", "/net", "/Network", "/System/Volumes/Data/Volumes", "/System/Volumes/Data/net"]
            .contains { path == $0 || path.hasPrefix($0 + "/") }
    }
}

extension String {
    /// `/Users/ben/code` as `~/code`.
    fileprivate func replacingHome(with home: String) -> String {
        self == home ? "~" : hasPrefix(home + "/") ? "~" + dropFirst(home.count) : self
    }
}
