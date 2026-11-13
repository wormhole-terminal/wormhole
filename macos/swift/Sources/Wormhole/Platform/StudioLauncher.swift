// The Theme Studio window (View ▸ Theme Studio, `themeStudio`): over the core's theme loader and the data folder's
// `themes` folder, the one the live theme reads its user themes from. Without the core there is no loader, and it says so.

import AppKit
import DesignSystem
import Foundation
import StudioModel
import StudioUI
import WindowUI

#if WITH_UNIFFI
    import AuxBridge
    import AuxModel
    import StudioBridge
    import UniFFISource
    import WormholeBindings
#endif

@MainActor
final class StudioLauncher {
    private let controller = StudioWindowController()
    private let themes: URL
    /// The project of the terminal in front (a trusted project's theme is the one in effect while one of its terminals is).
    private let project: @MainActor () -> (any StudioProject)?

    init(dataDirectory: URL, project: @escaping @MainActor () -> (any StudioProject)? = { nil }) {
        themes = dataDirectory.appendingPathComponent("themes")
        self.project = project
    }

    var document: StudioDocument? { controller.document }
    var session: StudioSession? { controller.activeSession }
    var isOpen: Bool { controller.isOpen }

    /// The menu's Save theme and Duplicate, Export JSON and Copy JSON.
    func saveTheme() async { await controller.saveTheme() }
    func duplicateTheme() { controller.duplicateTheme() }
    func setOutputs(_ outputs: StudioOutputs) { controller.outputs = outputs }
    func exportJSON() async { await controller.exportJSON() }
    func copyJSON() { controller.copyJSON() }

    /// ⌥⌘, opens the Studio, and closes an open one (asking about unsaved edits as the red light does).
    func toggle(store: ThemeStore = .shared) {
        if isOpen { controller.close() } else { show(store: store) }
    }

    func show(store: ThemeStore = .shared) {
        #if WITH_UNIFFI
            try? FileManager.default.createDirectory(at: themes, withIntermediateDirectories: true)
            SketchSource.preset = { WindowSketches.blocks(preset: $0, width: $1) }
            let environment = StudioEnvironment(
                resolver: FfiThemeResolver(), folder: DiskThemeFolder(directory: themes), knowledge: GalleryKnowledge(),
                files: FfiStudioFiles(directory: themes), project: project(),
                exportFolder: ExportFolder(
                    recall: { studioExportDir().map { URL(fileURLWithPath: $0) } },
                    remember: { studioRememberExportDir(dir: $0.path) }))
            do {
                // The window's own chrome in the gallery: the Studio is a layer below the window and is handed its specimens.
                try controller.show(theme: store.theme.name, environment: environment, store: store) { WindowSpecimens.view(for: $0) }
            } catch {
                FileHandle.standardError.write(Data("theme studio: \(error)\n".utf8))  // text: a log line
            }
        #else
            FileHandle.standardError.write(Data("theme studio: needs the Rust library (macos/scripts/build-ffi.sh)\n".utf8))  // text: a log line
        #endif
    }
}

/// The Studio's project seam, or none where there is no core.
@MainActor
enum StudioProjects {
    static func source(app: AppController) -> (any StudioProject)? {
        #if WITH_UNIFFI
            return StudioProjectSource(app: app)
        #else
            return nil
        #endif
    }
}
