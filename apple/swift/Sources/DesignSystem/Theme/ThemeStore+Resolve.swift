// How the store makes `tokens`: the bundled dump where it is enough, Rust's resolver where it isn't, and the
// window's look on top, as `UiColors::new` does in the GPUI app.

import Foundation

extension ThemeStore {
    func refresh() {
        let reference = appearance == .dark ? settings.darkTheme : settings.lightTheme
        let opaque = reduceTransparency || increaseContrast
        let look = opaque ? WindowLook(opacity: 1, blur: false) : settings.look
        let atDefaultLook = look == library.defaultLook
        let bundled = library.theme(id: reference)
        let customised = !settings.primary.isEmpty || settings.branch != .theme
        var shown: (theme: Theme, tokens: ThemeTokens)?
        var problem: String?

        if bundled == nil || customised || !(opaque || atDefaultLook) {
            if let resolver {
                do {
                    let request = ThemeRequest(
                        theme: reference, themesDirectory: themesDirectory, primary: settings.primary,
                        branch: settings.branch.spelling, increaseContrast: increaseContrast, look: look)
                    let resolved = try ResolvedTheme(json: resolver.resolve(request)).theme
                    shown = (resolved, resolved.tokens())
                } catch {
                    problem = "theme \(reference): \(error)"
                }
            } else if bundled == nil {
                problem = "theme \(reference) is not bundled and there is no resolver"
            }
        }
        // The dump: exact for a bundled theme at its default look. Without the resolver a pick, or an opacity
        // other than the default, shows as opaque text over the translucent surfaces.
        let base = shown?.theme ?? bundled ?? library.theme(id: library.defaultDark) ?? library.themes[0]
        let resolved = shown?.tokens ?? base.tokens(increaseContrast: increaseContrast)
        let seen = opaque ? nil : (shown != nil || atDefaultLook ? resolved.translucent : nil)
        (theme, tokens) = (base, resolved.windowed(opacity: look.opacity, text: seen))
        self.problem = problem
    }

    func reloadUserThemes() {
        if let resolver, let directory = themesDirectory {
            userThemes = (try? JSONDecoder().decode([ThemeSummary].self, from: resolver.userThemes(in: directory))) ?? []
        } else {
            userThemes = []
        }
        refresh()
    }
}
