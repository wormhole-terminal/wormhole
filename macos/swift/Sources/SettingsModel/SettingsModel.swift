// What the settings window shows and does, as one observable value: the schema, the values the backend holds,
// which page is open and what is searched. Views read it and call back; nothing in it draws. A write goes to the
// backend, which answers through its observation, so a change made anywhere (the file, another window) lands
// the same way as one made here.

import Observation

@MainActor @Observable
public final class SettingsModel {
    /// The page Changed shows is not a section of the schema: it lists the others' rows.
    public static let changedID = "changed"

    public let schema: SettingsSchema
    public let backend: any SettingsBackend
    public let host: any SettingsHost
    public private(set) var values: [String: JSONValue]
    /// The Profile page's fields and the Import page's open card: pages with state of their own.
    public let profile: ProfileDraft
    public let importDraft: ImportDraft
    /// The open page: a section id or `changed`.
    public var selection: String
    public var query = ""
    /// The New profile sheet is up, and the question before deleting the profile Wormhole is in.
    public var showsNewProfile = false
    /// The font an open font picker is looking at; the Fonts page's scene shows it instead of the settings' own.
    public var fontPreview: FontPreview?
    /// A row to bring into view and mark for a moment, after a search result was chosen.
    public private(set) var revealed: String?
    /// The Extensions and Project folders pages' state, made once the first time a page asks (nil where the host has none).
    @ObservationIgnored public private(set) lazy var probesModel = host.probes.map { ProbesModel(backend: $0) }
    @ObservationIgnored public private(set) lazy var projectsModel = host.projects.map { ProjectsModel(backend: $0) }
    @ObservationIgnored private var observation: SettingsObservation?
    @ObservationIgnored private let searcher: any SettingsSearching
    /// Core's rules for a recorded shortcut; `nil` leaves the recorder to its own plain rule.
    @ObservationIgnored public let shortcuts: (any ShortcutChecking)?
    /// A recorder row started or stopped recording: the global hotkey is held off while it has the keys.
    @ObservationIgnored public var onRecording: ((Bool) -> Void)?
    /// Why the global hotkey set now did not register with the system (`hotkey::error`); nil when it did or none is set.
    /// Opens the Theme Studio (the button beside Import theme).
    @ObservationIgnored public var openStudio: (() -> Void)?
    @ObservationIgnored public var hotkeyError: (() -> String?)?
    /// What macOS allows for notifications and what the last test came to; the app keeps it and `refreshNotify` reads it again.
    public var notifyStatus = NotifyStatus()
    @ObservationIgnored public var refreshNotify: (() -> Void)?
    /// Zen mode is on: the windows show less than these settings say, and a page with a row it overrides says so.
    public var zenOn = false
    /// Leaves Zen mode (the note's button).
    @ObservationIgnored public var leaveZen: (() -> Void)?

    public init(
        schema: SettingsSchema = .bundled, backend: any SettingsBackend, host: any SettingsHost,
        searcher: (any SettingsSearching)? = nil, shortcuts: (any ShortcutChecking)? = nil,
        selection: String = "appearance"
    ) {
        (self.schema, self.backend, self.host, self.selection) = (schema, backend, host, selection)
        self.shortcuts = shortcuts
        profile = ProfileDraft(host: host)
        importDraft = ImportDraft(host: host)
        self.searcher = searcher ?? SchemaSearch(schema: schema)
        values = backend.values
        observation = backend.observe { [weak self] in self?.values = self?.backend.values ?? [:] }
    }

    // MARK: Values

    public func value(_ key: String) -> JSONValue { values[key] ?? schema.defaults[key] ?? .null }

    public func set(_ key: String, _ value: JSONValue) { backend.set(key, to: value) }

    public func isChanged(_ key: String) -> Bool { value(key) != (schema.defaults[key] ?? .null) }

    public func isChanged(_ entry: SettingsSchema.Entry) -> Bool { entry.keys.contains(where: isChanged) }

    public func reset(_ entry: SettingsSchema.Entry) { backend.reset(entry.keys) }

    public func resetAll() { backend.reset(changedKeys) }

    /// Every key that differs from its default: what Reset all puts back, and what its question counts.
    public var changedKeys: [String] { changedEntries.flatMap(\.entry.keys).filter(isChanged) }
    public var changedKeyCount: Int { changedKeys.count }

    // MARK: Pages

    public struct Located: Identifiable, Equatable {
        public let section: SettingsSchema.Section
        public let group: SettingsSchema.Group
        public let entry: SettingsSchema.Entry
        public var id: String { entry.id }
    }

    /// The rows that differ from their default and show now, in page order: what the Changed page lists.
    public var changedEntries: [Located] {
        // Core decides which rows differ and show (`Values::changed_rows`); the model's own rule is for a build without it.
        if let rows = backend.changedRows() {
            _ = values
            return rows.compactMap { row in
                guard let section = schema.sections.first(where: { $0.id == row.section }) else { return nil }
                return section.groups.lazy.compactMap { group in
                    group.entries.first { $0.id == row.entry }.map { Located(section: section, group: group, entry: $0) }
                }.first
            }
        }
        return schema.sections.flatMap { section in
            section.groups.flatMap { group in
                group.entries.filter { isChanged($0) && $0.isShown(values: values) && !isBlock($0) }
                    .map { Located(section: section, group: group, entry: $0) }
            }
        }
    }

    public var changedCount: Int { changedEntries.count }

    /// A row that is a scene or a record list, never "changed".
    private func isBlock(_ entry: SettingsSchema.Entry) -> Bool {
        switch entry.control {
        case .preview, .panel: true
        default: false
        }
    }

    /// Sections in nav order, the Profile page (reached from the select above the list) apart.
    public var navSections: [SettingsSchema.Section] {
        // `values` is read so a flag turned on (which shows its page) redraws the nav.
        _ = values
        let shown = host.shownSections()
        return schema.sections.filter { $0.id != "profile" && $0.id != Self.changedID && shown?.contains($0.id) ?? true }
    }

    public var isSearching: Bool { !query.trimmingCharacters(in: .whitespaces).isEmpty }

    public var hits: [SearchHit] { isSearching ? searcher.search(query, values: values) : [] }

    /// What the settings index finds for `text` without touching the window's own search (the command palette's).
    public func find(_ text: String) -> [SearchHit] { searcher.search(text, values: values) }

    /// How many results each section holds, for the nav's counts while a search runs.
    public var hitCounts: [String: Int] { Dictionary(hits.map { ($0.section, 1) }, uniquingKeysWith: +) }

    // MARK: Moving

    public func show(_ section: String) {
        selection = section
        revealed = nil
    }

    /// Opens the page of a search result and marks the row, as choosing it in the results list does.
    public func jump(to hit: SearchHit) {
        query = ""
        selection = hit.section
        revealed = hit.entry.id
    }

    /// Opens the page of the row `id` and marks it (the command palette's setting result, which names no search).
    public func reveal(_ id: String) {
        guard let found = schema.entry(id) else { return }
        query = ""
        selection = found.section.id
        revealed = id
    }

    public func clearReveal() { revealed = nil }
}
