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
    /// The themes folder of the profile Wormhole is in, asked each time the Studio opens (a profile switch moves it): what the
    /// live theme reads its user themes from (`ThemeStore.themesDirectory`), else the data folder's own.
    private let themesFolder: @MainActor () -> URL
    /// The project of the terminal in front (a trusted project's theme is the one in effect while one of its terminals is).
    private let project: @MainActor () -> (any StudioProject)?

    init(
        dataDirectory: URL, themesFolder: (@MainActor () -> URL)? = nil,
        project: @escaping @MainActor () -> (any StudioProject)? = { nil }, keyChanged: @escaping @MainActor () -> Void = {}
    ) {
        controller.onKeyChange = keyChanged
        self.themesFolder = themesFolder ?? { dataDirectory.appendingPathComponent("themes") }
        self.project = project
    }

    var document: StudioDocument? { controller.document }
    var session: StudioSession? { controller.activeSession }
    var isOpen: Bool { controller.isOpen }

    /// What the Edit menu's Undo and Redo name while the Studio is the key window: its draft's own steps (`publish_edit_titles`);
    /// nil in any other window.
    var editTitles: (undo: String?, redo: String?)? {
        guard controller.isKey, let document = controller.document else { return nil }
        return (document.undoLabel, document.redoLabel)
    }

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
            let themes = themesFolder()
            try? FileManager.default.createDirectory(at: themes, withIntermediateDirectories: true)
            // The folder is asked again at every call, so a profile switch moves an open Studio to the new profile's themes.
            // Every caller asks from the main thread today; one from elsewhere waits for the main thread instead of trapping.
            let live: @Sendable () -> URL = { [themesFolder] in
                if Thread.isMainThread { return MainActor.assumeIsolated { themesFolder() } }
                return DispatchQueue.main.sync { MainActor.assumeIsolated { themesFolder() } }
            }
            SketchSource.preset = { WindowSketches.blocks(preset: $0, width: $1) }
            let environment = StudioEnvironment(
                resolver: FfiThemeResolver(), folder: FfiThemeFolder(folder: live), knowledge: GalleryKnowledge(uses: FfiTokenUses.all()),
                files: FfiStudioFiles(folder: live), project: project(),
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
