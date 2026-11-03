import SwiftUI
import AppKit

// The C interface the Wormhole app (Rust) drives the window through. Everything crosses as JSON text: the app
// sends a snapshot of its settings (`wh_settings_update`) and gets back one event per change of the person's
// (`handler`): {"t":"set","k":key,"v":value}, {"t":"reset","k":key}, {"t":"reset_all"},
// {"t":"action","k":key,"v":button's id} and {"t":"closed"}. A menu's value is its option's id, a button's is its id:
// neither is ever the words shown. The window reads and writes no file.

typealias EventHandler = @convention(c) (UnsafePointer<CChar>) -> Void

/// The store over the app's snapshot: reads come from the last snapshot, writes go to the app as events and show at
/// once (the app answers with a new snapshot that agrees or corrects them).
@Observable final class HostStore: SettingsStore {
    private(set) var revision = 0
    private var values: [String: String] = [:]
    private var defaults: [String: String] = [:]
    private var choices: [String: [Choice]] = [:]
    private var changed: Set<String> = []
    private var known: Set<String> = []
    private var themeData: ThemeData?
    private var previewData: PreviewData?
    private var fontList: [FontChoice]?
    private var lists: [String: [[String: Any]]] = [:]
    private var sheets: [String: [String: Any]] = [:]
    private var profileInfo: (name: String, letter: String)?
    private var recordingRow = ""
    private var notes: [String: String] = [:]
    private var updateCard: [String: Any]?
    private var quietPages: Set<String> = []
    private var bannerData: [String: String]?
    private var wordTable: [String: String] = [:]
    private var countedWords: (count: Int, words: [String: String]) = (-1, [:])
    @ObservationIgnored var send: (String) -> Void = { _ in }

    func value(for key: String) -> String? { values[key] }
    func defaultValue(for key: String) -> String? { defaults[key] }
    var changedKeys: Set<String> { changed }
    func options(for key: String) -> [Choice]? { choices[key] }
    var themes: ThemeData? { themeData }
    func word(_ id: String) -> String? { wordTable[id] }
    var preview: PreviewData? { previewData }
    var fonts: [FontChoice]? { fontList }
    func list(for group: String) -> [[String: Any]] { lists[group] ?? [] }
    func sheet(for id: String) -> [String: Any]? { sheets[id] }
    var profile: (name: String, letter: String)? { profileInfo }
    var update: [String: Any]? { updateCard }
    func quiet(_ page: String) -> Bool { quietPages.contains(page) }
    var banner: [String: String]? { bannerData }
    var recordsInHost: Bool { true }
    var recording: String { recordingRow }
    func note(for key: String) -> String? { notes[key] }
    func counted(_ name: String, _ count: Int) -> String? { countedWords.count == count ? countedWords.words[name] : nil }
    func record(_ action: String, for key: String, mods: String, key name: String) {
        emit(["t": "record", "a": action, "k": key, "m": mods, "n": name])
    }

    func set(_ value: String, for key: String) {
        // A row the app does not own keeps what was set until the window closes, so it still reacts.
        if known.contains(key) { values[key] = value } else { values[key] = value; changed.insert(key) }
        revision += 1
        emit(["t": "set", "k": key, "v": value])
    }
    func reset(_ key: String) { values[key] = nil; changed.remove(key); revision += 1; emit(["t": "reset", "k": key]) }
    func resetAll() { values = [:]; changed = []; revision += 1; emit(["t": "reset_all"]) }
    func perform(_ action: String, for key: String) { emit(["t": "action", "k": key, "v": action]) }

    func emit(_ event: [String: String]) {
        guard let data = try? JSONSerialization.data(withJSONObject: event), let text = String(data: data, encoding: .utf8) else { return }
        send(text)
    }

    /// Replace everything the app owns with its snapshot.
    func apply(_ json: [String: Any]) {
        let owned = json["known"] as? [String] ?? []
        let localOnly = values.filter { !known.contains($0.key) }
        known = Set(owned)
        values = (json["values"] as? [String: String] ?? [:]).merging(localOnly) { a, _ in a }
        defaults = json["defaults"] as? [String: String] ?? [:]
        choices = (json["options"] as? [String: [[String]]] ?? [:]).mapValues { $0.filter { $0.count == 2 }.map { Choice(id: $0[0], label: $0[1]) } }
        changed = Set(json["changed"] as? [String] ?? []).union(localOnly.keys)
        if let t = json["themes"] as? [String: Any] { themeData = Self.themes(t) }
        wordTable = json["words"] as? [String: String] ?? [:]
        if let c = json["counts"] as? [String: Any] { countedWords = (c["n"] as? Int ?? -1, c["words"] as? [String: String] ?? [:]) }
        previewData = (json["preview"] as? [String: Any]).flatMap(PreviewData.init(json:))
        if let f = json["fonts"] as? [[Any]] {
            fontList = f.compactMap { e in (e.first as? String).map { FontChoice(name: $0, mono: (e.last as? Bool) ?? false) } }
        }
        lists = json["lists"] as? [String: [[String: Any]]] ?? [:]
        sheets = json["sheets"] as? [String: [String: Any]] ?? [:]
        updateCard = json["update"] as? [String: Any]
        quietPages = Set(json["quiet"] as? [String] ?? [])
        bannerData = json["banner"] as? [String: String]
        recordingRow = json["recording"] as? String ?? ""
        notes = json["notes"] as? [String: String] ?? [:]
        if let p = json["profile"] as? [String], p.count == 2 { profileInfo = (p[0], p[1]) }
        revision += 1
    }

    private static func themes(_ t: [String: Any]) -> ThemeData? {
        let raw = t["palettes"] as? [String: [Int]] ?? [:]
        let palettes = raw.mapValues { $0.map { UInt32($0) } }.filter { $0.value.count >= 7 }
        guard !palettes.isEmpty else { return nil }
        let pairs = (t["pairs"] as? [[String]] ?? []).filter { $0.count == 3 }.map { ThemeData.Pair(name: $0[0], dark: $0[1], light: $0[2]) }
        var data = ThemeData(palettes: palettes, pairs: pairs, darks: t["darks"] as? [String] ?? [], lights: t["lights"] as? [String] ?? [])
        data.names = t["names"] as? [String: String] ?? [:]
        return data
    }
}

/// The one window.
@MainActor final class SettingsHost: NSObject, NSWindowDelegate {
    static let shared = SettingsHost()
    private(set) var window: NSWindow?
    private let store = HostStore()
    private var model: SettingsModel?

    func open(page: String, behind: Bool, handler: EventHandler) {
        store.send = { text in text.withCString { handler($0) } }
        if let window {
            show(window, page: page, behind: behind)
            return
        }
        let model = SettingsModel(store: store, page: page.isEmpty ? "Terminal" : page)
        self.model = model
        let host = NSHostingView(rootView: SettingsView(model: model))
        let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 840, height: 720),
                         styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView], backing: .buffered, defer: false)
        w.title = "Settings"
        w.contentView = host
        w.titlebarAppearsTransparent = true
        w.titleVisibility = .hidden
        w.contentMinSize = NSSize(width: 720, height: 520)
        w.isReleasedWhenClosed = false
        w.delegate = self
        w.setFrameAutosaveName("WormholeSettingsWindow")
        if !w.setFrameUsingName("WormholeSettingsWindow") { w.center() }
        window = w
        show(w, page: nil, behind: behind)
    }

    private func show(_ w: NSWindow, page: String?, behind: Bool) {
        if let page, !page.isEmpty { model?.page = page; model?.search = "" }
        if behind {
            w.orderBack(nil)
        } else {
            NSApp.activate(ignoringOtherApps: true)
            w.makeKeyAndOrderFront(nil)
        }
    }

    func update(_ json: [String: Any]) {
        store.apply(json)
        // Where the app wants the window to go (the palette's setting, a script): a page, a row to scroll to, a search.
        if let show = json["show"] as? [String: Any] {
            if let page = show["page"] as? String, !page.isEmpty { model?.page = page; model?.search = "" }
            if let search = show["search"] as? String { model?.search = search }
            if let row = show["scroll"] as? String, !row.isEmpty { model?.scrollTo = row }
        }
    }

    func close() { window?.close() }

    /// The window's content as a PNG, drawn by the view itself (no screen capture), for tests and bug reports.
    func shot(to path: String) -> Bool {
        guard let v = window?.contentView, let rep = v.bitmapImageRepForCachingDisplay(in: v.bounds) else { return false }
        v.cacheDisplay(in: v.bounds, to: rep)
        guard let data = rep.representation(using: .png, properties: [:]) else { return false }
        return (try? data.write(to: URL(fileURLWithPath: path))) != nil
    }

    func windowWillClose(_ notification: Notification) {
        store.emit(["t": "closed"])
        window?.delegate = nil
        window = nil
        model = nil
    }
}

private func text(_ p: UnsafePointer<CChar>?) -> String? { p.map { String(cString: $0) } }

@_cdecl("wh_settings_open")
public func wh_settings_open(_ page: UnsafePointer<CChar>?, _ behind: Bool, _ handler: @convention(c) (UnsafePointer<CChar>) -> Void) {
    MainActor.assumeIsolated { SettingsHost.shared.open(page: text(page) ?? "", behind: behind, handler: handler) }
}

@_cdecl("wh_settings_update")
public func wh_settings_update(_ json: UnsafePointer<CChar>?) {
    guard let s = text(json), let data = s.data(using: .utf8),
          let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
    MainActor.assumeIsolated { SettingsHost.shared.update(object) }
}

@_cdecl("wh_settings_is_open")
public func wh_settings_is_open() -> Bool { MainActor.assumeIsolated { SettingsHost.shared.window != nil } }

@_cdecl("wh_settings_close")
public func wh_settings_close() { MainActor.assumeIsolated { SettingsHost.shared.close() } }

@_cdecl("wh_settings_shot")
public func wh_settings_shot(_ path: UnsafePointer<CChar>?) -> Bool {
    guard let path = text(path) else { return false }
    return MainActor.assumeIsolated { SettingsHost.shared.shot(to: path) }
}
