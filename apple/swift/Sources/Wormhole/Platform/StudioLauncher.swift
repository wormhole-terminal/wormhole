// The Theme Studio window (View ▸ Theme Studio, `themeStudio`): over the core's theme loader and the data folder's
// `themes` folder, the one the live theme reads its user themes from. Without the core there is no loader, and it says so.

import AppKit
import DesignSystem
import Foundation
import StudioModel
import StudioUI

#if WITH_UNIFFI
    import AuxBridge
    import AuxModel
    import UniFFISource
#endif

@MainActor
final class StudioLauncher {
    private let controller = StudioWindowController()
    private let themes: URL

    init(dataDirectory: URL) { themes = dataDirectory.appendingPathComponent("themes") }

    var document: StudioDocument? { controller.document }
    var isOpen: Bool { controller.isOpen }

    /// The menu's Save theme and Duplicate, Export JSON and Copy JSON.
    func saveTheme() async { await controller.saveTheme() }
    func duplicateTheme() { controller.duplicateTheme() }
    func setOutputs(_ outputs: StudioOutputs) { controller.outputs = outputs }
    func exportJSON() async { await controller.exportJSON() }
    func copyJSON() { controller.copyJSON() }

    func show(store: ThemeStore = .shared) {
        #if WITH_UNIFFI
            try? FileManager.default.createDirectory(at: themes, withIntermediateDirectories: true)
            SketchSource.preset = { WindowSketches.blocks(preset: $0, width: $1) }
            let environment = StudioEnvironment(
                resolver: FfiThemeResolver(), folder: DiskThemeFolder(directory: themes), knowledge: GalleryKnowledge())
            do {
                try controller.show(theme: store.theme.name, environment: environment, store: store)
            } catch {
                FileHandle.standardError.write(Data("theme studio: \(error)\n".utf8))  // text: a log line
            }
        #else
            FileHandle.standardError.write(Data("theme studio: needs the Rust library (apple/scripts/build-ffi.sh)\n".utf8))  // text: a log line
        #endif
    }
}
