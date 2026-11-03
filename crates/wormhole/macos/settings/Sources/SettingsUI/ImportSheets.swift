import SwiftUI
import AppKit

/// The Import page's two sheets. Their content comes from the host as data (its words, its terminals' proposals); the
/// review's ticks are this view's until Import sends the ones left on.

/// What a terminal's settings would bring: a tick per setting, what the other terminal wrote and what would be set.
struct ImportReviewSheet: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    @Environment(\.dismiss) var dismiss
    @State private var ticked: Set<String>? = nil

    var data: [String: Any] { m.sheetDict(row.sheet) ?? [:] }
    var rows: [[String: Any]] { data["rows"] as? [[String: Any]] ?? [] }
    var skipped: [[String]] { data["skipped"] as? [[String]] ?? [] }
    func word(_ key: String) -> String { data[key] as? String ?? "" }

    var body: some View {
        let on = ticked ?? Set(rows.filter { $0["on"] as? Bool ?? false }.compactMap { $0["id"] as? String })
        VStack(alignment: .leading, spacing: Space.s3) {
            Text(word("title")).font(.headline)
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s2) {
                    Text(word("take")).role(.caption, weight: .semibold).foregroundStyle(.secondary)
                    if rows.isEmpty { Text(word("nothing")).foregroundStyle(.secondary) }
                    ForEach(rows, id: \.self.description) { r in
                        let id = r["id"] as? String ?? ""
                        VStack(alignment: .leading, spacing: 2) {
                            Toggle(isOn: Binding(get: { on.contains(id) }, set: { v in var next = on; if v { next.insert(id) } else { next.remove(id) }; ticked = next })) {
                                HStack(spacing: Space.s2) {
                                    Text(r["label"] as? String ?? "")
                                    Spacer()
                                    Text(r["value"] as? String ?? "").foregroundStyle(.secondary).lineLimit(1)
                                }
                            }.toggleStyle(.checkbox)
                            Text(r["from"] as? String ?? "").role(.caption, mono: true).foregroundStyle(.tertiary).lineLimit(1).padding(.leading, 20)
                            if let warn = r["warn"] as? String, !warn.isEmpty { Text(warn).role(.caption).foregroundStyle(.orange).padding(.leading, 20) }
                        }
                    }
                    if !skipped.isEmpty {
                        Text(word("leave")).role(.caption, weight: .semibold).foregroundStyle(.secondary).padding(.top, Space.s2)
                        ForEach(Array(skipped.enumerated()), id: \.offset) { _, s in
                            VStack(alignment: .leading, spacing: 1) {
                                Text(s.first ?? "").role(.caption, mono: true).foregroundStyle(.secondary).lineLimit(1)
                                Text(s.last ?? "")
                            }
                        }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading)
            }.frame(maxHeight: 360)
            HStack {
                Spacer()
                Button(m.w("Cancel")) { dismiss() }.keyboardShortcut(.cancelAction)
                Button(word("apply")) {
                    m.perform("apply=" + on.sorted { (Int($0) ?? 0) < (Int($1) ?? 0) }.joined(separator: ","), row.key)
                    dismiss()
                }.keyboardShortcut(.defaultAction).disabled(on.isEmpty)
            }
        }.padding(Space.s4).frame(width: 460)
    }
}

/// How each terminal found compares with Wormhole: a strength and today's drawbacks, each with its sources.
struct CompareSheet: View {
    let row: Row
    @Environment(SettingsModel.self) var m
    @Environment(\.dismiss) var dismiss

    var data: [String: Any] { m.sheetDict(row.sheet) ?? [:] }
    var sections: [[String: Any]] { data["sections"] as? [[String: Any]] ?? [] }

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s3) {
            Text(data["title"] as? String ?? "").font(.headline)
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s4) {
                    if sections.isEmpty { Text(data["none"] as? String ?? "").foregroundStyle(.secondary) }
                    ForEach(Array(sections.enumerated()), id: \.offset) { _, s in section(s) }
                }.frame(maxWidth: .infinity, alignment: .leading)
            }.frame(maxHeight: 420)
            HStack { Spacer(); Button(data["done"] as? String ?? "") { dismiss() }.keyboardShortcut(.defaultAction) }
        }.padding(Space.s4).frame(width: 520)
    }

    func section(_ s: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            HStack {
                Text(s["title"] as? String ?? "").font(.headline)
                if let a = s["asOf"] as? String, !a.isEmpty { Text(a).role(.caption).foregroundStyle(.secondary) }
            }
            if let st = s["strength"] as? String, !st.isEmpty { Text(st).foregroundStyle(.secondary) }
            Text(s["label"] as? String ?? "").role(.caption, weight: .semibold).foregroundStyle(.secondary)
            ForEach(Array((s["gaps"] as? [[String: Any]] ?? []).enumerated()), id: \.offset) { _, g in
                Divider()
                HStack(alignment: .top) {
                    VStack(alignment: .leading, spacing: 1) {
                        Text(g["text"] as? String ?? "").fixedSize(horizontal: false, vertical: true)
                        if let a = g["asOf"] as? String, !a.isEmpty { Text(a).role(.caption).foregroundStyle(.secondary) }
                    }
                    Spacer()
                    ForEach(Array((g["sources"] as? [[String]] ?? []).enumerated()), id: \.offset) { _, src in
                        if src.count == 2, let url = URL(string: src[1]) {
                            Button { NSWorkspace.shared.open(url) } label: { Label(src[0], systemImage: "arrow.up.right.square") }.buttonStyle(.link)
                        }
                    }
                }
            }
        }
    }
}
