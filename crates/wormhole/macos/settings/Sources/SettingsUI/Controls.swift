import SwiftUI
import UniformTypeIdentifiers
import AppKit

/// Native controls wherever macOS has one (Picker, Toggle, Stepper, TextField, ColorPicker, Button).
/// What is custom here has no native equivalent: the shortcut recorder and the chips.

/// A menu of options: a native pop-up, `-` is a divider.
struct PopupControl: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    var body: some View {
        Picker(row.label, selection: Binding(get: { row.listed ? row.value : m.get(row.key, row) }, set: { v in
            if row.listed { m.perform("=" + v, row.key) }
            // The menu's last items are not values: they open a panel or the config file, and the menu keeps its pick.
            else if v == "choose_app" { FilePanel.app(m, row.key) }
            else if v == "edit_shells" { m.perform("edit_shells", row.key) }
            else { m.set(row.key, v) }
        })) {
            ForEach(Array(m.options(row).enumerated()), id: \.offset) { _, o in
                if o == .divider { Divider() }
                else if row.appIcons, let img = AppIcons.icon(bundle: o.id) ?? AppIcons.icon(named: o.label) { Label { Text(m.w(o.label)) } icon: { Image(nsImage: img) }.tag(o.id) }
                else { Text(m.w(o.label)).tag(o.id) }
            }
        }.labelsHidden().pickerStyle(.menu).fixedSize()
    }
}

struct SwitchControl: View {
    let key: String; let row: Row; var disabled = false
    @Environment(SettingsModel.self) var m
    var body: some View {
        Toggle(row.label, isOn: Binding(get: { row.listed ? row.value == "on" : m.isOn(key, row) }, set: { on in if row.listed { m.perform(on ? "on" : "off", key) } else { m.set(key, on ? "on" : "off") } })).labelsHidden().toggleStyle(.switch).disabled(disabled)
    }
}

struct FieldControl: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    @State private var draft = ""
    @FocusState private var focused: Bool
    var body: some View {
        if row.listed {
            // A profile's text: written when the person is done with it (return or leaving the field), not per key.
            TextField(row.label, text: $draft, axis: row.multiline ? .vertical : .horizontal).lineLimit(row.multiline ? 1...5 : 1...1)
                .labelsHidden().textFieldStyle(.plain).multilineTextAlignment(.trailing).frame(width: row.multiline ? 260 : 200)
                .focused($focused)
                .onAppear { draft = row.value }
                .onChange(of: row.value) { _, v in if !focused { draft = v } }
                .onChange(of: focused) { _, f in if !f && draft != row.value { m.perform("=" + draft, row.key) } }
                .onSubmit { if draft != row.value { m.perform("=" + draft, row.key) } }
        } else {
            TextField(row.label, text: Binding(get: { m.get(row.key, row) }, set: { m.set(row.key, $0) })).labelsHidden()
                .textFieldStyle(.plain).multilineTextAlignment(.trailing).frame(width: 200)
        }
    }
}

/// A font row: the family in a native menu, the size in a native stepper.
struct FontControl: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    var body: some View {
        let size = Int(m.value(row.key + "#size")) ?? row.size
        HStack(spacing: Space.s2) {
            Picker(row.label, selection: Binding(get: { m.get(row.key, row) }, set: { m.set(row.key, $0) })) {
                if row.same { Text(m.w("Same as terminal")).tag("same_as_terminal"); Divider() }
                Section(m.w("Monospaced")) { ForEach(m.fontList.filter { $0.1 }, id: \.0) { Text($0.0).tag($0.0) } }
                Section(m.w("Proportional")) { ForEach(m.fontList.filter { !$0.1 }, id: \.0) { Text($0.0).tag($0.0) } }
            }.labelsHidden().pickerStyle(.menu).fixedSize().font(.system(size: 13, design: row.mono ? .monospaced : .default))
            Stepper("\(size) pt", value: Binding(get: { size }, set: { m.set(row.key + "#size", String($0)) }), in: 8...36).monospacedDigit()
        }
    }
}

/// The shortcut recorder: click, press the keys, they show as key caps. Esc cancels, Delete clears. The app decides
/// what the keys mean (it refuses taken ones and says who has them) and which row records; this only captures them.
struct KeyControl: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @State private var monitor: Any?

    private var recording: Bool { m.store.recording == row.key }

    var body: some View {
        let cur = m.get(row.key, row)
        VStack(alignment: .trailing, spacing: 4) {
            Button { toggle() } label: {
                if recording { Text(m.w("Type shortcut…")).foregroundStyle(t.primary) }
                else if cur.isEmpty { Text(m.w("Record…")) }
                else { HStack(spacing: 3) { ForEach(Array(Self.caps(cur).enumerated()), id: \.offset) { _, c in cap(c) } } }
            }.buttonStyle(.bordered)
            if let note = m.store.notes[row.key] {
                Text(note).role(.caption).foregroundStyle(.orange).multilineTextAlignment(.trailing)
            }
        }
        .onChange(of: recording) { _, on in on ? listen() : unlisten() }
        .onAppear { if recording { listen() } }
        .onDisappear { unlisten(); if recording { m.store.record("stop", for: row.key) } }
    }

    func cap(_ s: String) -> some View {
        Text(s).role(.caption, weight: .medium).frame(minWidth: 14).padding(.horizontal, 3).frame(height: 18)
            .background(RoundedRectangle(cornerRadius: Radius.chip).fill(.quaternary))
    }
    static func caps(_ s: String) -> [String] {
        var out: [String] = []; var rest = ""
        for ch in s { if "⌃⌥⇧⌘".contains(ch) { out.append(String(ch)) } else { rest.append(ch) } }
        if !rest.isEmpty { out.append(rest) }
        return out
    }

    func toggle() { m.store.record(recording ? "stop" : "start", for: row.key) }

    /// Capture keys while recording; every key is swallowed so none reaches the window.
    func listen() {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { e in
            m.store.record("key", for: row.key, mods: Self.mods(e.modifierFlags), key: Self.name(e))
            return nil
        }
    }
    func unlisten() { if let monitor { NSEvent.removeMonitor(monitor) }; monitor = nil }

    static func mods(_ f: NSEvent.ModifierFlags) -> String {
        [(f.contains(.control), "ctrl"), (f.contains(.option), "alt"), (f.contains(.shift), "shift"), (f.contains(.command), "cmd")]
            .filter { $0.0 }.map { $0.1 }.joined(separator: ",")
    }

    /// The key as the app names it (GPUI's names): letters and digits lowercase, the rest by key code.
    static func name(_ e: NSEvent) -> String {
        if let special = named[e.keyCode] { return special }
        return (e.charactersIgnoringModifiers ?? "").lowercased()
    }
    private static let named: [UInt16: String] = [
        49: "space", 36: "enter", 76: "enter", 48: "tab", 53: "escape", 51: "backspace", 117: "delete",
        123: "left", 124: "right", 125: "down", 126: "up", 115: "home", 119: "end", 116: "pageup", 121: "pagedown",
        122: "f1", 120: "f2", 99: "f3", 118: "f4", 96: "f5", 97: "f6", 98: "f7", 100: "f8", 101: "f9", 109: "f10",
        103: "f11", 111: "f12", 105: "f13", 107: "f14", 113: "f15", 106: "f16", 64: "f17", 79: "f18", 80: "f19", 90: "f20",
    ]
}

/// A row's checkboxes (Also translucent: Settings, Theme Studio).
struct ChecksControl: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    var body: some View {
        HStack(spacing: Space.s4) {
            ForEach(Array(row.checks.enumerated()), id: \.offset) { i, c in
                let id = row.checkIDs[i], key = row.key + "#" + id
                Toggle(m.w(c), isOn: Binding(get: { row.listed ? row.checksOn.contains(id) : m.on(key) }, set: { on in if row.listed { m.perform("\(id) \(on ? "on" : "off")", row.key) } else { m.set(key, on ? "on" : "off") } })).toggleStyle(.checkbox)
            }
        }
    }
}

struct Chip: View {
    let text: String; let fill: Color; var ink: Color = .white
    var body: some View { Text(text).role(.caption, weight: .semibold).foregroundStyle(ink).padding(.horizontal, 7).frame(height: 16).background(Capsule().fill(fill)) }
}

/// A button: bordered like any native one, prominent when it is the row's main action.
struct Btn: View {
    @Environment(SettingsModel.self) var m
    let title: String; var primary = false; var destructive = false; let action: () -> Void
    init(_ title: String, primary: Bool = false, destructive: Bool = false, action: @escaping () -> Void = {}) {
        self.title = title; self.primary = primary; self.destructive = destructive; self.action = action
    }
    var body: some View {
        let title = m.w(self.title)
        if primary { Button(title, action: action).buttonStyle(.borderedProminent) }
        else if destructive { Button(role: .destructive, action: action) { Text(title).foregroundStyle(Color.red) }.buttonStyle(.bordered) }
        else { Button(title, action: action).buttonStyle(.bordered) }
    }
}

/// The icon of an installed app for the "Open folders in" menu, nil when it isn't installed.
enum AppIcons {
    static let ids = ["Zed": "dev.zed.Zed", "Visual Studio Code": "com.microsoft.VSCode", "Cursor": "com.todesktop.230313mzl4w4u92",
                      "Xcode": "com.apple.dt.Xcode", "Finder": "com.apple.finder"]
    static func icon(named name: String) -> NSImage? { ids[name].flatMap { icon(bundle: $0) } }
    static func icon(bundle id: String) -> NSImage? {
        guard let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: id) else { return nil }
        let img = NSWorkspace.shared.icon(forFile: url.path); img.size = NSSize(width: 16, height: 16); return img
    }
}

/// The file panels of the window: they run here, as a sheet of the Settings window, and hand the host the path.
enum FilePanel {
    /// "Choose app…": an app from anywhere; the host keeps it by its bundle identifier.
    @MainActor static func app(_ m: SettingsModel, _ key: String) {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.application]
        panel.canChooseDirectories = false; panel.allowsMultipleSelection = false
        panel.directoryURL = URL(fileURLWithPath: "/Applications")
        panel.prompt = m.w("Choose")
        run(panel) { m.perform("app:" + $0.path, key) }
    }
    /// "Import Theme…": a theme file; the host reads it.
    @MainActor static func theme(_ m: SettingsModel, _ key: String) {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = false; panel.allowsMultipleSelection = false
        panel.prompt = m.w("Import")
        run(panel) { m.perform("import:" + $0.path, key) }
    }
    @MainActor private static func run(_ panel: NSOpenPanel, _ done: @escaping @MainActor (URL) -> Void) {
        let finish: (NSApplication.ModalResponse) -> Void = { response in
            if response == .OK, let url = panel.url { Task { @MainActor in done(url) } }
        }
        if let window = NSApp.keyWindow { panel.beginSheetModal(for: window, completionHandler: finish) } else { panel.begin(completionHandler: finish) }
    }
}
