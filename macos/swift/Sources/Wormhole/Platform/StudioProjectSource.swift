// The project of the terminal in front, for the Theme Studio: whose theme is in effect, and where "Use in this project"
// writes. Which project, whether its theme is on and trusted, and the file are the core's (`ProjectService`); this
// only says which terminal is in front.

#if WITH_UNIFFI
    import FilesBridge
    import Foundation
    import StudioModel
    import WindowUI
    import WormholeBindings
    import WorkspaceModel

    @MainActor
    struct StudioProjectSource: StudioProject {
        let app: AppController

        /// The folder of the terminal in front as the panel reads it: a key where a name is not text, never the drawn
        /// folder, whose U+FFFD could name a planted sibling (`caf\u{FFFD}/.wormhole` beside `caf\xE9/`).
        private var folder: String? { app.hub.state.selectedPath }

        private var service: ProjectService? { (app.panelBackends.project as? RustProjectBackend)?.service }

        func overlay() -> (root: String, text: String)? {
            guard let folder, let service, let text = service.themeOverlay(folder: folder),
                let root = try? service.themeTarget(folder: folder)
            else { return nil }
            return (root, text)
        }

        func write(_ json: String) -> Result<URL, StudioFailure> {
            guard let service, let folder else { return .failure(.noTerminal) }
            do {
                return .success(URL(fileURLWithPath: try service.writeTheme(folder: folder, json: json)))
            } catch let failure as ProjectFailure {
                switch failure {
                case .Refused(let reason), .Failed(let reason): return .failure(StudioFailure(reason))
                case .NotFound: return .failure(.noProjectFolder)
                }
            } catch {
                return .failure(StudioFailure("\(error)"))
            }
        }
    }
#endif
