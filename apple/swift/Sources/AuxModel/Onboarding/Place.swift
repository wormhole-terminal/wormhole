// A folder the first step offers: where you worked lately and the places a Finder sidebar starts with
// (`onboarding::places`). Not everyone has a ~/Projects, so nothing is scanned for repositories.

import Foundation

public struct Place: Equatable, Sendable, Identifiable {
    public enum Kind: Sendable { case recent, home, desktop, documents, developer }

    public var kind: Kind
    /// "Desktop", or a recent folder's name.
    public var name: String
    public var url: URL
    /// Where it is, with the home folder as `~`.
    public var detail: String
    public var id: URL { url }

    /// The folders the first step lists under `home`: the recent roots that still exist (newest first), then Home,
    /// Desktop, Documents, and Developer when there is one. `exists` answers for the disk, so tests decide it.
    public static func list(home: URL, recents: [URL], exists: (URL) -> Bool) -> [Place] {
        var places = recents.filter(exists).map {
            Place(kind: .recent, name: $0.lastPathComponent, url: $0, detail: tilde($0.deletingLastPathComponent(), home: home))
        }
        let standard: [(Kind, String, URL)] = [
            (.home, "Home", home), (.desktop, "Desktop", home.appending(path: "Desktop")),
            (.documents, "Documents", home.appending(path: "Documents")),
            (.developer, "Developer", home.appending(path: "Developer")),
        ]
        for (kind, name, url) in standard where kind != .developer || exists(url) {
            places.append(Place(kind: kind, name: name, url: url, detail: tilde(url, home: home)))
        }
        return places
    }

    /// `~/…` for a path in the home folder (`~` for the home folder itself).
    public static func tilde(_ url: URL, home: URL) -> String {
        let path = url.standardizedFileURL.path, root = home.standardizedFileURL.path
        if path == root { return "~" }
        return path.hasPrefix(root + "/") ? "~" + path.dropFirst(root.count) : path
    }
}
