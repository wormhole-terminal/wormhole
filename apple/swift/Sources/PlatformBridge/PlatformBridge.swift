// The macOS edge's logic from the Rust core, over the FFI: the keymap and the menu bar (`keys`), the
// notification rules (`notify`), the folder picker's providers (`folder_picker`) and the palette's search
// (`view::search`). Each is a factory the app calls once at launch; the protocols they fill live in
// `PlatformModel` and `PaletteModel`, which know nothing of Rust and keep working, on their dumped or mocked
// stand-ins, in a build without the FFI.

import PaletteModel
import PlatformModel
import WormholeBindings

public enum PlatformBridge {
    /// Makes core's keymap the one every menu, recorder and tooltip reads (`Keymap.shared`).
    public static func installKeymap() { Keymap.install(makeKeymap()) }

    /// Core's table as it is now: every action and the keys macOS keeps for itself.
    public static func makeKeymap() -> Keymap {
        Keymap(
            entries: keymapEntries().map(KeymapEntry.init),
            systemKeys: keymapSystemKeys().map { (KeySpec($0.key), $0.what) })
    }

    /// The menu bar from core's `keymap::menus::menus`.
    public static func makeMenus() -> any MenuProviding { CoreMenus() }

    /// The notification rules from core's `notify`.
    public static func makeNoticeRules() -> any NoticeDeciding { CoreNoticeRules() }

    /// What core starts the notification settings from (`Settings::default()`).
    public static func defaultNoticeSettings() -> NoticeSettings { NoticeSettings(notifyDefaultSettings()) }

    /// The folder picker's providers and decorators from core's `folder_picker`.
    public static func makeFolders(recentsJSON: String? = nil, recentTitle: String = "Recent") -> CoreFolders {
        CoreFolders(recentsJSON: recentsJSON, recentTitle: recentTitle)
    }

    /// Makes core's fuzzy matcher and keymap lists (the palette's actions, the shortcut sheet) the palette's.
    public static func installPaletteMatcher() {
        PaletteSearch.install(CorePaletteMatcher())
        ListedKeymap.install(CoreKeymapLists())
    }
}
