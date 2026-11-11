// The Command line tool row (`cli_tool`): where the `wh` script stands and whether its folder is on the PATH a new
// terminal gets. The sentences arrive written by the core; the row draws them and sends Install and Uninstall back.

public struct CommandLineToolRecord: Sendable, Equatable {
    public enum State: Sendable { case notInstalled, installed, foreign }
    public var state: State
    /// The command's name (`wh`).
    public var command: String
    /// The row's line: what it is for and where it stands.
    public var headline: String
    /// What needs saying under it: whose `wh` is in the way, or that the installed copy runs another Wormhole.
    public var note: String?
    /// The installed copy runs the binary that runs now; when it does not, Update is offered.
    public var current: Bool
    /// The folder the script is in, which the PATH note asks about.
    public var folder: String?

    public init(
        state: State, command: String = "wh", headline: String, note: String? = nil, current: Bool = false,
        folder: String? = nil
    ) {
        (self.state, self.command, self.headline, self.note, self.current, self.folder) =
            (state, command, headline, note, current, folder)
    }
}

/// Whether the script's folder is on a new terminal's PATH, with the line to add when it is not.
public struct CommandLinePathNote: Sendable, Equatable {
    public enum Verdict: Sendable { case on, off, unknown }
    public var verdict: Verdict
    public var text: String
    /// The line to paste into the shell's startup file (only when `off`).
    public var line: String?

    public init(verdict: Verdict, text: String, line: String? = nil) { (self.verdict, self.text, self.line) = (verdict, text, line) }
}
