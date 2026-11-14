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
    /// The watch on the selected project's folder: a change of its files reads the page again.
    private var following: AnyObject?

    /// Trusting a project needs the owner to answer; without a way to ask, the trusted list says it cannot be done.
    public var canApprove: Bool { backend.canApprove }

    public init(backend: any ProjectsBackend) { self.backend = backend }

    public func refresh() async {
        defaults = await backend.defaults()
        trusted = await backend.trusted()
        known = await backend.known()
        page = if let selected { await backend.page(selected) } else { nil }
    }

    /// Reads the page again whenever the profile's approvals change on disk, for as long as the calling task runs (the
    /// page's `.task`: it ends as the page goes). Returns at once where nothing can be followed.
    public func followTrust() async {
        let (changes, continuation) = AsyncStream<Void>.makeStream(bufferingPolicy: .bufferingNewest(1))
        guard let token = backend.followTrust(changed: { continuation.yield() }) else { return }
        for await _ in changes { await refresh() }
        withExtendedLifetime(token) {}
    }

    public func select(_ root: String?) async {
        selected = root
        watch(true)
        await refresh()
    }

    /// The page came on screen (`true`) or its window closed: the selected project's watch runs only while it can be seen.
    public func watch(_ on: Bool) {
        following = nil
        guard on, let root = selected else { return }
        following = backend.follow(root) { [weak self] in Task { @MainActor in await self?.refresh() } }
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
