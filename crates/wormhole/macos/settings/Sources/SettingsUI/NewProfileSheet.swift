import SwiftUI

/// The "New profile" sheet: a name and what the profile starts from. The host says when it is up and what it says
/// (the sheet "new-profile"); Create and Cancel go back as actions, and a refusal comes back as its `error`.
struct NewProfileSheet: View {
    @Environment(SettingsModel.self) var m
    @State private var name = ""
    @State private var from: String?
    @FocusState private var focused: Bool
    var data: [String: Any] { m.sheetDict("new-profile") ?? [:] }
    var options: [[String]] { data["options"] as? [[String]] ?? [] }

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s3) {
            Text(data["title"] as? String ?? "").font(.headline)
            TextField(data["name"] as? String ?? "", text: $name).textFieldStyle(.roundedBorder).focused($focused).onSubmit(create)
            Picker(data["from"] as? String ?? "", selection: Binding(get: { from ?? data["chosen"] as? String ?? "" }, set: { from = $0 })) {
                ForEach(options.filter { $0.count == 2 }, id: \.self) { Text($0[1]).tag($0[0]) }
            }
            Text(data["note"] as? String ?? "").role(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            if let error = data["error"] as? String, !error.isEmpty { Text(error).role(.caption).foregroundStyle(.red) }
            HStack {
                Spacer()
                Button(data["cancel"] as? String ?? "") { m.perform("new_profile_cancel", "Profile:") }.keyboardShortcut(.cancelAction)
                Button(data["create"] as? String ?? "", action: create).keyboardShortcut(.defaultAction)
            }
        }.padding(Space.s4).frame(width: 380).onAppear { focused = true }
    }

    func create() {
        m.perform("new_profile_create:\(from ?? data["chosen"] as? String ?? "")\n\(name)", "Profile:")
    }
}
