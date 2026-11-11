// What the window lists in the palette besides the keymap: the front terminal's trusted project and the shells a new
// terminal can run (core's `command_palette/rows/items.rs` `ProjectItem` and `terminal::shells::ShellChoice`).

import Foundation

/// The front terminal's project: its name for the group's label, and its actions once the project is trusted.
public struct PaletteProject: Sendable, Equatable {
    /// "terminal/.wormhole".
    public var name: String
    public var actions: [Action]

    public struct Action: Sendable, Equatable {
        public var id: String
        public var title: String
        /// What it does or runs, set in the code font.
        public var detail: String?
        /// Its key as glyphs ("⌃⌘T").
        public var key: String?

        public init(id: String, title: String, detail: String? = nil, key: String? = nil) {
            (self.id, self.title, self.detail, self.key) = (id, title, detail, key)
        }
    }

    public init(name: String, actions: [Action]) { (self.name, self.actions) = (name, actions) }
}

/// A shell besides the default one (which the plain New terminal opens): what a "New terminal: fish" row starts.
public struct PaletteShell: Sendable, Equatable {
    public var key: String
    public var name: String
    /// What it runs as a line ("/opt/homebrew/bin/fish -l").
    public var commandLine: String

    public init(key: String, name: String, commandLine: String) { (self.key, self.name, self.commandLine) = (key, name, commandLine) }
}
