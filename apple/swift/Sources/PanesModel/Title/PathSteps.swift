// The folder's ways to be drawn, measured. The core lists them (`TitleSnapshot.pathSteps`, widest first, each with the
// middle folders left out one more); the shell measures each text in its own font and cuts a last folder that alone is
// wider than `lastMax` in its middle, keeping the extension whole.

import Foundation

public enum PathFace: Sendable { case parents, last }

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
    public static func measured(_ texts: [PathText], lastMax: Double, measure: (PathFace, String) -> Double) -> [PathStep] {
        texts.map { text in
            let last = Elide.fit(text.last, max: lastMax, measure: { measure(.last, $0) }, shorten: Elide.middle)
            return PathStep(text: PathText(parents: text.parents, last: last), width: measure(.parents, text.parents) + measure(.last, last))
        }
    }
}
