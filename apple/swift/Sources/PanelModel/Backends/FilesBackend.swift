// Where the Files tab's data comes from: the disk, read under caps and link-safe by core's `storage::files`, the pins
// in the store, the folder watch. Swift holds the tree and draws it; the FFI will implement this.

public protocol FilesBackend: Sendable {
    /// One folder below `root`, sorted, cleaned and capped as `Dir::from_listing` does.
    func list(_ folder: String, in root: String) async -> Result<Directory, DirectoryFailure>
    /// The file for the viewer: capped, links never followed out of `root`.
    func read(_ path: String, in root: String) async -> FileContent
    /// Renames `path` to `name` in its own folder; refuses a name that can't be one and anything that exists.
    func rename(_ path: String, to name: String, in root: String) async -> Result<Void, RenameFailure>
    func pins() async -> [String]
    func setPinned(_ folder: String, _ pinned: Bool) async
    /// Fires when something under `root` changed. The OS thread only marks; the store debounces.
    func changes(in root: String) -> AsyncStream<Void>
}
