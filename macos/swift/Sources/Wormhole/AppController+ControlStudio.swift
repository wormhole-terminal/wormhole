// The control folder's `studio-do` verbs that the theme menu, the title row and the sheets give a person: open another theme, New Theme,
// New Theme From, Duplicate, Use (the Preview chip), Add Appearance, Update from, Use in this project, Use with Match system, and a
// look at what the menu offers. They go through the window's session, as a click would; a question the session asks (the system's
// sheet) is answered through `question` like any other.

import DesignSystem
import Foundation
import StudioModel
import StudioUI

extension AppController {
    /// Whether `words` (`studio-do ...`) was one of these; it replies itself.
    func runStudioSession(_ words: [String]) -> Bool {
        guard let session = studio.session else { return false }
        let rest = words.dropFirst().joined(separator: " ")
        switch words.first {
        case "open", "new", "new-from", "new-from-own", "duplicate", "use", "use-in-project", "update-apply", "follow-system", "add-appearance", "close":
            Task { @MainActor in
                switch words.first {
                case "open": await session.open(rest)
                case "new": await session.openNew()
                case "new-from": await session.newFrom(ThemeChoice(name: rest, value: rest), builtin: true)
                case "new-from-own": await session.newFrom(ThemeChoice(name: URL(fileURLWithPath: rest).deletingPathExtension().lastPathComponent, value: rest), builtin: false)
                case "duplicate": session.duplicate()
                case "use": await session.use()
                case "use-in-project": session.useInProject()
                case "update-apply": session.askUpdate(); session.applyUpdate()
                case "follow-system": session.document.followSystem()
                case "add-appearance": session.addAppearance(rest == "dark" ? .dark : .light)
                case "close": session.window?.performClose(nil)
                default: break
                }
                reply(studioLine(session))
            }
            return true
        case "menu":
            let list = session.themeList
            reply(
                "studio menu: yours=\(list.own.map(\.name)) families=\(list.builtins.count) wormhole=\(list.builtins.filter(\.wormhole).map(\.name)) project=\(session.hasProjectTheme) "
                    + "addable=\(session.document.missingAppearance?.rawValue ?? "-") update=\(session.document.update.state) from=\(session.document.update.from) date=\(session.document.update.date) "
                    + "inuse=\(session.document.inUse) newer=\(session.document.isNewer) follow=\(session.document.canFollowSystem) subject=\(session.document.subject) notice=\(session.document.notice.map { $0.text } ?? "-")")
            return true
        default:
            return false
        }
    }

    private func studioLine(_ session: StudioSession) -> String {
        let document = session.document
        return "studio-do: name=\(document.name) subject=\(document.subject) v1=\(document.file.isV1) pair=\(document.file.versions) shown=\(document.shown ?? "-") dirty=\(document.isDirty) "
            + "inuse=\(document.inUse) counts=\(document.counts) notice=\(document.notice.map { $0.text } ?? "-") theme=\(themes.theme.name) pick=\(themes.shownPick)"
    }
}
