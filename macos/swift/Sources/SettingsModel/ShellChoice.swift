// A shell a new terminal can run (core `terminal::shells::ShellChoice`), with what starting it takes: the palette's and the
// menu bar's "New terminal: fish". The core lists them from `config.json` and `/etc/shells`.

public struct ShellChoice: Sendable, Equatable {
    public var key: String
    public var name: String
    /// What it runs as a line ("/opt/homebrew/bin/fish -l").
    public var commandLine: String
    public var program: String
    public var arguments: [String]
    /// What a plain New terminal opens (`default_shell`): the palette leaves it out.
    public var isDefault: Bool

    public init(key: String, name: String, commandLine: String, program: String, arguments: [String], isDefault: Bool) {
        (self.key, self.name, self.commandLine, self.program, self.arguments, self.isDefault) =
            (key, name, commandLine, program, arguments, isDefault)
    }
}
