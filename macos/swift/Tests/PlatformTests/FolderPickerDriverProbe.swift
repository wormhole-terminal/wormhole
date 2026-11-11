import PlatformModel
import PlatformUI

/// A driver over two mock providers: the folders of the current folder, and a slower recent list.
@MainActor
enum FolderPickerDriverProbe {
    static func make() -> FolderPickerDriver {
        FolderPickerDriver(
            context: PickerContext(cwd: "/Users/ben/Projects", home: "/Users/ben"),
            providers: [MockFolders(folders: ["Projects", "src"]), MockFolders(folders: [], recent: ["~/Projects/terminal"])],
            decorators: [MockFolders.sample])
    }
}
