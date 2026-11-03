import SwiftUI

/// All settings values, by setting key ("<page>:<group>:<row>", plus "#size" / the check's name / the part's
/// name for a row's sub-values). The window never reads or writes a file: a host implements this over its own
/// config (the Wormhole app's Rust side, over FFI) and hands it to `SettingsView`.
///
/// Implement it as an `@Observable` class (or bump `revision` inside `withMutation`) so SwiftUI redraws on change.
public protocol SettingsStore: AnyObject {
    /// Changes with every write; the views read it to know they must redraw.
    var revision: Int { get }
    /// The value set for the key, nil when it is at its default.
    func value(for key: String) -> String?
    /// The key's default, nil when the host has none (the catalog's is used).
    func defaultValue(for key: String) -> String?
    /// Keys whose value differs from their default (the Changed page).
    var changedKeys: Set<String> { get }
    func set(_ value: String, for key: String)
    func reset(_ key: String)
    func resetAll()
    /// A button was pressed: its id (never its words) and the row (or "<page>:" for a page's footer) it belongs to.
    func perform(_ action: String, for key: String)
    /// The options a menu row offers, when the host knows better than the catalog (the app's real choices: stable ids, words in its language).
    func options(for key: String) -> [Choice]?
    /// The real fonts, themes and accent the previews draw with.
    var preview: PreviewData? { get }
    /// A word of the window in the app's language, by its id (`wordID`); nil shows the English it was made from.
    func word(_ id: String) -> String?
    /// The themes to pick from, when the host has its own.
    var themes: ThemeData? { get }
    /// The installed font families (name, monospaced), when the host has them.
    var fonts: [FontChoice]? { get }
    /// Rows the host adds to the end of a group ("<page>:<group>"): things that exist on this Mac, not in the catalog.
    func list(for group: String) -> [[String: Any]]
    /// What an extension's approval sheet shows, by the id its list entry names.
    func sheet(for id: String) -> [String: Any]?
    /// The profile Wormhole is in: its name and the letter of its avatar.
    var profile: (name: String, letter: String)? { get }
    /// Whether the host records shortcuts itself (it validates and saves them); else the control does, locally.
    /// Where an update stands: status, hint, tone (ok, warn, muted), buttons, busy, percent (-1 when no download). nil: the card shows nothing real.
    var update: [String: Any]? { get }
    /// Whether a page's fixed notice does not apply now.
    func quiet(_ page: String) -> Bool
    var recordsInHost: Bool { get }
    /// The shortcut row the host waits for keys on, "" when none.
    var recording: String { get }
    /// A warning to show on a row (keys already taken, a hotkey that did not register).
    func note(for key: String) -> String?
    /// A message that says a number, in the app's language with its plural form, for `count`: nil when the host has none for it.
    func counted(_ name: String, _ count: Int) -> String?
    /// The recorder of a row starts or stops; a key press is sent with its held modifiers ("ctrl,alt,shift,cmd") and name.
    func record(_ action: String, for key: String, mods: String, key name: String)
}

public struct FontChoice: Hashable {
    public var name: String
    public var mono: Bool
    public init(name: String, mono: Bool) { self.name = name; self.mono = mono }
}

public extension SettingsStore {
    func perform(_ action: String, for key: String) {}
    func options(for key: String) -> [Choice]? { nil }
    var themes: ThemeData? { nil }
    var preview: PreviewData? { nil }
    func word(_ id: String) -> String? { nil }
    var fonts: [FontChoice]? { nil }
    func list(for group: String) -> [[String: Any]] { [] }
    func sheet(for id: String) -> [String: Any]? { nil }
    var profile: (name: String, letter: String)? { nil }
    var update: [String: Any]? { nil }
    func quiet(_ page: String) -> Bool { false }
    var recordsInHost: Bool { false }
    var recording: String { "" }
    func note(for key: String) -> String? { nil }
    func counted(_ name: String, _ count: Int) -> String? { nil }
    func record(_ action: String, for key: String, mods: String, key name: String) {}
}

/// The default store: values in memory only, nothing is written anywhere.
@Observable public final class InMemorySettings: SettingsStore {
    public private(set) var revision = 0
    private var values: [String: String]
    private let defaults: [String: String]

    public init(defaults: [String: String] = SettingsCatalog.standard.defaults, initial: [String: String] = InMemorySettings.demoSeed) {
        self.defaults = defaults
        self.values = initial.filter { defaults[$0.key] != $0.value }
    }
    /// The board's seeded differences, so the Changed page has something to show.
    public static let demoSeed = ["Terminal:0:0": "50_000_lines", "Sessions:0:1": "compact", "Terminal:0:2": "on", "Notifications:0:4": "on", "Git:1:1": "on"]

    public func value(for key: String) -> String? { values[key] }
    public func defaultValue(for key: String) -> String? { defaults[key] }
    public var changedKeys: Set<String> { Set(values.keys) }
    public func set(_ value: String, for key: String) {
        if defaults[key] == value { values[key] = nil } else { values[key] = value }
        revision += 1
    }
    public func reset(_ key: String) { values[key] = nil; revision += 1 }
    public func resetAll() { values = [:]; revision += 1 }
}

/// The id a word has in the app's catalogs: a slug of its English and the FNV-1a hash of all of it
/// (`scripts/native-words.py` makes the same id). The English is only the source of the id; what shows is the
/// host's word for it.
func wordID(_ english: String) -> String {
    var h: UInt32 = 0x811C9DC5
    for b in english.utf8 { h = (h ^ UInt32(b)) &* 0x01000193 }
    var slug = ""
    for ch in english.lowercased().prefix(40) {
        if ch.isASCII && (ch.isLetter || ch.isNumber) { slug.append(ch) } else if !slug.hasSuffix("-") { slug.append("-") }
    }
    slug = slug.trimmingCharacters(in: CharacterSet(charactersIn: "-"))
    let hex = String(h, radix: 16).leftPad(8)
    return slug.isEmpty ? "native-\(hex)" : "native-\(slug)-\(hex)"
}

private extension String {
    func leftPad(_ n: Int) -> String { String(repeating: "0", count: max(0, n - count)) + self }
}

/// Words by their English source: `words("", "Reset")`, the host's text for it.
struct Words {
    let lookup: (String) -> String
    func callAsFunction(_ key: String, _ english: String) -> String { lookup(english) }
    func callAsFunction(_ english: String) -> String { lookup(english) }
}

/// The window's state: the store (values), the catalog (rows) and what only the window knows (page, search).
@MainActor @Observable public final class SettingsModel {
    let store: any SettingsStore
    let catalog: SettingsCatalog
    var page: String
    var search = ""
    var scrollTo: String? = nil
    var hoverFont: String? = nil
    var specimenTab = "Terminal"

    public init(store: any SettingsStore, catalog: SettingsCatalog = .standard, page: String = "Terminal") {
        self.store = store; self.catalog = catalog; self.page = page
    }
    /// The words of the window, in the host's language.
    var words: Words { let s = store; return Words { _ = s.revision; return s.word(wordID($0)) ?? $0 } }
    /// A word with a value in it: the host's text has `%@` where the value goes.
    func w(_ english: String, _ value: String) -> String { words(english).replacingOccurrences(of: "%@", with: value) }
    func w(_ english: String) -> String { words(english) }

    func value(_ key: String) -> String { _ = store.revision; return store.value(for: key) ?? store.defaultValue(for: key) ?? catalog.defaults[key] ?? "" }
    func get(_ key: String, _ row: Row) -> String { _ = store.revision; return store.value(for: key) ?? store.defaultValue(for: key) ?? row.defaultValue }
    func isOn(_ key: String, _ row: Row) -> Bool { get(key, row) == "on" }
    func set(_ key: String, _ v: String) { store.set(v, for: key) }
    func options(_ row: Row) -> [Choice] { _ = store.revision; return store.options(for: row.key) ?? row.choices }
    var profileInfo: (name: String, letter: String) { _ = store.revision; return store.profile ?? ("Default", "D") }
    var fontList: [(String, Bool)] { _ = store.revision; return store.fonts?.map { ($0.name, $0.mono) } ?? fontChoices }
    var themeData: ThemeData { _ = store.revision; return store.themes ?? .standard }
    func perform(_ action: String, _ key: String) { store.perform(action, for: key) }
    var updateCard: [String: Any]? { _ = store.revision; return store.update }
    func note(_ key: String) -> String? { _ = store.revision; return store.note(for: key) }
    var recordingKey: String { _ = store.revision; return store.recording }

    /// The rows of a group: the catalog's that apply, then the host's own (agents, projects, probes found on this Mac).
    func rows(_ page: Page, _ g: Int) -> [Row] {
        _ = store.revision
        let group = "\(page.id):\(g)"
        return page.groups[g].rows.filter { visible($0, in: page, group: g) && !(store.recordsInHost && $0.kind == .update && store.update == nil) } + store.list(for: group).compactMap { Row(entry: $0, group: group) }
    }
    func sheetData(_ id: String) -> ApprovalSheetData? {
        _ = store.revision
        return store.sheet(for: id).flatMap(ApprovalSheetData.init(entry:))
    }
    /// A plural message from the host (the count it was made for must be the one shown), else the English fallback.
    func counted(_ name: String, _ count: Int, _ fallback: String) -> String { _ = store.revision; return store.counted(name, count) ?? fallback }
    func isChanged(_ key: String) -> Bool { _ = store.revision; return store.changedKeys.contains(key) }

    // Reading a setting by place, as the previews do.
    func str(_ page: String, _ g: Int, _ r: Int) -> String { value("\(page):\(g):\(r)") }
    func on(_ page: String, _ g: Int, _ r: Int) -> Bool { str(page, g, r) == "on" }
    func on(_ key: String) -> Bool { value(key) == "on" }

    func visible(_ row: Row, in page: Page, group g: Int) -> Bool {
        guard let n = row.needs else { return true }
        return isOn(page.groups[g].rows[n].key, page.groups[g].rows[n])
    }

    /// Rows whose value differs from the default.
    var changed: [(Page, Row)] {
        _ = store.revision
        let keys = store.changedKeys
        return catalog.pages.flatMap { p in p.groups.flatMap { $0.rows }.filter { $0.isSetting && keys.contains($0.key) }.map { (p, $0) } }
    }
}
