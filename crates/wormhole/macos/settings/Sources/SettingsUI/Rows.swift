import SwiftUI

/// A row's text: label with its chips and (i), and the hint under it when the board shows one.
/// v2 keeps a hint only where it warns or says who a tile row is; every other hint goes behind the (i).
struct RowText: View {
    let row: Row
    var hint: String? = nil
    var stateChip: (String, Color)? = nil
    var changedDot = false
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @State private var showInfo = false

    static let warnHints: Set<String> = ["Terminals, notes and themes are kept", "Applies to new terminals", "Encrypted, on this Mac only. Off erases what’s saved",
        "A side set to Meta can’t type @ { [ on some layouts", "Only for you: nothing is written into the project folder"]

    var keepsHint: Bool { hint != nil || row.tile != nil || [.project, .probe, .value].contains(row.kind) || Self.warnHints.contains(row.hint) }
    var shownHint: String { hint ?? (keepsHint ? m.words(row.key + ".hint", row.hint) : "") }
    var infoText: String {
        [keepsHint ? "" : row.hint, row.info].filter { !$0.isEmpty }.map { m.words(row.key + ".info", $0) }.joined(separator: " ")
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            HStack(spacing: 6) {
                Text(m.words(row.key + ".label", row.label))
                if !row.chip.isEmpty { Chip(text: row.chip, fill: t.primary, ink: t.onPrimary) }
                if changedDot { Circle().fill(t.primary).frame(width: 6, height: 6).help("Changed from its default") }
                if let c = stateChip { Chip(text: c.0, fill: c.1) }
                if !infoText.isEmpty && row.kind != .danger {
                    Button { showInfo.toggle() } label: { Image(systemName: "info.circle").foregroundStyle(.secondary) }
                        .buttonStyle(.borderless)
                        .popover(isPresented: $showInfo) { Text(infoText).padding(Space.s3).frame(width: 260, alignment: .leading).fixedSize(horizontal: false, vertical: true) }
                }
            }
            if !shownHint.isEmpty { Text(shownHint).role(.body).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true) }
            if !row.warnWhenOff.isEmpty && row.kind == .toggle && !m.isOn(row.key, row) { Text(row.warnWhenOff).role(.body).foregroundStyle(t.warning).fixedSize(horizontal: false, vertical: true) }
            if !row.note.isEmpty { Text(row.note).role(.body).foregroundStyle(row.kind == .probe ? t.warning : .secondary) }
        }
    }
}

/// A tile in a tile row: coloured rounded square with a symbol.
struct Tile: View {
    let colour: Color; let symbol: String
    static func symbol(for icon: String?) -> String? { ["claude": "asterisk", "prompt": "chevron.right", "spark": "sparkle"][icon ?? ""] }
    var body: some View {
        RoundedRectangle(cornerRadius: Radius.tile).fill(colour).frame(width: 28, height: 28)
            .overlay(Image(systemName: symbol).font(.system(size: 13)).foregroundStyle(.white))
    }
}

struct RowView: View {
    let row: Row
    var showReset = false
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @State private var confirming = false

    var tall: Bool { row.tile != nil || [.project, .probe, .agent, .skill].contains(row.kind) }
    var tileColour: Color { Color(css: row.tile ?? (row.kind == .project ? "#32ade6" : row.kind == .probe ? "#a2845e" : "#3b4252")) }
    var tileSymbol: String { Tile.symbol(for: row.icon) ?? (row.kind == .project ? "folder.fill" : row.kind == .probe ? "puzzlepiece.extension.fill" : "sparkle") }

    var body: some View {
        switch row.kind {
        case .themes: ThemePicker()
        case .parts: PartsRows()
        case .swatches: ColourRow(row: row)
        case .update: UpdateRow()
        default: line
        }
    }

    var line: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            LabeledContent {
                HStack(spacing: Space.s2) {
                    ControlView(row: row, confirming: $confirming)
                    if showReset {
                        Button { m.store.reset(row.key) } label: { Image(systemName: "arrow.counterclockwise") }.buttonStyle(.borderless).help("Reset to default").frame(width: 20)
                    }
                }
            } label: {
                HStack(spacing: Space.s3) {
                    if tall { Tile(colour: tileColour, symbol: tileSymbol) }
                    RowText(row: row, stateChip: stateChip, changedDot: !showReset && row.isSetting && m.isChanged(row.key))
                }
            }
            if row.kind == .checks || (row.kind == .skill && !row.checks.isEmpty) { ChecksControl(row: row).padding(.leading, tall ? 40 : 0) }
        }
        .confirmationDialog(row.confirm.isEmpty ? "Reset all settings?" : row.confirm, isPresented: $confirming, titleVisibility: .visible) {
            Button(row.action.isEmpty ? (row.actions.first ?? "Reset") : row.action, role: row.kind == .danger ? .destructive : nil) { if row.resetAll { m.store.resetAll() } else { m.perform(row.action.isEmpty ? "confirm" : row.action, row.key) } }
        } message: { Text(row.confirmText) }
    }
    var stateChip: (String, Color)? {
        let v = m.get(row.key, row)
        switch row.kind {
        case .probe:
            switch v { case "on": return ("Approved", Palette.chipGood); case "off": return ("Off", Palette.chipQuiet)
            case "changed": return ("Changed", Palette.chipWarn); case "unsupported": return ("Needs code", Palette.chipQuiet); default: return ("Can’t be enabled", Palette.chipBad) }
        case .project:
            switch v { case "trusted": return ("Trusted", Palette.chipGood); case "changed": return ("Changed since you trusted it", Palette.chipWarn); default: return ("Not trusted", Palette.chipQuiet) }
        case .agent: return v == "repair" ? ("Needs repair", Palette.chipWarn) : nil
        default: return nil
        }
    }
}

/// The control at a row's trailing edge, one view per kind.
struct ControlView: View {
    let row: Row
    @Binding var confirming: Bool
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @State private var approving = false

    var body: some View {
        let cur = m.get(row.key, row)
        switch row.kind {
        case .toggle: SwitchControl(key: row.key, row: row)
        case .popup: PopupControl(row: row)
        case .field: FieldControl(row: row)
        case .font: FontControl(row: row)
        case .key: KeyControl(row: row)
        case .link:
            HStack(spacing: 4) {
                Text(row.fontLink ? "\(m.value("Fonts:0:0")), \(m.value("Fonts:0:0#size")) pt" : row.value).foregroundStyle(.secondary)
                Image(systemName: "chevron.right").font(.system(size: 10, weight: .semibold)).foregroundStyle(.tertiary)
            }.contentShape(Rectangle()).onTapGesture { if row.fontLink { m.search = ""; m.page = "Fonts" } }
        case .button: Btn(row.value) { if row.confirm.isEmpty { m.perform(row.value, row.key) } else { confirming = true } }
        case .danger: Btn(row.value, destructive: true) { confirming = true }
        case .value: Text(row.value).foregroundStyle(.secondary)
        case .agent:
            switch cur {
            case "on": Menu { Button("Repair Hooks") { m.perform("Repair Hooks", row.key) }; Button("Remove Hooks", role: .destructive) { m.perform("Remove Hooks", row.key) } } label: { Text("Hooks on") }.fixedSize()
            case "off": Btn("Add Hooks", primary: true) { m.perform("Add Hooks", row.key) }
            default: Btn("Repair Hooks", primary: true) { m.perform("Repair Hooks", row.key) }
            }
        case .skill: HStack { ForEach(Array(row.actions.enumerated()), id: \.offset) { i, a in Btn(a, primary: row.primary == i) { if a == "Reset All…" { confirming = true } else { m.perform(a, row.key) } } } }
        case .project: Btn(cur == "trusted" ? "Revoke" : "Trust with Touch ID…") { m.perform(cur == "trusted" ? "Revoke" : "Trust with Touch ID…", row.key) }
        case .probe:
            if cur == "off" && !row.sheet.isEmpty { Btn("Turn on…") { approving = true }.sheet(isPresented: $approving) { ApprovalSheet(row: row) { approving = false; m.perform("Approve", row.key) } } }
            else if cur == "on" || cur == "off" { Toggle(row.label, isOn: Binding(get: { cur == "on" }, set: { m.perform($0 ? "Turn on" : "Turn off", row.key) })).labelsHidden().toggleStyle(.switch) }
            else if cur == "changed" && !row.sheet.isEmpty { Btn("Approve Again…") { approving = true }.sheet(isPresented: $approving) { ApprovalSheet(row: row) { approving = false; m.perform("Approve", row.key) } } }
            else { Toggle(row.label, isOn: .constant(false)).labelsHidden().toggleStyle(.switch).disabled(true) }
        default: EmptyView()
        }
    }
}

/// The title row's parts as switch rows; "-" is the flexible space between the left and right groups.
struct PartsRows: View {
    @Environment(SettingsModel.self) var m
    var body: some View {
        ForEach(Array(titlePartLayout.enumerated()), id: \.offset) { _, p in
            if p.name == "-" {
                LabeledContent { EmptyView() } label: { VStack(alignment: .leading, spacing: 1) { Text("Flexible space").foregroundStyle(.secondary); Text("Parts above sit at the left, below at the right").role(.body).foregroundStyle(.secondary) } }
            } else {
                let key = "Appearance:4:0" + p.name
                Toggle(isOn: Binding(get: { m.on(key) }, set: { m.set(key, $0 ? "on" : "off") })) {
                    VStack(alignment: .leading, spacing: 1) {
                        Text(p.name)
                        if !p.note.isEmpty { Text(p.note).role(.body).foregroundStyle(.secondary) }
                    }
                }.toggleStyle(.switch)
            }
        }
    }
}

struct UpdateRow: View {
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    var body: some View {
        LabeledContent {
            Btn("Check Now") { m.perform("Check Now", "Updates:") }
        } label: {
            HStack(spacing: Space.s3) {
                Circle().fill(Palette.green).frame(width: 28, height: 28).overlay(Image(systemName: "checkmark").foregroundStyle(.white))
                VStack(alignment: .leading, spacing: 1) { Text("Wormhole is up to date."); Text("Checked 4 minutes ago.").role(.body).foregroundStyle(.secondary) }
            }
        }
    }
}

/// An extension's approval sheet: what it asks for, before it is turned on.
struct ApprovalSheet: View {
    let row: Row; let approve: () -> Void
    @Environment(\.dismiss) var dismiss
    static let permSymbol = ["run": "terminal", "notify": "bell", "link": "link"]
    @Environment(SettingsModel.self) var m
    var data: ApprovalSheetData? { m.sheetData(row.sheet) }
    var body: some View {
        VStack(alignment: .leading, spacing: Space.s3) {
            Text(data?.title ?? "Approve \(row.label)").font(.headline)
            Text(data?.identity ?? "").foregroundStyle(.secondary)
            ForEach(data?.since ?? [], id: \.self) { Text($0).foregroundStyle(.secondary) }
            GroupBox {
                VStack(alignment: .leading, spacing: Space.s3) {
                    ForEach(Array((data?.perms ?? []).enumerated()), id: \.offset) { _, p in
                        HStack(alignment: .top, spacing: Space.s2) {
                            Image(systemName: Self.permSymbol[p.icon] ?? "circle").frame(width: 18)
                            VStack(alignment: .leading, spacing: 1) {
                                Text(p.title)
                                if !p.detail.isEmpty { Text(p.detail).role(.body).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true) }
                            }
                        }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading).padding(Space.s1)
            }
            HStack { Spacer(); Button("Cancel") { dismiss() }.keyboardShortcut(.cancelAction); Button("Approve", action: approve).keyboardShortcut(.defaultAction) }
        }.padding(Space.s4).frame(width: 380)
    }
}
