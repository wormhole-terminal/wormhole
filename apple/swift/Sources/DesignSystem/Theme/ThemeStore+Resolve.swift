// How the store makes `tokens`: the bundled dump where it is enough, Rust's resolver where it isn't, and the
// window's look on top, as `UiColors::new` does in the GPUI app.

import Foundation

extension ThemeStore {
    func refresh() {
        if let preview {
            // The draft is judged by the resolver already, in the contrast it was asked for, and shows opaque.
            (theme, tokens, problem) = (preview, preview.tokens(increaseContrast: increaseContrast), nil)
            return
        }
        let reference = appearance == .dark ? settings.darkTheme : settings.lightTheme
        let opaque = reduceTransparency || increaseContrast
        let look = opaque ? WindowLook(opacity: 1, blur: false) : settings.look
        let atDefaultLook = look == library.defaultLook
        let bundled = library.theme(id: reference)
        let customised = !settings.primary.isEmpty || settings.branch != .theme
        var shown: (theme: Theme, tokens: ThemeTokens)?
        var problem: String?

        if bundled == nil || customised || projectTheme != nil || !(opaque || atDefaultLook) {
            if let resolver {
                do {
                    let request = ThemeRequest(
                        theme: reference, themesDirectory: themesDirectory, primary: settings.primary,
                        branch: settings.branch.spelling, increaseContrast: increaseContrast, look: look, projectJSON: projectTheme)
                    let resolved = try ResolvedTheme(json: resolver.resolve(request)).theme
                    shown = (resolved, resolved.tokens())
                } catch {
                    problem = "theme \(reference): \(error)"  // text: a developer diagnostic: nothing in the interface shows ThemeStore.problem
                }
            } else if bundled == nil {
                problem = "theme \(reference) is not bundled and there is no resolver"  // text: a developer diagnostic: nothing in the interface shows ThemeStore.problem
            }
        }
        // The dump: exact for a bundled theme at its default look. Without the resolver a pick, or an opacity
        // other than the default, shows as opaque text over the translucent surfaces.
        let base = shown?.theme ?? bundled ?? library.theme(id: library.defaultDark) ?? library.themes[0]
        let resolved = shown?.tokens ?? base.tokens(increaseContrast: increaseContrast)
        let seen = opaque ? nil : (shown != nil || atDefaultLook ? resolved.translucent : nil)
        let windowed = resolved.windowed(opacity: look.opacity, text: seen)
        let lined = showsBorders || increaseContrast ? windowed : windowed.withoutLines()
        (theme, tokens) = (base, lined.with(interfaceFont))
        self.problem = problem
    }

    /// A user theme in its own colours at the default look, for a card of the theme picker; nil when it does not resolve.
    public func userTheme(_ summary: ThemeSummary) -> Theme? {
        guard let resolver else { return nil }
        let request = ThemeRequest(
            theme: summary.id, themesDirectory: themesDirectory, primary: "", branch: "", increaseContrast: false,
            look: library.defaultLook)
        return (try? ResolvedTheme(json: resolver.resolve(request)))?.theme
    }

    /// Reads the themes folder again: a theme was saved or trashed there.
    public func reloadThemes() { reloadUserThemes() }

    /// Reads the themes folder again when a file a pick may come from changed (the theme in use edited by hand or by a
    /// script, another theme it extends, a theme saved elsewhere): one stamp of the folder and the picks, then the same
    /// re-resolve as a save. Called whenever the settings follower reports a change, and costs a `stat` when none.
    public func refreshIfThemeFilesChanged() {
        guard let resolver, let directory = themesDirectory else { return }
        let stamp = resolver.filesStamp(in: directory, picks: [settings.darkTheme, settings.lightTheme])
        if stamp != filesStamp { reloadUserThemes() }
    }

    func reloadUserThemes() {
        if let resolver, let directory = themesDirectory {
            filesStamp = resolver.filesStamp(in: directory, picks: [settings.darkTheme, settings.lightTheme])
            userThemes = (try? JSONDecoder().decode([ThemeSummary].self, from: resolver.userThemes(in: directory))) ?? []
        } else {
            userThemes = []
        }
        refresh()
    }
}
