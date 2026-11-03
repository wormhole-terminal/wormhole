import SwiftUI

/// The app's settings as the window sees them: the last snapshot the app sent (`apply`), keyed "<page>:<group>:<row>"
/// (plus "#size", a checkbox's or a part's id for a row's sub-values). A write shows at once and goes to the app as
/// an event; the app answers with a snapshot that agrees or corrects it. The window reads and writes no file.
///
/// Each field is observed on its own and replaced only when the snapshot changed it, so a change redraws only the
/// views that read it.
@Observable final class HostStore {
    private(set) var values: [String: String] = [:]
    private(set) var defaults: [String: String] = [:]
    /// The options of a menu row, the app's (stable ids, words in its language).
    private(set) var choices: [String: [Choice]] = [:]
    /// Keys whose value differs from their default (the Changed page).
    private(set) var changed: Set<String> = []
    private var known: Set<String> = []
    private(set) var themes: ThemeData?
    private(set) var preview: PreviewData?
    private(set) var fonts: [FontChoice] = []
    /// Rows the app adds to the end of a group ("<page>:<group>"): things that exist on this Mac.
    private(set) var lists: [String: [[String: Any]]] = [:]
    /// What a sheet shows, by the id a row names.
    private(set) var sheets: [String: [String: Any]] = [:]
    private(set) var profile: [String] = []
    /// The shortcut row the app waits for keys on, "" when none.
    private(set) var recording = ""
    /// A warning to show on a row (keys already taken, a hotkey that did not register).
    private(set) var notes: [String: String] = [:]
    /// Where an update stands; nil where there is no feed to ask.
    private(set) var update: [String: Any]?
    /// Pages whose fixed notice does not apply now, and pages this build doesn't show.
    private(set) var quiet: Set<String> = []
    private(set) var hidden: Set<String> = []
    /// What is wrong with the app's own config file: `text`, and `action` + `button` when it can be opened.
    private(set) var banner: [String: String]?
    /// The window's words in the app's language, by `wordID`.
    private(set) var words: [String: String] = [:]
    private var counts: [String: Any] = [:]
    @ObservationIgnored var send: (String) -> Void = { _ in }

    /// A message that says a number in the app's language: nil unless the app made it for `count`.
    func counted(_ name: String, _ count: Int) -> String? {
        counts["n"] as? Int == count ? (counts["words"] as? [String: String])?[name] : nil
    }

    func set(_ value: String, for key: String) {
        // A row the app does not own keeps what was set until the window closes, so it still reacts.
        values[key] = value
        if !known.contains(key) { changed.insert(key) }
        emit(["t": "set", "k": key, "v": value])
    }
    func reset(_ key: String) { values[key] = nil; changed.remove(key); emit(["t": "reset", "k": key]) }
    func resetAll() { values = [:]; changed = []; emit(["t": "reset_all"]) }
    /// A button: its id (never its words) and the row (or "<page>:" for a page's own buttons).
    func perform(_ action: String, for key: String) { emit(["t": "action", "k": key, "v": action]) }
    /// A shortcut row starts or stops recording; a key press goes with its held modifiers ("ctrl,alt,shift,cmd").
    func record(_ action: String, for key: String, mods: String = "", key name: String = "") {
        emit(["t": "record", "a": action, "k": key, "m": mods, "n": name])
    }

    func emit(_ event: [String: String]) {
        guard let data = try? JSONSerialization.data(withJSONObject: event), let text = String(data: data, encoding: .utf8) else { return }
        send(text)
    }

    /// Replace everything the app owns with its snapshot.
    func apply(_ json: [String: Any]) {
        let localOnly = values.filter { !known.contains($0.key) }
        known = Set(json["known"] as? [String] ?? [])
        values = (json["values"] as? [String: String] ?? [:]).merging(localOnly) { a, _ in a }
        defaults = json["defaults"] as? [String: String] ?? [:]
        choices = (json["options"] as? [String: [[String]]] ?? [:]).mapValues { $0.filter { $0.count == 2 }.map { Choice(id: $0[0], label: $0[1]) } }
        changed = Set(json["changed"] as? [String] ?? []).union(localOnly.keys)
        if let t = json["themes"] as? [String: Any] { themes = Self.themes(t) }
        words = json["words"] as? [String: String] ?? [:]
        counts = json["counts"] as? [String: Any] ?? [:]
        let p = (json["preview"] as? [String: Any]).flatMap(PreviewData.init(json:))
        if p != preview { preview = p }
        if let f = json["fonts"] as? [[Any]] {
            fonts = f.compactMap { e in (e.first as? String).map { FontChoice(name: $0, mono: (e.last as? Bool) ?? false) } }
        }
        // Not Equatable as Swift sees them: compared as the property lists they are, so an unchanged one stays.
        let l = json["lists"] as? [String: [[String: Any]]] ?? [:], s = json["sheets"] as? [String: [String: Any]] ?? [:]
        let u = json["update"] as? [String: Any]
        if (l as NSDictionary) != (lists as NSDictionary) { lists = l }
        if (s as NSDictionary) != (sheets as NSDictionary) { sheets = s }
        if (u as NSDictionary?) != (update as NSDictionary?) { update = u }
        quiet = Set(json["quiet"] as? [String] ?? [])
        hidden = Set(json["hidden"] as? [String] ?? [])
        banner = json["banner"] as? [String: String]
        recording = json["recording"] as? String ?? ""
        notes = json["notes"] as? [String: String] ?? [:]
        if let p = json["profile"] as? [String], p.count == 2 { profile = p }
    }

    private static func themes(_ t: [String: Any]) -> ThemeData? {
        let raw = t["palettes"] as? [String: [Int]] ?? [:]
        let palettes = raw.mapValues { $0.map { UInt32($0) } }.filter { $0.value.count >= 7 }
        guard !palettes.isEmpty else { return nil }
        let pairs = (t["pairs"] as? [[String]] ?? []).filter { $0.count == 3 }.map { ThemeData.Pair(name: $0[0], dark: $0[1], light: $0[2]) }
        return ThemeData(palettes: palettes, names: t["names"] as? [String: String] ?? [:], pairs: pairs,
                         darks: t["darks"] as? [String] ?? [], lights: t["lights"] as? [String] ?? [])
    }
}

struct FontChoice: Hashable {
    var name: String
    var mono: Bool
}

/// The id a word has in the app's catalogs: a slug of its English and the FNV-1a hash of all of it
/// (`scripts/native-words.py` makes the same id). The English is only the source of the id; what shows is the
/// app's word for it.
func wordID(_ english: String) -> String {
    var h: UInt32 = 0x811C9DC5
    for b in english.utf8 { h = (h ^ UInt32(b)) &* 0x01000193 }
    var slug = ""
    for ch in english.lowercased().prefix(40) {
        if ch.isASCII && (ch.isLetter || ch.isNumber) { slug.append(ch) } else if !slug.hasSuffix("-") { slug.append("-") }
    }
    slug = slug.trimmingCharacters(in: CharacterSet(charactersIn: "-"))
    let hex = String(repeating: "0", count: max(0, 8 - String(h, radix: 16).count)) + String(h, radix: 16)
    return slug.isEmpty ? "native-\(hex)" : "native-\(slug)-\(hex)"
}

/// The window's state: the store (values), the catalog (rows) and what only the window knows (page, search).
@MainActor @Observable final class SettingsModel {
    let store: HostStore
    let catalog = SettingsCatalog.standard
    var sel: Sel { didSet { record(from: oldValue) } }
    var search = ""
    /// The pages that show their sub-pages in the sidebar, and the selections back and forward walk.
    var expanded: Set<String> = []
    var back: [Sel] = []
    var fwd: [Sel] = []
    @ObservationIgnored var navigating = false
    var hoverFont: String? = nil
    var specimenTab = "Terminal"
    /// The explanation sheet that is open (a sheet's id), nil when none.
    var infoSheet: String? = nil

    var isSearching: Bool { !search.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }

    init(store: HostStore, page: String) {
        self.store = store; self.sel = Sel(page: page)
    }
    /// A word of the window in the app's language (its English when the app has none); `%@` is where `value` goes.
    func w(_ english: String) -> String { store.words[wordID(english)] ?? english }
    func w(_ english: String, _ value: String) -> String { w(english).replacingOccurrences(of: "%@", with: value) }

    func value(_ key: String) -> String { store.values[key] ?? store.defaults[key] ?? "" }
    func get(_ key: String, _ row: Row) -> String { store.values[key] ?? store.defaults[key] ?? row.defaultValue }
    func isOn(_ key: String, _ row: Row) -> Bool { get(key, row) == "on" }
    func set(_ key: String, _ v: String) { store.set(v, for: key) }
    func options(_ row: Row) -> [Choice] { store.choices[row.key] ?? row.choices }
    var profileInfo: (name: String, letter: String) { store.profile.count == 2 ? (store.profile[0], store.profile[1]) : ("Default", "D") }
    var fontList: [(String, Bool)] { store.fonts.map { ($0.name, $0.mono) } }
    var themeData: ThemeData { store.themes ?? ThemeData() }
    func perform(_ action: String, _ key: String) { store.perform(action, for: key) }

    /// The rows of a group: the catalog's that apply, then the app's own (agents, projects, probes found on this Mac).
    func rows(_ page: Page, _ g: Int) -> [Row] {
        let group = "\(page.id):\(g)"
        return page.groups[g].rows.filter { visible($0, in: page, group: g) && !($0.kind == .update && store.update == nil) }
            + (store.lists[group] ?? []).compactMap { Row(entry: $0, group: group) }
    }
    func sheetData(_ id: String) -> ApprovalSheetData? { store.sheets[id].map(ApprovalSheetData.init(entry:)) }
    /// A plural message from the app (the count it was made for must be the one shown), else the English fallback.
    func counted(_ name: String, _ count: Int, _ fallback: String) -> String { store.counted(name, count) ?? fallback }
    func isChanged(_ key: String) -> Bool { store.changed.contains(key) }

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
        let keys = store.changed
        return catalog.pages.flatMap { p in p.groups.flatMap { $0.rows }.filter { $0.isSetting && keys.contains($0.key) }.map { (p, $0) } }
    }
}
