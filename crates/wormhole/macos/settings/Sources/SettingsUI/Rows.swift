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
    var shownHint: String { hint ?? (keepsHint ? m.w(row.hint) : "") }
    var infoText: String {
        [keepsHint ? "" : row.hint, row.info].filter { !$0.isEmpty }.map { m.w($0) }.joined(separator: " ")
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            HStack(spacing: 6) {
                Text(m.w(row.label))
                if !row.chip.isEmpty { Chip(text: m.w(row.chip), fill: t.primary, ink: t.onPrimary) }
                if changedDot { Circle().fill(t.primary).frame(width: 6, height: 6).help(m.w("Changed from its default")).accessibilityHidden(true) }
                if let c = stateChip { Chip(text: c.0, fill: c.1) }
                if !infoText.isEmpty && row.kind != .danger {
                    Button { showInfo.toggle() } label: { Image(systemName: "info.circle").foregroundStyle(.secondary) }
                        .buttonStyle(.borderless)
                        .help(m.w("About %@", m.w(row.label)) + "\n\n" + infoText).accessibilityLabel(m.w(row.label))
                        .popover(isPresented: $showInfo) { Text(infoText).padding(Space.s3).frame(width: 260, alignment: .leading).fixedSize(horizontal: false, vertical: true) }
                }
            }
            if !shownHint.isEmpty { Text(shownHint).role(.body).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true) }
            if !row.warnWhenOff.isEmpty && row.kind == .toggle && !m.isOn(row.key, row) { Text(m.w(row.warnWhenOff)).role(.body).foregroundStyle(t.warning).fixedSize(horizontal: false, vertical: true) }
            if !row.note.isEmpty { Text(m.w(row.note)).role(.body).foregroundStyle(row.kind == .probe ? t.warning : .secondary) }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(m.w(row.label))
        .accessibilityValue([stateChip?.0 ?? "", m.w(row.chip)].filter { !$0.isEmpty }.joined(separator: ", "))
    }
}

struct RowView: View {
    let row: Row
    var showReset = false
    @Environment(SettingsModel.self) var m
    @Environment(\.tok) var t
    @State private var confirming = false

    var tall: Bool { row.tile != nil || [.project, .probe, .agent, .skill, .review].contains(row.kind) }
    var tileColour: Color { Color(css: row.tile ?? (row.kind == .project ? "#32ade6" : row.kind == .probe ? "#a2845e" : "#3b4252")) }
    var tileSymbol: String { Tile.symbol(for: row.icon) ?? (row.kind == .project ? "folder.fill" : row.kind == .probe ? "puzzlepiece.extension.fill" : "sparkle") }
    var spokenHelp: String {
        let changed = row.isSetting && m.isChanged(row.key) ? [m.w("Changed from its default")] : []
        let warning = row.kind == .toggle && !m.isOn(row.key, row) ? m.w(row.warnWhenOff) : ""
        return ([m.w(row.hint), m.w(row.info), m.w(row.note), warning] + changed).filter { !$0.isEmpty }.joined(separator: " ")
    }

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
                        .accessibilityHint(spokenHelp)
                    if showReset {
                        Button { m.store.reset(row.key) } label: { Image(systemName: "arrow.counterclockwise") }
                            .buttonStyle(.borderless).help(m.w("Reset to default")).accessibilityLabel(m.w("Reset %@ to default", m.w(row.label))).frame(width: 20)
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
        .opacity(row.dim ? 0.5 : 1)
        .contextMenu {
            if row.isSetting && !row.listed && m.isChanged(row.key) {
                Button { m.store.reset(row.key) } label: { Label(m.w("Reset to default"), systemImage: "arrow.counterclockwise") }
            }
        }
        .confirmationDialog(row.confirm.isEmpty ? m.w("Reset all settings?") : m.w(row.confirm), isPresented: $confirming, titleVisibility: .visible) {
            Button(m.w(row.action.isEmpty ? (row.actions.first ?? "Reset") : row.action), role: row.kind == .danger ? .destructive : nil) { if row.resetAll { m.store.resetAll() } else { m.perform(row.actionID.isEmpty ? "confirm" : row.actionID, row.key) } }
        } message: { Text(m.w(row.confirmText)) }
    }
    var stateChip: (String, Color)? {
        let v = m.get(row.key, row)
        switch row.kind {
        case .probe:
            switch v { case "on": return (m.w("Approved"), Palette.chipGood); case "off": return (m.w("Off"), Palette.chipQuiet)
            case "changed": return (m.w("Changed"), Palette.chipWarn); case "unsupported": return (m.w("Needs code"), Palette.chipQuiet); default: return (m.w("Can’t be enabled"), Palette.chipBad) }
        case .project:
            switch v { case "trusted": return (m.w("Trusted"), Palette.chipGood); case "changed": return (m.w("Changed since you trusted it"), Palette.chipWarn); default: return (m.w("Not trusted"), Palette.chipQuiet) }
        case .agent: return v == "repair" ? (m.w("Needs repair"), Palette.chipWarn) : nil
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
    @State private var sheeting = false

    var body: some View {
        let cur = m.get(row.key, row)
        switch row.kind {
        case .toggle: SwitchControl(key: row.key, row: row)
        case .popup: PopupControl(row: row)
        case .field: FieldControl(row: row)
        case .font: FontControl(row: row)
        case .key: KeyControl(row: row)
        case .link:
            Button {
                if row.fontLink { m.go("Fonts") } else if !row.sheet.isEmpty { m.infoSheet = row.sheet }
            } label: {
                HStack(spacing: 4) {
                    Text(row.fontLink ? "\(m.value("Fonts:0:0")), \(m.value("Fonts:0:0#size")) pt" : row.value).foregroundStyle(.secondary)
                    Image(systemName: "chevron.forward").font(.system(size: 10, weight: .semibold)).foregroundStyle(.tertiary)
                }.contentShape(Rectangle())
            }.buttonStyle(.borderless).accessibilityLabel(m.w(row.label))
        case .button: Btn(m.get(row.key, row)) { if row.confirm.isEmpty { m.perform(row.actionID.isEmpty ? snakeID(row.value) : row.actionID, row.key) } else { confirming = true } }
        case .danger: Btn(row.value, destructive: true) { confirming = true }
        case .value: Text(m.value(row.key)).foregroundStyle(.secondary)
        case .agent:
            switch cur {
            case "on": Menu { Button(m.w("Repair Hooks")) { m.perform("repair_hooks", row.key) }; Button(m.w("Remove Hooks"), role: .destructive) { m.perform("remove_hooks", row.key) } } label: { Text(m.w("Hooks on")) }.fixedSize()
            case "off": Btn("Add Hooks", primary: true) { m.perform("add_hooks", row.key) }
            default: Btn("Repair Hooks", primary: true) { m.perform("repair_hooks", row.key) }
            }
        case .skill: HStack { ForEach(Array(row.actions.enumerated()), id: \.offset) { i, a in Btn(a, primary: row.primary == i) { if a == "Reset All…" { confirming = true } else { m.perform(row.actionIDs[i], row.key) } } } }
        case .dots:
            HStack(spacing: Space.s2) {
                ForEach(row.dots, id: \.self) { d in
                    Button { m.perform("=" + d[0], row.key) } label: {
                        Circle().fill(Color(css: d[2])).frame(width: 18, height: 18)
                            .overlay { if row.value == d[0] { Circle().strokeBorder(Color(css: d[2]), lineWidth: 2).padding(-4) } }.padding(4)
                    }.buttonStyle(.plain).help(d[1])
                }
            }
        case .folder:
            Button { m.perform(row.actionID, row.key) } label: { Image(systemName: "xmark") }.buttonStyle(.borderless).help(row.action)
        case .review: Btn(row.value) { sheeting = true }.sheet(isPresented: $sheeting) { ImportReviewSheet(row: row) }
        case .project: Btn(cur == "trusted" ? "Revoke" : "Trust with Touch ID…") { m.perform(cur == "trusted" ? "revoke" : "trust", row.key) }
        case .probe:
            if cur == "off" && !row.sheet.isEmpty { Btn("Turn on…") { approving = true }.sheet(isPresented: $approving) { ApprovalSheet(row: row) { ticked in approving = false; m.perform("approve=" + ticked, row.key) } } }
            else if cur == "on" || cur == "off" { Toggle(row.label, isOn: Binding(get: { cur == "on" }, set: { m.perform($0 ? "turn_on" : "turn_off", row.key) })).labelsHidden().toggleStyle(.switch) }
            else if cur == "changed" && !row.sheet.isEmpty { Btn("Approve Again…") { approving = true }.sheet(isPresented: $approving) { ApprovalSheet(row: row) { ticked in approving = false; m.perform("approve=" + ticked, row.key) } } }
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
                LabeledContent { EmptyView() } label: { VStack(alignment: .leading, spacing: 1) { Text(m.w("Flexible space")).foregroundStyle(.secondary); Text(m.w("Parts above sit at the left, below at the right")).role(.body).foregroundStyle(.secondary) } }
            } else {
                let key = "Appearance:4:0#" + titlePartID[p.name, default: p.name]
                Toggle(isOn: Binding(get: { m.on(key) }, set: { m.set(key, $0 ? "on" : "off") })) {
                    VStack(alignment: .leading, spacing: 1) {
                        Text(m.w(p.name))
                        if !p.note.isEmpty { Text(m.w(p.note)).role(.body).foregroundStyle(.secondary) }
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
        if let card = m.store.update { hosted(card) }
    }

    /// The real state, from the app.
    func hosted(_ c: [String: Any]) -> some View {
        let tone = c["tone"] as? String ?? "muted"
        let buttons = (c["buttons"] as? [[String]] ?? []).filter { $0.count == 2 }
        let percent = c["percent"] as? Int ?? -1
        let busy = c["busy"] as? Bool ?? false
        let primary = c["primary"] as? Int ?? -1
        let colour: Color = tone == "ok" ? Palette.green : tone == "warn" ? t.warning : Color.secondary
        let symbol = tone == "ok" ? "checkmark" : tone == "warn" ? "exclamationmark" : "arrow.triangle.2.circlepath"
        return LabeledContent {
            HStack(spacing: Space.s2) {
                if percent >= 0 { ProgressView(value: Double(percent), total: 100).frame(width: 120); Text("\(percent)%").monospacedDigit().foregroundStyle(.secondary) }
                else if busy { ProgressView().controlSize(.small) }
                ForEach(Array(buttons.enumerated()), id: \.offset) { i, b in
                    Btn(b[1], primary: i == primary) { m.perform(b[0], "Updates:") }
                }
            }
        } label: {
            HStack(spacing: Space.s3) {
                Circle().fill(colour).frame(width: 28, height: 28).overlay(Image(systemName: symbol).foregroundStyle(.white))
                VStack(alignment: .leading, spacing: 1) {
                    Text(m.w(c["status"] as? String ?? ""))
                    if let hint = c["hint"] as? String, !hint.isEmpty { Text(hint).role(.body).foregroundStyle(.secondary) }
                }
            }
        }
    }
}

/// An extension's approval sheet: what it asks for, before it is turned on.
struct ApprovalSheet: View {
    let row: Row; let approve: (String) -> Void
    @State private var ticked: Set<String>? = nil
    @Environment(\.dismiss) var dismiss
    static let permSymbol = ["run": "terminal", "notify": "bell", "link": "link"]
    @Environment(SettingsModel.self) var m
    var data: ApprovalSheetData? { m.sheetData(row.sheet) }
    var body: some View {
        let on = ticked ?? Set((data?.perms ?? []).filter { $0.on }.map(\.id))
        VStack(alignment: .leading, spacing: Space.s3) {
            Text(data?.title ?? m.w("Approve %@", row.label)).font(.headline)
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
                            // What the Probe asks for the person may leave off: a combination is a warning, not a grant.
                            if !p.id.isEmpty {
                                Spacer()
                                Toggle(p.title, isOn: Binding(get: { on.contains(p.id) }, set: { v in var next = on; if v { next.insert(p.id) } else { next.remove(p.id) }; ticked = next })).labelsHidden().toggleStyle(.checkbox)
                            }
                        }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading).padding(Space.s1)
            }
            HStack { Spacer(); Button(m.w("Cancel")) { dismiss() }.keyboardShortcut(.cancelAction); Button(m.w("Approve")) { approve(on.sorted { (Int($0) ?? 0) < (Int($1) ?? 0) }.joined(separator: ",")) }.keyboardShortcut(.defaultAction) }
        }.padding(Space.s4).frame(width: 380)
    }
}
