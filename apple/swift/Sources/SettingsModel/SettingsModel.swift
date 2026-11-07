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
    /// The open page: a section id or `changed`.
    public var selection: String
    public var query = ""
    /// A row to bring into view and mark for a moment, after a search result was chosen.
    public private(set) var revealed: String?
    @ObservationIgnored private var observation: SettingsObservation?
    @ObservationIgnored private let searcher: any SettingsSearching

    public init(
        schema: SettingsSchema = .bundled, backend: any SettingsBackend, host: any SettingsHost,
        searcher: (any SettingsSearching)? = nil, selection: String = "appearance"
    ) {
        (self.schema, self.backend, self.host, self.selection) = (schema, backend, host, selection)
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

    public func resetAll() { backend.reset(changedEntries.flatMap(\.entry.keys).filter(isChanged)) }

    // MARK: Pages

    public struct Located: Identifiable, Equatable {
        public let section: SettingsSchema.Section
        public let group: SettingsSchema.Group
        public let entry: SettingsSchema.Entry
        public var id: String { entry.id }
    }

    /// The rows that differ from their default and show now, in page order: what the Changed page lists.
    public var changedEntries: [Located] {
        schema.sections.flatMap { section in
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
    public var navSections: [SettingsSchema.Section] { schema.sections.filter { $0.id != "profile" && $0.id != Self.changedID } }

    public var isSearching: Bool { !query.trimmingCharacters(in: .whitespaces).isEmpty }

    public var hits: [SearchHit] { isSearching ? searcher.search(query, values: values) : [] }

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

    public func clearReveal() { revealed = nil }
}
