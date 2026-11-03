import SwiftUI

/// Changes & diffs: the Changes panel and a diff, split or unified, wrapped or not.
struct DiffPreview: View {
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m
    struct FileRow { let name, badge: String; let depth: Int; let dir: Bool }

    var body: some View {
        let tree = m.str("Git", 0, 0) == "settings_git_tree", untracked = m.on("Git", 0, 1)
        let split = m.str("Git", 1, 0) == "settings_git_split", wrap = m.on("Git", 1, 1)
        let indent = String(repeating: " ", count: Int(m.str("Git", 1, 2).dropFirst()) ?? 4)
        let ln = m.line(t)
        let files: [FileRow] = (tree
            ? [.init(name: "src/", badge: "", depth: 0, dir: true), .init(name: "limits.rs", badge: "M", depth: 1, dir: false), .init(name: "lib.rs", badge: "M", depth: 1, dir: false),
               .init(name: "tests/", badge: "", depth: 0, dir: true), .init(name: "limits.rs", badge: "A", depth: 1, dir: false)]
            : [.init(name: "src/limits.rs", badge: "M", depth: 0, dir: false), .init(name: "src/lib.rs", badge: "M", depth: 0, dir: false), .init(name: "tests/limits.rs", badge: "A", depth: 0, dir: false)])
            + (untracked ? [.init(name: "Untracked", badge: "", depth: 0, dir: true), .init(name: "notes.txt", badge: "U", depth: tree ? 1 : 0, dir: false)] : [.init(name: "notes.txt", badge: "U", depth: 0, dir: false)])
        let long = indent + "let budget = self.refill(now).min(self.capacity * BURST_FACTOR);"
        let del = t.removed.opacity(0.16), add = Palette.added.opacity(0.14)
        func cell(_ s: String, _ bg: Color, strong: Bool) -> some View {
            Text(s.isEmpty ? " " : s).role(.micro, mono: true).foregroundStyle(strong ? t.text : t.muted).lineLimit(wrap ? nil : 1).truncationMode(.tail)
                .padding(.horizontal, 6).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading).background(bg)
        }
        let splitRows: [(String, String, Color?, Color?)] = [
            ("fn take(&mut self) {", "fn take(&mut self) {", nil, nil), (indent + "let budget = self.tokens;", long, del, add),
            (indent + "if budget == 0 {", indent + "if budget == 0 {", nil, nil), ("", indent + indent + "self.stats.denied += 1;", t.text.opacity(0.03), add)]
        let uni: [(String, Color?)] = [(" fn take(&mut self) {", nil), ("-" + indent + "let budget = self.tokens;", del), ("+" + long, add),
            (" " + indent + "if budget == 0 {", nil), ("+" + indent + indent + "self.stats.denied += 1;", add)]
        return HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 1) {
                Text(m.w("Changes · 4")).role(.caption, weight: .semibold).foregroundStyle(t.muted).padding(.horizontal, 4).padding(.bottom, 6)
                ForEach(Array(files.enumerated()), id: \.offset) { _, f in
                    HStack(spacing: 6) {
                        Text(f.name).role(f.dir ? .micro : .caption, weight: f.dir ? .semibold : .regular).lineLimit(1).foregroundStyle(f.dir ? t.muted : t.text)
                        Spacer(minLength: 0)
                        Text(f.badge).role(.micro, mono: true).foregroundStyle(f.badge == "M" ? t.warning : f.badge == "A" ? Palette.green : t.muted)
                    }.padding(.leading, 4 + CGFloat(f.depth) * 12).padding(.trailing, 4).frame(height: 18)
                }
                Spacer(minLength: 0)
            }.padding(.horizontal, 6).padding(.vertical, Space.s2).frame(width: 160)
            Rule(colour: ln, vertical: true)
            VStack(alignment: .leading, spacing: 0) {
                Text("src/limits.rs").role(.caption).foregroundStyle(t.muted).padding(.horizontal, 10).padding(.vertical, 7).frame(maxWidth: .infinity, alignment: .leading)
                Rule(colour: ln)
                if split {
                    ForEach(Array(splitRows.enumerated()), id: \.offset) { _, r in
                        HStack(spacing: 0) {
                            cell(r.0, r.2 ?? .clear, strong: r.2 != nil && r.2 == del)
                            Rule(colour: ln, vertical: true)
                            cell(r.1, r.3 ?? .clear, strong: r.3 != nil)
                        }.fixedSize(horizontal: false, vertical: true)
                    }
                } else {
                    ForEach(Array(uni.enumerated()), id: \.offset) { _, r in cell(r.0, r.1 ?? .clear, strong: r.1 != nil).fixedSize(horizontal: false, vertical: true) }
                }
                Spacer(minLength: 0)
            }.frame(maxWidth: .infinity).clipped()
        }.frame(height: 200).background(t.window)
    }
}
