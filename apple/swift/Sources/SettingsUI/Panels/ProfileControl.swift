// The Profile page's fields (`profile.rs`): name, letter and colour, where terminals start and the folders that open
// in the profile, the environment, the git identity and the agents' login folders. Text applies on blur or Return,
// a swatch or the folder list at once; what core refuses stays under the fields as its sentence.

import AppKit
import DesignSystem
import SettingsModel
import SidebarUI
import SwiftUI
import WordsModel
import WorkspaceModel

struct ProfileControl: View {
    let draft: ProfileDraft
    let name: String
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .trailing, spacing: Space._2) {
            switch name {
            case "profile-name": identity
            case "profile-start": start
            case "profile-env": env
            case "profile-git": git
            case "profile-logins": logins
            default: EmptyView()
            }
            if draft.problem != nil, name == "profile-name" { problem }
        }
    }

    // MARK: Fields

    private func text(_ path: WritableKeyPath<ProfileForm, String>) -> Binding<String> {
        Binding(get: { draft.form?[keyPath: path] ?? "" }, set: { draft.form?[keyPath: path] = $0 })
    }

    private func field(_ placeholder: String, _ path: WritableKeyPath<ProfileForm, String>, width: CGFloat = SettingsMetric.fieldWidth, id: String? = nil) -> some View {
        InputField(placeholder: placeholder, text: text(path), width: width, frameId: id, onCommit: draft.commit)
    }

    private var identity: some View {
        VStack(alignment: .trailing, spacing: Space._2) {
            HStack(spacing: Space._2) {
                ProfileChip(letter: draft.form?.letter ?? "", color: draft.form?.color ?? "blue")
                field(Words.text("settings-profile-name-field", fallback: "Name"), \.name, width: SettingsMetric.fieldPairWidth, id: "profile.name")
                field("W", \.avatar, width: SettingsMetric.fieldLetterWidth, id: "profile.avatar")
            }
            HStack(spacing: Space._1) {
                ForEach(GroupColor.allCases, id: \.self) { color in
                    Button { draft.edit { $0.color = color.rawValue } } label: {
                        Circle().fill(color.mark(tokens).color).frame(width: Metric.glyph, height: Metric.glyph)
                            .overlay {
                                if draft.form?.color == color.rawValue {
                                    Circle().strokeBorder(tokens.textPrimary, lineWidth: tokens.metrics.focusRingWidth).padding(-Space._1)
                                }
                            }
                            .frame(width: Metric.hitMin, height: Metric.hitMin).contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .recordFrame("profile.color.\(color.rawValue)")
                    .accessibilityLabel(color.rawValue.capitalized)
                    .accessibilityAddTraits(draft.form?.color == color.rawValue ? .isSelected : [])
                }
            }
        }
    }

    private var start: some View {
        VStack(alignment: .trailing, spacing: Space._2) {
            HStack(spacing: Space._2) {
                field(Words.text("settings-profile-the-home-folder", fallback: "~ (the home folder)"), \.startDir, id: "profile.start")
                chooser { path in draft.form?.startDir = path; draft.commit() }
            }
            ForEach(draft.form?.folders ?? [], id: \.self) { folder in
                HStack(spacing: Space._2) {
                    Image(Icon.folder).foregroundStyle(tokens.textMuted)
                    Text(folder).textRole(.body).lineLimit(1).truncationMode(.middle)
                    Spacer(minLength: 0)
                    IconButton(symbol: Icon.remove.rawValue, label: LocalizedStringKey(Words.text("settings-profile-stop-opening-folder", fallback: "Stop opening this folder in the profile"))) {
                        draft.edit { $0.folders.removeAll { $0 == folder } }
                    }
                }
                .frame(width: SettingsMetric.fieldWidth + Metric.control + Space._2, height: Metric.control)
            }
            LinkButton(title: Words.text("settings-profile-add-a-folder", fallback: "Add a folder…")) {
                FolderPanel.choose { path in draft.edit { if !$0.folders.contains(path) { $0.folders.append(path) } } }
            }
        }
    }

    private var env: some View {
        TextEditor(text: text(\.env))
            .scrollContentBackground(.hidden).textRole(.body)
            .padding(Space._1).frame(width: SettingsMetric.fieldBoxWidth, height: Metric.control * 2 + Space._2)
            .background(tokens.inputBackground, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
            .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.inputBorder, lineWidth: tokens.metrics.borderWidth))
            .overlay(alignment: .topLeading) {
                if (draft.form?.env ?? "").isEmpty {
                    Text(Words.text("settings-profile-name-value-one-per-line", fallback: "NAME=value, one per line"))
                        .textRole(.body).foregroundStyle(tokens.textFaint)
                        .padding(.horizontal, Space._2 + Space._1).padding(.top, Space._1 + 1).allowsHitTesting(false)
                }
            }
            .recordFrame("profile.env")
            .accessibilityLabel(Words.text("settings-profile-env", fallback: "Environment"))
            .modifier(CommitOnBlur(commit: draft.commit))
    }

    private var git: some View {
        HStack(spacing: Space._2) {
            field(Words.text("settings-profile-name-field", fallback: "Name"), \.gitName, width: SettingsMetric.fieldPairWidth, id: "profile.git-name")
            field(Words.text("settings-profile-email", fallback: "Email"), \.gitEmail, width: SettingsMetric.fieldPairWidth, id: "profile.git-email")
        }
    }

    private var logins: some View {
        VStack(alignment: .trailing, spacing: Space._2) {
            ForEach(Array((draft.form?.logins ?? []).enumerated()), id: \.offset) { index, login in
                HStack(spacing: Space._2) {
                    Text(login.name).textRole(.body).foregroundStyle(tokens.textMuted)
                        .frame(width: SettingsMetric.agentLabelWidth, alignment: .leading)
                    InputField(
                        placeholder: login.placeholder,
                        text: Binding(get: { draft.form?.logins[index].dir ?? "" }, set: { draft.form?.logins[index].dir = $0 }),
                        onCommit: draft.commit)
                    chooser { path in draft.form?.logins[index].dir = path; draft.commit() }
                }
            }
        }
    }

    // MARK: Pieces

    /// The button that picks a folder for a field, as tall as the field beside it.
    private func chooser(_ picked: @escaping (String) -> Void) -> some View {
        IconButton(symbol: Icon.folder.rawValue, label: LocalizedStringKey(Words.text("settings-profile-choose-a-folder", fallback: "Choose a folder"))) {
            FolderPanel.choose(picked)
        }
        .frame(width: Metric.control, height: Metric.control)
        .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth))
    }

    private var problem: some View {
        Text(draft.problem ?? "").textRole(.caption).foregroundStyle(tokens.statusDangerText)
            .frame(width: SettingsMetric.fieldBoxWidth, alignment: .trailing).fixedSize(horizontal: false, vertical: true)
    }
}

/// A folder picked in the system's panel, as `~/…` under the home folder, else in full (`tilde`).
enum FolderPanel {
    @MainActor static func choose(_ picked: @escaping (String) -> Void) {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = Words.text("settings-profile-choose", fallback: "Choose")
        guard let url = PanelPick.run(panel) else { return }
        picked((url.path as NSString).abbreviatingWithTildeInPath)
    }
}

/// Applies a multi-line field when it loses focus: `TextEditor` has no submit.
private struct CommitOnBlur: ViewModifier {
    let commit: () -> Void
    @FocusState private var focused: Bool

    func body(content: Content) -> some View {
        content.focused($focused).onChange(of: focused) { _, now in if !now { commit() } }
    }
}
