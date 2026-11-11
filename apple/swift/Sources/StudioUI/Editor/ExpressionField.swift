// The field a value is typed into: a hex colour or an expression (`mix(primary, surface.window, 30%)`). It is
// checked as it is typed (what is wrong and where, with the closest name offered), a name being typed completes
// from the list under it, and the draft takes the value on Return; meanwhile it keeps its last good one.

import DesignSystem
import StudioModel
import SwiftUI

struct ExpressionField: View {
    @Bindable var document: StudioDocument
    let token: String
    @State private var text = ""
    /// The row of the completion list the arrows are on, and whether Esc closed the list.
    @State private var highlighted = 0
    @State private var dismissed = false
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        let suggestions = dismissed ? [] : NameCompletion.suggestions(for: text, among: document.expressionNames)
        let row = min(highlighted, max(suggestions.count - 1, 0))
        VStack(alignment: .leading, spacing: Space._1) {
            TextField(StudioWords.expressionPlaceholder, text: $text)
                .textFieldStyle(.plain).codeFont().focused($focused)
                .padding(.horizontal, Space._2).frame(height: Metric.control)
                .background(tokens.inputBackground, in: shape)
                .overlay(shape.strokeBorder(border, lineWidth: tokens.metrics.borderWidth))
                .onSubmit { document.set(token, to: text) }
                .onChange(of: text) { _, new in
                    (highlighted, dismissed) = (0, false)
                    if focused { document.validate(token, typing: new) }
                }
                // ↑ ↓ move in the list, Tab and Return take the row, Esc closes the list before it closes the window.
                .onKeyPress(.tab) { take(suggestions, row) }
                .onKeyPress(.return) { take(suggestions, row) }
                .onKeyPress(.downArrow) { move(1, in: suggestions) }
                .onKeyPress(.upArrow) { move(-1, in: suggestions) }
                .onKeyPress(.escape) {
                    guard !suggestions.isEmpty else { return .ignored }
                    dismissed = true
                    return .handled
                }
                .accessibilityLabel(StudioWords.sectionValue)
            if !suggestions.isEmpty { completions(suggestions, highlighted: row) }
            else if let error = document.fieldError, error.token == token { problem(error) }
        }
        .onAppear(perform: load)
        .onChange(of: token) { load() }
        .onChange(of: document.file) { if !focused { load() } }
    }

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: tokens.metrics.radius) }
    private var border: Color {
        document.fieldError?.token == token ? tokens.statusDanger : focused ? tokens.inputBorderFocus : tokens.inputBorder
    }

    private func load() {
        text = document.file.own(token, in: document.shown) ?? document.row(for: token).text
        if text.hasSuffix("px") { text.removeLast(2) }
        document.validate(token, typing: "")
    }

    private func take(_ names: [String], _ row: Int) -> KeyPress.Result {
        guard names.indices.contains(row) else { return .ignored }
        text = NameCompletion.completing(text, with: names[row])
        return .handled
    }

    private func move(_ step: Int, in names: [String]) -> KeyPress.Result {
        guard !names.isEmpty else { return .ignored }
        highlighted = min(max(highlighted + step, 0), names.count - 1)
        return .handled
    }

    private func completions(_ names: [String], highlighted row: Int) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(names.enumerated()), id: \.element) { index, name in
                Button { text = NameCompletion.completing(text, with: name) } label: {
                    HStack(spacing: Space._2) {
                        TokenSwatch(chip: .color(document.resolvedColor(name)), size: Metric.tileSmall)
                        Text(name).codeFont()
                        Spacer(minLength: 0)
                    }
                    .padding(.horizontal, Space._2).frame(height: Metric.controlSmall).contentShape(Rectangle())
                    .background(index == row ? tokens.listSelected : .clear)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.vertical, Space._1)
        .background(tokens.surfaceElevated, in: RoundedRectangle(cornerRadius: tokens.metrics.radius))
        .overlay(RoundedRectangle(cornerRadius: tokens.metrics.radius).strokeBorder(tokens.border, lineWidth: tokens.metrics.borderWidth))
    }

    private func problem(_ error: FieldError) -> some View {
        VStack(alignment: .leading, spacing: Space._1) {
            Text(error.message).font(TextRole.caption.font).foregroundStyle(tokens.statusDanger.opacity(1))
                .fixedSize(horizontal: false, vertical: true)
            if let unknown = NameCompletion.unknownName(in: error.message),
                let near = NameCompletion.closest(to: unknown, among: document.expressionNames)
            {
                Button { text = text.replacingOccurrences(of: unknown, with: near); document.validate(token, typing: text) } label: {
                    Text("\(StudioWords.didYouMean) `\(near)`?").font(TextRole.caption.font).foregroundStyle(tokens.textLink)
                }
                .buttonStyle(.plain)
            }
        }
        .accessibilityElement(children: .combine)
    }
}
