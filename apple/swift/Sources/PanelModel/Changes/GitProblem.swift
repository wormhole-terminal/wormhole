// What the Changes and History tabs say when git cannot answer (`features/git/problem.rs`): git is missing and how to
// install it, git refuses the folder's owner (the command is shown, never run), or git failed (its first line, and the
// whole message to copy). Core hands over git's message; the sentences are the app's, as in the GPUI app's crate.

public struct GitProblem: Equatable, Sendable {
    public enum Tab: Sendable { case changes, history }

    public var headline: String
    public var hint: String?
    /// Shown in the code font: the command to run, or git's own first line.
    public var detail: String
    /// The whole message, for Copy error; nil when there is nothing more to copy.
    public var copy: String?

    public init(message: String, tab: Tab) {
        if message == "git is not installed" {
            headline = tab == .changes ? "Changes need git, and it isn't installed." : "History needs git, and it isn't installed."
            (hint, detail, copy) = ("Install the Xcode Command Line Tools, then retry.", "xcode-select --install", nil)
        } else if let path = Self.dubiousPath(message) {
            headline = "git doesn't trust who owns this folder."
            (hint, detail, copy) = ("If you trust it, run this once, then retry.", "git config --global --add safe.directory '\(path)'", message)
        } else {
            headline = "Couldn't read this repository."
            (hint, detail, copy) = (nil, message.split(separator: "\n", omittingEmptySubsequences: false).first.map(String.init) ?? "", message)
        }
    }

    /// The folder git refuses, when it can stand in a shell command as it is: a quote or a control character in a folder
    /// name someone else chose leaves the generic wording.
    static func dubiousPath(_ message: String) -> String? {
        guard let first = message.split(separator: "\n", omittingEmptySubsequences: false).first,
            let range = first.range(of: "dubious ownership in repository at '"), first.hasSuffix("'")
        else { return nil }
        let path = first[range.upperBound...].dropLast()
        return path.isEmpty || path.contains("'") || path.unicodeScalars.contains(where: { $0.properties.generalCategory == .control }) ? nil : String(path)
    }
}
