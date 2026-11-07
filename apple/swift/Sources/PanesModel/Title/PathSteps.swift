// The path as the title row draws it when narrow: whole, then with parents dropped out of the middle (the
// ones nearest the start first), down to the start, the `…` and the last folder (`view/layout/title_row.rs`).

import Foundation

/// Which text of the path is being measured: the folders before the last (regular) or the last (medium).
public enum PathFace: Sendable { case parents, last }

/// The path as the row draws it: `parents` in regular weight, `last` in medium.
public struct PathText: Sendable, Equatable {
    /// `~/…/Icssaddons/`: the start, the `…` for what was left out, the parents kept, each with its slash.
    public var parents: String
    /// `Tube`.
    public var last: String

    public init(parents: String, last: String) { (self.parents, self.last) = (parents, last) }

    public var full: String { parents + last }
}

public struct PathStep: Sendable, Equatable {
    public var text: PathText
    public var width: Double
}

public enum PathSteps {
    /// The ways to draw `shown` (a path as written for display), the widest first. The start (`~`, or the
    /// first folder of a path outside the home folder) always stays, and so does the last folder, which is
    /// cut in its middle only when it alone is wider than `lastMax`.
    public static func of(_ shown: String, lastMax: Double, measure: (PathFace, String) -> Double) -> [PathStep] {
        let (head, mids, last) = split(shown)
        let cut = Elide.fit(last, max: lastMax, measure: { measure(.last, $0) }, shorten: Elide.middle)
        let lastWidth = measure(.last, cut)
        return (0...mids.count).reversed().map { kept in
            var parents = head
            if kept < mids.count { parents += "\(Elide.ellipsis)/" }
            for name in mids[(mids.count - kept)...] { parents += name + "/" }
            return PathStep(text: PathText(parents: parents, last: cut), width: measure(.parents, parents) + lastWidth)
        }
    }

    /// A path as its start (with the slash after it), the folders between it and the last, and the last. A
    /// path of one name (`~`, `/Users`) is all last.
    static func split(_ shown: String) -> (head: String, mids: [String], last: String) {
        let rooted = shown.hasPrefix("/")
        var names = shown.split(separator: "/").map(String.init)
        guard let last = names.last else { return ("", [], shown) }
        let withRoot = { (name: String) in rooted ? "/" + name : name }
        if names.count == 1 { return ("", [], withRoot(last)) }
        let head = withRoot(names.removeFirst()) + "/"
        names.removeLast()
        return (head, names, last)
    }
}
