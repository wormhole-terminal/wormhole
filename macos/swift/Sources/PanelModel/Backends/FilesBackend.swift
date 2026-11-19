// Where the Files tab's data comes from: the disk, read under caps and link-safe by core's `storage::files`, the pins
// in the store, the folder watch. Swift holds the tree and draws it; the FFI will implement this.

public protocol FilesBackend: Sendable {
    /// One folder below `root`, sorted, cleaned and capped as `Dir::from_listing` does.
    func list(_ folder: String, in root: String) async -> Result<Directory, DirectoryFailure>
    /// What the folder or file at `path` looks like from outside, as a word that stays the same until it changes (the
    /// core's `stamp_below`, one lstat); nil when nothing is there or the backend cannot say. Asked before a read: what
    /// reads the same word as last time is not read again.
    func stamp(_ path: String, in root: String) async -> String?
    /// The file for the viewer: capped, links never followed out of `root`.
    func read(_ path: String, in root: String) async -> FileContent
    /// Renames `path` to `name` in its own folder; refuses a name that can't be one and anything that exists.
    func rename(_ path: String, to name: String, in root: String) async -> Result<Void, RenameFailure>
    /// The real path of the file (`folder`: the folder) at `path`, every link resolved, for handing it to another app; nil
    /// when it is gone, is not of that kind any more or leads out of `root`.
    func resolve(_ path: String, in root: String, folder: Bool) async -> String?
    func pins() async -> [String]
    func setPinned(_ folder: String, _ pinned: Bool) async
    /// Fires when something under `root` changed. The OS thread only marks; the store debounces.
    func changes(in root: String) -> AsyncStream<Void>
}

extension FilesBackend {
    /// A backend with no disk under it (previews, tests) reads every time.
    public func stamp(_ path: String, in root: String) async -> String? { nil }
}
