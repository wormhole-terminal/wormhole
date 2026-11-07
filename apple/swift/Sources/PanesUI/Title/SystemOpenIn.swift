// Open in… against the system: which of the editors Wormhole knows are installed (the system is asked by bundle
// identifier), their icons, and opening a folder in one as when the person drags it onto the app. The table
// mirrors `wormhole-core` `platform::open_in::EDITORS` and goes when that rule arrives through the FFI; the
// chosen app is the app-wide `open_in` setting, the window's to say.

import AppKit
import PanesModel

@MainActor
public struct SystemOpenIn: OpenInBackend {
    private let chosen: () -> String

    public init(chosen: @escaping () -> String = { "" }) { self.chosen = chosen }

    public func apps() -> [OpenInApp] {
        Self.editors.filter { Self.url(for: $0.bundle) != nil } + [.finder]
    }

    public func choice() -> OpenInChoice {
        OpenInChoice.resolve(chosen: chosen(), installed: apps()) { bundle in
            (Self.editors + [.finder]).first { $0.bundle == bundle }?.name
        }
    }

    public func open(_ folder: String, in app: OpenInApp) {
        let target = URL(fileURLWithPath: folder)
        guard app != .finder, let url = Self.url(for: app.bundle) else {
            NSWorkspace.shared.open(target)
            return
        }
        NSWorkspace.shared.open([target], withApplicationAt: url, configuration: NSWorkspace.OpenConfiguration())
    }

    /// The app's own icon at the glyph size, nil for one that isn't installed.
    public static func icon(_ app: OpenInApp) -> NSImage? {
        guard let url = url(for: app.bundle) else { return nil }
        return NSWorkspace.shared.icon(forFile: url.path)
    }

    private static func url(for bundle: String) -> URL? { NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundle) }

    private static let editors: [OpenInApp] = [
        ("dev.zed.Zed", "Zed", "Z"), ("dev.zed.Zed-Preview", "Zed Preview", "ZP"),
        ("com.microsoft.VSCode", "Visual Studio Code", "VS"), ("com.microsoft.VSCodeInsiders", "VS Code Insiders", "VI"),
        ("com.todesktop.230313mzl4w4u92", "Cursor", "Cu"), ("com.exafunction.windsurf", "Windsurf", "Ws"),
        ("com.vscodium", "VSCodium", "VC"), ("com.sublimetext.4", "Sublime Text", "St"),
        ("com.sublimetext.3", "Sublime Text 3", "St"), ("com.apple.dt.Xcode", "Xcode", "Xc"),
        ("com.jetbrains.intellij", "IntelliJ IDEA", "IJ"), ("com.jetbrains.intellij.ce", "IntelliJ IDEA Community", "IC"),
        ("com.jetbrains.PhpStorm", "PhpStorm", "Ph"), ("com.jetbrains.WebStorm", "WebStorm", "WS"),
        ("com.jetbrains.pycharm", "PyCharm", "PC"), ("com.jetbrains.pycharm.ce", "PyCharm Community", "PC"),
        ("com.jetbrains.goland", "GoLand", "GL"), ("com.jetbrains.rubymine", "RubyMine", "RM"),
        ("com.jetbrains.CLion", "CLion", "CL"), ("com.jetbrains.rider", "Rider", "Ri"),
        ("com.jetbrains.datagrip", "DataGrip", "DG"), ("com.jetbrains.rustrover", "RustRover", "RR"),
        ("com.google.android.studio", "Android Studio", "AS"), ("com.panic.Nova", "Nova", "No"),
        ("com.barebones.bbedit", "BBEdit", "BB"), ("com.macromates.TextMate", "TextMate", "TM"),
    ].map { OpenInApp(bundle: $0.0, name: $0.1, monogram: $0.2) }
}
