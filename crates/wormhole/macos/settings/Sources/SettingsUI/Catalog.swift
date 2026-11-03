import SwiftUI

/// What the Settings window shows, as data. A host may hand over its own `SettingsCatalog`;
/// `SettingsCatalog.standard` is the app's own (Pages.swift).
/// Words are a key plus English text: `Row.key + ".label"`, `".hint"`, `Page.id + ".title"`.

public enum Kind: String {
    case field, popup, link, danger, toggle, button, themes, swatches, checks, parts, font, agent, skill, key, update, value, project, probe
}

struct Row: Identifiable {
    var key = ""   // stable setting key "<page>:<group>:<row>", stamped by SettingsCatalog
    var id: String { key }
    let label, hint: String
    let kind: Kind
    var options: [String] = []
    var value = ""
    var info = ""
    var tile: String? = nil
    var actions: [String] = []
    var checks: [String] = []
    var checksOn: [String] = []
    var needs: Int? = nil
    var chip = ""
    var note = ""
    var confirm = ""
    var confirmText = ""
    var size = 13
    var icon: String? = nil
    var primary: Int? = nil
    var sheet = ""
    var swatches = ""
    var warnWhenOff = ""
    var mono = false
    var same = false
    var appIcons = false
    var fontLink = false
    var resetAll = false
    var action = ""
    /// A row the host listed (an agent, a project, a probe): it acts through buttons, never by being set.
    var listed = false

    init(_ label: String, _ hint: String, _ kind: Kind, options: [String] = [], value: String = "", info: String = "",
         tile: String? = nil, actions: [String] = [], checks: [String] = [], checksOn: [String] = [], needs: Int? = nil,
         chip: String = "", note: String = "", confirm: String = "", confirmText: String = "", size: Int = 13,
         icon: String? = nil, primary: Int? = nil, sheet: String = "", swatches: String = "", warnWhenOff: String = "",
         mono: Bool = false, same: Bool = false, appIcons: Bool = false, fontLink: Bool = false, resetAll: Bool = false,
         action: String = "") {
        self.label = label; self.hint = hint; self.kind = kind; self.options = options; self.value = value
        self.info = info; self.tile = tile; self.actions = actions; self.checks = checks; self.checksOn = checksOn
        self.needs = needs; self.chip = chip; self.note = note; self.confirm = confirm; self.confirmText = confirmText
        self.size = size; self.icon = icon; self.primary = primary; self.sheet = sheet; self.swatches = swatches
        self.warnWhenOff = warnWhenOff; self.mono = mono; self.same = same; self.appIcons = appIcons
        self.fontLink = fontLink; self.resetAll = resetAll; self.action = action
    }

    /// The first option for a menu, else the row's value.
    var defaultValue: String { kind == .popup ? (options.first { $0 != "-" } ?? "") : value }
    var isSetting: Bool { [.toggle, .popup, .field, .font, .key].contains(kind) }
}

struct SettingGroup: Identifiable {
    var id: String { title }
    let title: String
    var rows: [Row]
}

struct Page: Identifiable {
    let id, title, preview: String
    var badge = false
    let notice: (String, String, String)?
    let footer: String
    let footerButtons: [String]
    var groups: [SettingGroup]
    init(id: String, title: String, preview: String, notice: (String, String, String)?, footer: String,
         footerButtons: [String], groups: [SettingGroup], badge: Bool = false) {
        self.badge = badge; self.id = id; self.title = title; self.preview = preview; self.notice = notice
        self.footer = footer; self.footerButtons = footerButtons; self.groups = groups
    }
}

struct NavItem { let title, id, colour, symbol: String }

public struct SettingsCatalog {
    var pages: [Page]
    var navItems: [String: NavItem]
    var navGroups: [(String?, [String])]

    init(pages: [Page], navItems: [String: NavItem], navGroups: [(String?, [String])]) {
        // Stamp each row with its key so the rest of the code never rebuilds one.
        self.pages = pages.map { p in
            var p = p
            p.groups = p.groups.enumerated().map { g, grp in
                var grp = grp
                grp.rows = grp.rows.enumerated().map { r, row in var row = row; row.key = "\(p.id):\(g):\(r)"; return row }
                return grp
            }
            return p
        }
        self.navItems = navItems; self.navGroups = navGroups
    }

    public static let standard = SettingsCatalog(pages: standardPages, navItems: standardNav, navGroups: standardNavGroups)

    func page(_ id: String) -> Page? { pages.first { $0.id == id } }
    func row(_ key: String) -> Row? {
        let p = key.split(separator: ":").map(String.init)
        guard p.count == 3, let g = Int(p[1]), let r = Int(p[2]), let page = page(p[0]), g < page.groups.count, r < page.groups[g].rows.count else { return nil }
        return page.groups[g].rows[r]
    }

    /// Every key's default, including the parts of a row (font size, checkboxes, title-row parts).
    public var defaults: [String: String] {
        var d: [String: String] = [:]
        for p in pages { for g in p.groups { for row in g.rows {
            d[row.key] = row.defaultValue
            if row.kind == .font { d[row.key + "#size"] = String(row.size) }
            for c in row.checks { d[row.key + c] = row.checksOn.contains(c) ? "on" : "off" }
            if row.kind == .parts { for (name, on) in titleParts { d[row.key + name] = on ? "on" : "off" } }
        } } }
        return d
    }
}

let standardNav: [String: NavItem] = Dictionary(uniqueKeysWithValues: [
    NavItem(title: "Profile", id: "Profile", colour: "#636366", symbol: "person.fill"),
    NavItem(title: "Changed", id: "Changed", colour: "#3b4252", symbol: "arrow.counterclockwise"),
    NavItem(title: "Terminal", id: "Terminal", colour: "#2c2c2e", symbol: "terminal"),
    NavItem(title: "Appearance", id: "Appearance", colour: "#0a84ff", symbol: "circle.lefthalf.filled"),
    NavItem(title: "Fonts", id: "Fonts", colour: "#636366", symbol: "textformat"),
    NavItem(title: "Terminals & sidebar", id: "Sessions", colour: "#5e5ce6", symbol: "sidebar.left"),
    NavItem(title: "Changes & diffs", id: "Git", colour: "#ff9f0a", symbol: "arrow.triangle.branch"),
    NavItem(title: "Notifications", id: "Notifications", colour: "#ff453a", symbol: "bell.fill"),
    NavItem(title: "Coding agents", id: "Agents", colour: "#bf5af2", symbol: "sparkle"),
    NavItem(title: "Menu bar & hotkey", id: "MenuBar", colour: "#64748b", symbol: "menubar.rectangle"),
    NavItem(title: "Import", id: "Import", colour: "#30b0c7", symbol: "square.and.arrow.down"),
    NavItem(title: "Updates", id: "Updates", colour: "#30d158", symbol: "arrow.triangle.2.circlepath"),
    NavItem(title: "Advanced", id: "Advanced", colour: "#8e8e93", symbol: "slider.horizontal.3"),
    NavItem(title: "Motion", id: "Motion", colour: "#ff375f", symbol: "play.fill"),
    NavItem(title: "Project folders", id: "Projects", colour: "#32ade6", symbol: "folder.fill"),
    NavItem(title: "Extensions", id: "Extensions", colour: "#a2845e", symbol: "puzzlepiece.extension.fill"),
    NavItem(title: "Feature flags", id: "FeatureFlags", colour: "#ff6b35", symbol: "flag.fill"),
].map { ($0.id, $0) })

let standardNavGroups: [(String?, [String])] = [
    (nil, ["Terminal", "Appearance", "Fonts"]),
    (nil, ["Sessions", "Git", "Notifications", "Agents"]),
    (nil, ["MenuBar", "Import", "Updates", "Advanced"]),
    ("Preview", ["Motion", "Projects", "Extensions", "FeatureFlags"]),
]

let previewName = ["window": "Window", "terminal": "Split terminals", "sidebar": "Terminals list", "diff": "Changes panel and diff",
                   "agents": "Terminals list", "banner": "Notifications", "specimen": "Terminal font", "menubar": "Menu bar", "motion": "Terminals list"]
let previewCaption = previewCaptionBase.merging(previewCaptionOverride) { _, new in new }
let previewCaptionBase = [
    "window": "Your theme, opacity, blur, glass, dividers, status bar and title row, as one window draws them.",
    "terminal": "Two split terminals as your settings draw them: the gaps between them and the minimap.",
    "sidebar": "How the list draws a group and its terminals at this density, with the row details you keep.",
    "diff": "The Changes panel and a diff as you set them up: list or tree, split or unified, wrapped or not.",
    "agents": "How the Terminals list shows each agent: working, asking you, finished. With hooks it knows the moment; without, it reads the agent’s output.",
    "banner": "The banners you would get. macOS decides where they show and for how long (System Settings › Notifications › Wormhole).",
    "specimen": "The terminal font at its size and line height. In a font menu, the font under the pointer shows here.",
    "menubar": "The menu bar extra and its menu of terminals waiting for you.",
    "motion": "Loops while you look. Under Reduce Motion it holds still.",
]

/// The title row's parts: name, on by default (Appearance → Title row). The first five sit left of the divider.
let titleParts: [(String, Bool)] = [("Folder", true), ("Branch", true), ("Branch status", true), ("Terminal name", true), ("Terminal status", true),
    ("Agent model and context", true), ("Open ports", true), ("Pull request", false), ("Open in…", true), ("Split buttons", true)]
let titlePartText = ["Folder": "wormhole", "Branch": "main", "Branch status": "↑2", "Terminal name": "cargo test", "Terminal status": "Working",
    "Agent model and context": "Opus · 42%", "Open ports": ":3000", "Pull request": "#128", "Open in…": "Zed", "Split buttons": "◫"]

/// An extension's approval sheet: what it asks for, in the Extensions page's words.
struct ApprovalSheetData {
    let title, identity: String
    let since: [String]
    let perms: [(icon: String, title: String, detail: String)]

    init(entry e: [String: Any]) {
        title = e["title"] as? String ?? ""
        identity = e["identity"] as? String ?? ""
        since = e["since"] as? [String] ?? []
        perms = (e["perms"] as? [[String]] ?? []).filter { $0.count == 3 }.map { ($0[0], $0[1], $0[2]) }
    }
}

extension Row {
    /// A row the host listed: its fields by name. The key is the group's plus the entry's id.
    init?(entry e: [String: Any], group: String) {
        guard let id = e["id"] as? String, let kind = Kind(rawValue: e["kind"] as? String ?? "") else { return nil }
        func s(_ k: String) -> String { e[k] as? String ?? "" }
        func list(_ k: String) -> [String] { e[k] as? [String] ?? [] }
        self.init(s("label"), s("hint"), kind, options: list("options"), value: s("value"), info: s("info"),
                  tile: e["tile"] as? String, actions: list("actions"), checks: list("checks"), checksOn: list("checksOn"),
                  chip: s("chip"), note: s("note"), confirm: s("confirm"), confirmText: s("confirmText"),
                  icon: e["icon"] as? String, primary: e["primary"] as? Int, sheet: s("sheet"), action: s("action"))
        key = "\(group):\(id)"
        listed = true
    }
}
