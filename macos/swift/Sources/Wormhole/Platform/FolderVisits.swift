// Which of the terminals' folders are visits for the folder picker's recent folders (`app_state/recent_folders.rs`): a
// terminal's folder the first time it is seen and every time it changes, in the list of the profile of the window that
// holds it, while that profile has the Folder picker flag on; with the flag off, the profile's list is forgotten the first
// time a terminal of it has a folder. Plain values in and out, so the rule is tested without a window or the core.

import WorkspaceModel

struct FolderVisits {
    /// One window as the rule sees it: its profile, whether the flag is on there, and each terminal's folder.
    struct Window {
        var profile: String
        var recorded: Bool
        var folders: [(terminal: TerminalID, folder: String?)]
    }

    /// What a look at the windows comes to: the visits to record, in order, and the profiles whose list is forgotten.
    struct Outcome: Equatable {
        struct Visit: Equatable {
            var profile: String
            var folder: String
        }

        var visits: [Visit] = []
        var forget: [String] = []
    }

    private var visited: [TerminalID: String] = [:]
    private var forgotten: Set<String> = []

    /// The windows as they are now, looked at after every change of a terminal's folder and of the windows themselves.
    mutating func take(_ windows: [Window]) -> Outcome {
        var outcome = Outcome()
        for window in windows {
            guard window.recorded else {
                if !forgotten.contains(window.profile), window.folders.contains(where: { $0.folder != nil }) {
                    forgotten.insert(window.profile)
                    outcome.forget.append(window.profile)
                }
                continue
            }
            for (terminal, folder) in window.folders {
                guard let folder, visited[terminal] != folder else { continue }
                visited[terminal] = folder
                outcome.visits.append(.init(profile: window.profile, folder: folder))
            }
        }
        return outcome
    }
}
