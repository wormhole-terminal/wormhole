// The field a value is typed into: a hex colour or an expression (`mix(primary, surface.window, 30%)`). It is
// checked as it is typed (what is wrong and where, with the closest name offered), a name being typed completes
// from core's list under it (names and functions, `completions`), and the draft takes the value on Return; meanwhile
// it keeps its last good one. ↑ ↓ move along the list, ⇥ and ↩ take the row, Esc closes the list, a click takes a row.

import DesignSystem
import StudioModel
import SwiftUI

struct ExpressionField: View {
    @Bindable var document: StudioDocument
    let token: String
    @State private var text = ""
    @State private var selection: TextSelection?
    /// The row of the completion list the arrows are on, and whether Esc closed the list.
    @State private var highlighted = 0
    @State private var dismissed = false
    @FocusState private var focused: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        let offered = dismissed ? nil : document.completions(text, caret: caret, for: token)
        let suggestions = offered?.items ?? []
        let row = min(highlighted, max(suggestions.count - 1, 0))
        VStack(alignment: .leading, spacing: Space._1) {
            TextField(StudioWords.expressionPlaceholder, text: $text, selection: $selection)
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
                .onKeyPress(.tab) { take(offered, row) }
                .onKeyPress(.return) { take(offered, row) }
                .onKeyPress(.downArrow) { move(1, in: suggestions) }
                .onKeyPress(.upArrow) { move(-1, in: suggestions) }
                .onKeyPress(.escape) {
                    guard !suggestions.isEmpty else { return .ignored }
                    dismissed = true
                    return .handled
                }
                .accessibilityLabel(StudioWords.sectionValue)
            if let offered, !suggestions.isEmpty { completions(offered, highlighted: row) }
            else if let error = document.fieldError, error.token == token { problem(error) }
        }
        .onAppear(perform: load)
        .onChange(of: token) { load() }
        .onChange(of: document.file) { if !focused { load() } }
    }

    private var shape: RoundedRectangle { RoundedRectangle(cornerRadius: tokens.metrics.radius) }
    /// The field's own edge: red on an error, the input focus colour while it has the caret, else the input's (an always-on
    /// outline, not the keyboard's ring).
    private var border: Color {
        document.fieldError?.token == token ? tokens.statusDanger : focused ? tokens.inputBorderFocus : tokens.inputBorder
    }

    private func load() {
        text = document.file.own(token, in: document.shown) ?? document.row(for: token).text
        if text.hasSuffix("px") { text.removeLast(2) }
        document.validate(token, typing: "")
    }

    /// The caret as a UTF-8 offset, which is how core counts: the end of the text while the field has no selection yet.
    private var caret: Int {
        guard case let .selection(range)? = selection?.indices, range.upperBound <= text.endIndex else { return text.utf8.count }
        return text.utf8.distance(from: text.startIndex, to: range.upperBound)
    }

    /// Takes the row in place of the word typed, the caret after it; the next look at the field offers what follows.
    private func take(_ offered: FieldCompletions?, _ row: Int) -> KeyPress.Result {
        guard let offered, offered.items.indices.contains(row) else { return .ignored }
        take(offered.items[row], from: offered)
        return .handled
    }

    private func take(_ item: FieldCompletion, from offered: FieldCompletions) {
        let taken = offered.taking(item, in: text)
        text = taken.text
        let at = text.utf8.index(text.utf8.startIndex, offsetBy: taken.caret)
        selection = TextSelection(insertionPoint: at)
    }

    private func move(_ step: Int, in names: [FieldCompletion]) -> KeyPress.Result {
        guard !names.isEmpty else { return .ignored }
        highlighted = min(max(highlighted + step, 0), names.count - 1)
        return .handled
    }

    private func completions(_ offered: FieldCompletions, highlighted row: Int) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(offered.items.enumerated()), id: \.offset) { index, item in
                Button { take(item, from: offered) } label: {
                    HStack(spacing: Space._2) {
                        if !item.function { TokenSwatch(chip: .color(document.resolvedColor(item.label)), size: Metric.tileSmall) }
                        Text(item.label).codeFont().lineLimit(1)
                        Spacer(minLength: 0)
                        Text(item.function ? StudioWords.completionFunction : document.groupName(of: item.label) ?? "")
                            .font(TextRole.caption.font).foregroundStyle(tokens.textFaint)
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
