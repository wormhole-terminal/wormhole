import SwiftUI
import AppKit

/// Native controls wherever macOS has one (Picker, Toggle, Stepper, TextField, ColorPicker, Button).
/// What is custom here has no native equivalent: the shortcut recorder and the chips.

/// A menu of options: a native pop-up, `-` is a divider.
struct PopupControl: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    var body: some View {
        Picker(row.label, selection: Binding(get: { m.get(row.key, row) }, set: { m.set(row.key, $0) })) {
            ForEach(Array(m.options(row).enumerated()), id: \.offset) { _, o in
                if o == "-" { Divider() }
                else if row.appIcons, let img = AppIcons.icon(o) { Label { Text(o) } icon: { Image(nsImage: img) }.tag(o) }
                else { Text(o).tag(o) }
            }
        }.labelsHidden().pickerStyle(.menu).fixedSize()
    }
}

struct SwitchControl: View {
    let key: String; let row: Row; var disabled = false
    @Environment(SettingsModel.self) var m
    var body: some View {
        Toggle(row.label, isOn: Binding(get: { m.isOn(key, row) }, set: { m.set(key, $0 ? "on" : "off") })).labelsHidden().toggleStyle(.switch).disabled(disabled)
    }
}

struct FieldControl: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    var body: some View {
        TextField(row.label, text: Binding(get: { m.get(row.key, row) }, set: { m.set(row.key, $0) })).labelsHidden()
            .textFieldStyle(.plain).multilineTextAlignment(.trailing).frame(width: 200)
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
                if row.same { Text("Same as terminal").tag("Same as terminal"); Divider() }
                Section("Monospaced") { ForEach(m.fontList.filter { $0.1 }, id: \.0) { Text($0.0).tag($0.0) } }
                Section("Proportional") { ForEach(m.fontList.filter { !$0.1 }, id: \.0) { Text($0.0).tag($0.0) } }
            }.labelsHidden().pickerStyle(.menu).fixedSize().font(.system(size: 13, design: row.mono ? .monospaced : .default))
            Stepper("\(size) pt", value: Binding(get: { size }, set: { m.set(row.key + "#size", String($0)) }), in: 8...36).monospacedDigit()
        }
    }
}

/// The shortcut recorder: click, press the keys, they show as key caps. Esc cancels, Delete clears.
struct KeyControl: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @State private var recording = false
    @State private var monitor: Any?

    var body: some View {
        let cur = m.get(row.key, row)
        Button { recording ? stop() : start() } label: {
            if recording { Text("Type shortcut…").foregroundStyle(t.primary) }
            else if cur.isEmpty { Text("Record…") }
            else { HStack(spacing: 3) { ForEach(Array(Self.caps(cur).enumerated()), id: \.offset) { _, c in cap(c) } } }
        }.buttonStyle(.bordered).onDisappear { stop() }
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
    func start() {
        recording = true
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { e in
            if e.keyCode == 53 { stop(); return nil }
            if e.keyCode == 51 { m.set(row.key, ""); stop(); return nil }
            let f = e.modifierFlags
            guard f.contains(.command) || f.contains(.control) || f.contains(.option) else { return nil }
            let mods = (f.contains(.control) ? "⌃" : "") + (f.contains(.option) ? "⌥" : "") + (f.contains(.shift) ? "⇧" : "") + (f.contains(.command) ? "⌘" : "")
            let key = e.keyCode == 49 ? "Space" : (e.charactersIgnoringModifiers ?? "").uppercased()
            m.set(row.key, mods + key); stop(); return nil
        }
    }
    func stop() { recording = false; if let monitor { NSEvent.removeMonitor(monitor) }; monitor = nil }
}

/// A row's checkboxes (Also translucent: Settings, Theme Studio).
struct ChecksControl: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    var body: some View {
        HStack(spacing: Space.s4) {
            ForEach(row.checks, id: \.self) { c in
                Toggle(c, isOn: Binding(get: { row.listed ? row.checksOn.contains(c) : m.on(row.key + c) }, set: { on in if row.listed { m.perform("\(c) \(on ? "on" : "off")", row.key) } else { m.set(row.key + c, on ? "on" : "off") } })).toggleStyle(.checkbox)
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
    let title: String; var primary = false; var destructive = false; let action: () -> Void
    init(_ title: String, primary: Bool = false, destructive: Bool = false, action: @escaping () -> Void = {}) {
        self.title = title; self.primary = primary; self.destructive = destructive; self.action = action
    }
    var body: some View {
        if primary { Button(title, action: action).buttonStyle(.borderedProminent) }
        else if destructive { Button(role: .destructive, action: action) { Text(title).foregroundStyle(Color.red) }.buttonStyle(.bordered) }
        else { Button(title, action: action).buttonStyle(.bordered) }
    }
}

/// The icon of an installed app for the "Open folders in" menu, nil when it isn't installed.
enum AppIcons {
    static let ids = ["Zed": "dev.zed.Zed", "Visual Studio Code": "com.microsoft.VSCode", "Cursor": "com.todesktop.230313mzl4w4u92",
                      "Xcode": "com.apple.dt.Xcode", "Finder": "com.apple.finder"]
    static func icon(_ name: String) -> NSImage? {
        guard let id = ids[name], let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: id) else { return nil }
        let img = NSWorkspace.shared.icon(forFile: url.path); img.size = NSSize(width: 16, height: 16); return img
    }
}
