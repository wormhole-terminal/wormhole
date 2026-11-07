// The Theme Studio window (View ▸ Theme Studio, `themeStudio`): over the core's theme loader and the data folder's
// `themes` folder, the one the live theme reads its user themes from. Without the core there is no loader, and it says so.

import AppKit
import DesignSystem
import Foundation
import StudioModel
import StudioUI

#if WITH_UNIFFI
    import UniFFISource
#endif

@MainActor
final class StudioLauncher {
    private let controller = StudioWindowController()
    private let themes: URL

    init(dataDirectory: URL) { themes = dataDirectory.appendingPathComponent("themes") }

    func show(store: ThemeStore = .shared) {
        #if WITH_UNIFFI
            try? FileManager.default.createDirectory(at: themes, withIntermediateDirectories: true)
            let environment = StudioEnvironment(resolver: FfiThemeResolver(), folder: DiskThemeFolder(directory: themes))
            do {
                try controller.show(theme: store.theme.name, environment: environment, store: store)
            } catch {
                FileHandle.standardError.write(Data("theme studio: \(error)\n".utf8))
            }
        #else
            FileHandle.standardError.write(Data("theme studio: needs the Rust library (apple/scripts/build-ffi.sh)\n".utf8))
        #endif
    }
}
