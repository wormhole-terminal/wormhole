// What a window shows of itself outside its content, kept current from the state it follows: its name (the selected
// terminal's, as the Window menu, Cmd-` and Mission Control list it: `window/windows.rs` `sync_title_soon`) and how it
// opens and closes (`scaling.rs` `sync_window_animation`: the system's document animation with the Motion feature on, a cut
// under Reduce Motion, the default for a test instance and a torn-off window). Both are events: the observation fires
// when the selection, the name or either fact moves, never on a poll.

import AppKit
import Observation
import WindowUI
import WindowsModel
import WorkspaceModel

extension AppController {
    func followWindow(_ session: WindowSession) {
        withObservationTracking { applyWindowFacts(session) } onChange: { [weak self, weak session] in
            Task { @MainActor in
                if let self, let session { self.followWindow(session) }
            }
        }
    }

    private func applyWindowFacts(_ session: WindowSession) {
        guard let window = session.window else { return }
        // No terminal, or none with a name yet: the window keeps the name it opened with.
        if let name = session.hub.state.selectedTerminal?.title, !name.isEmpty, window.title != name { window.title = name }
        let look = session.dress.look
        let behaviour = session.windows.backend.windowAnimation(
            motion: look.motionFeature, reduceMotion: look.themes.reduceMotion, background: options.background,
            grows: session.grow != nil)
        let wanted: NSWindow.AnimationBehavior = switch behaviour {
        case .standard: .default
        case .document: .documentWindow
        case .cut: .none
        }
        if window.animationBehavior != wanted { window.animationBehavior = wanted }
    }
}
