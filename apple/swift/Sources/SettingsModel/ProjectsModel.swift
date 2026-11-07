// What the Project folders page does: which project it shows (none: the defaults for all of them) and the records of
// each, read again after anything that changes them.

import Observation

@MainActor @Observable
public final class ProjectsModel {
    private let backend: any ProjectsBackend
    public private(set) var defaults: [ProjectDefault] = []
    public private(set) var trusted: [ProjectChoice] = []
    public private(set) var known: [ProjectChoice] = []
    /// The project picked, `nil` for "All projects (defaults)".
    public private(set) var selected: String?
    public private(set) var page: ProjectPage?

    public init(backend: any ProjectsBackend) { self.backend = backend }

    public func refresh() async {
        defaults = await backend.defaults()
        trusted = await backend.trusted()
        known = await backend.known()
        page = if let selected { await backend.page(selected) } else { nil }
    }

    public func select(_ root: String?) async {
        selected = root
        await refresh()
    }

    public func setDefault(_ key: String, on: Bool) async {
        await backend.setDefault(key, on: on)
        await refresh()
    }

    public func setChoice(_ feature: String, to choice: Bool?) async {
        guard let selected else { return }
        await backend.setChoice(selected, feature: feature, to: choice)
        await refresh()
    }

    public func trust() async {
        guard let selected else { return }
        await backend.trust(selected)
        await refresh()
    }

    public func openTerminal() {
        if let selected { backend.openTerminal(selected) }
    }

    public func revoke(_ root: String) async {
        await backend.revoke(root)
        await refresh()
    }
}
