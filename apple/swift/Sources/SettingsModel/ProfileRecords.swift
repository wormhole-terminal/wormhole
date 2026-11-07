// The profiles Settings can make and delete (core's `ProfileList` behind `profiles.json`): the choices the New profile
// sheet copies from, and which one Wormhole is in.

public struct ProfileChoice: Sendable, Equatable, Identifiable {
    public let id: String
    public let name: String
    public let isDefault: Bool
    public let isActive: Bool
    /// "Delete Work? Its terminals, notes, tasks and settings are removed, …" and "A copy of Work", in the catalogs' words.
    public let deleteQuestion: String
    public let copyLabel: String

    public init(
        id: String, name: String, isDefault: Bool = false, isActive: Bool = false, deleteQuestion: String = "",
        copyLabel: String = ""
    ) {
        (self.id, self.name, self.isDefault, self.isActive) = (id, name, isDefault, isActive)
        (self.deleteQuestion, self.copyLabel) = (deleteQuestion, copyLabel)
    }
}

/// What a project's own settings say over the person's (`.wormhole/config.json`'s `settings`), while one of its
/// terminals is in front: the rows it sets are marked and read "Set by <project>/<folder>/config.json".
public struct ProjectSettingsMark: Sendable, Equatable {
    public var name: String
    /// `.wormhole`, or an older project's folder name.
    public var folder: String
    public var root: String
    /// The ids of the rows the project sets.
    public var entries: Set<String>
    /// "Set by <name>/<folder>/config.json while one of its terminals is in front; yours is kept", in the catalogs' words.
    public var note: String

    public init(name: String, folder: String, root: String, entries: Set<String>, note: String) {
        (self.name, self.folder, self.root, self.entries, self.note) = (name, folder, root, entries, note)
    }
}
