// What the Changes and History tabs say when git cannot answer (`wormhole_core::git::problem`): git is missing and how to install
// it, git refuses the folder's owner (the command is shown, never run), or git failed (its first line, and the whole message to
// copy). Core reads git's message and words it, so the GPUI and the Swift app say the same; the app installs core's rule at launch
// (`FilesBridge.installRules()`), and previews and tests get the plain failure.

public struct GitProblem: Equatable, Sendable {
    public enum Tab: Sendable { case changes, history }

    public var headline: String
    public var hint: String?
    /// Shown in the code font: the command to run, or git's own first line.
    public var detail: String
    /// The whole message, for Copy error; nil when there is nothing more to copy.
    public var copy: String?

    public init(message: String, tab: Tab) {
        self = Self.rule(message, tab == .history)
    }

    public init(headline: String, hint: String? = nil, detail: String, copy: String? = nil) {
        (self.headline, self.hint, self.detail, self.copy) = (headline, hint, detail, copy)
    }

    /// Core's rule: git's message and whether the History tab asks; the stand-in is the generic failure.
    // why: written once on the main thread at launch, before any window reads it; every later access is a read.
    nonisolated(unsafe) public static var rule: @Sendable (_ message: String, _ history: Bool) -> GitProblem = { message, _ in
        GitProblem(
            headline: "Couldn't read this repository.", // text: the stand-in of core's own git::problem words, which the installed rule speaks
            detail: message.split(separator: "\n", omittingEmptySubsequences: false).first.map(String.init) ?? "", copy: message)
    }
}
