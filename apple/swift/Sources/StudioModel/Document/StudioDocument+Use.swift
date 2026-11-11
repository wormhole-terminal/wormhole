// Using the theme being edited (`studio/use_theme.rs`): whether the app shows it (apart from the edits previewed), making it the
// pick of its appearance (saving first when there is something to save), and, for a saved pair, following the system's light and
// dark with both picks.

import DesignSystem
import Foundation

extension StudioDocument {
    /// Whether the app shows this theme (the edits being previewed apart). Worked out when the document or the pick
    /// last changed: finding out whether a pick names a file reads the themes folder, which a draw never does.
    public func refreshInUse() {
        let pick = store.shownPick
        switch subject {
        case .project: inUse = true
        case .untitled(let from): inUse = !isDirty && pick.caseInsensitiveCompare(from) == .orderedSame
        case .own(let url):
            let id = file.id
            inUse = store.picks?.selects(pick, path: url.path) == true
                || id.map { pick == $0 || pick.hasPrefix($0 + ".") } == true
                || store.userThemes.contains { $0.path == url.path && ($0.id == pick || $0.name == pick) }
        }
        shownPick = pick
    }

    /// Use: the theme becomes the pick of its appearance. The session saves a draft with edits (or a copy with no file)
    /// first, asking about a name or an overwrite, and calls this once the theme is a file.
    public func use() {
        guard let value = pickValue(shown) else { return }
        if let picks = store.picks { picks.pick(value, dark: appearance == .dark) } else if let id = store.userThemes.first(where: { $0.name == name })?.id { store.setTheme(id) }
        refreshInUse()
    }

    /// Whether Use with Match system is offered: a saved pair whose two appearances are not both picks yet.
    public var canFollowSystem: Bool {
        guard let light = pickValue("light"), let dark = pickValue("dark"), file.isPair, file.id != nil else { return false }
        return !(store.settings.lightTheme == light && store.settings.darkTheme == dark)
    }

    /// Use with Match system: the light pick is the pair's light appearance, the dark pick its dark one.
    public func followSystem() {
        guard let light = pickValue("light"), let dark = pickValue("dark") else { return }
        store.picks?.followSystem(light: light, dark: dark)
        notice = .done(StudioText.pairUsed(name))
        refreshInUse()
    }
}
