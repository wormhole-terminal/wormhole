// A banner (`style::banner`): the kind's glyph, one line in label size, and the buttons at the trailing end; over a
// pane or in a page. Tinted fill in the control corners; Increase Contrast draws the kind's outline.

import SwiftUI

/// What a banner says it is: its glyph and its `banner.*` colours.
public enum BannerKind: Sendable, Equatable { case info, warning, danger, success }

/// A tinted notice with one line of words and, at the end, the buttons that go with it.
public struct Banner<Actions: View>: View {
    let kind: BannerKind
    let text: String
    let actions: Actions
    @Environment(\.tokens) private var tokens

    /// Where the view that asked for it was built, for inspect mode.
    let file: StaticString
    let line: UInt

    public init(kind: BannerKind, text: String, file: StaticString = #fileID, line: UInt = #line, @ViewBuilder actions: () -> Actions) {
        (self.kind, self.text, self.actions) = (kind, text, actions())
        (self.file, self.line) = (file, line)
    }

    public var body: some View {
        let shape = RoundedRectangle(cornerRadius: tokens.metrics.radius)
        HStack(spacing: Space._3) {
            Image(systemName: symbol).font(.system(size: Metric.titleGlyph)).foregroundStyle(glyphColor)
            Text(text).textRole(.label, weight: .regular).foregroundStyle(tokens.textPrimary)
                .frame(maxWidth: .infinity, alignment: .leading)
            // The words give way, never the buttons' labels.
            actions.fixedSize()
        }
        .padding(.leading, Space._3).padding(.trailing, Space._2).padding(.vertical, Self.padY)
        .frame(minHeight: Self.minHeight)
        .background(fill, in: shape)
        .contrastOutline(shape, color: border)
        .marked("banner", style: "padding leading _3 trailing _2 · min height 40 · gap _3 · radius control · text label", file: file, line: line)  // text: inspect mode's note, in the scales' names
        .accessibilityElement(children: .contain)
        .accessibilityLabel(text)
    }

    /// `BANNER_MIN_H` and `BANNER_PAD_Y` (`style/scale.rs`, `style/messages.rs`).
    static var minHeight: CGFloat { 40 }
    private static var padY: CGFloat { 6 }

    private var symbol: String {
        switch kind {
        case .info: "info.circle"
        case .warning, .danger: "exclamationmark.triangle"
        case .success: "checkmark.shield"
        }
    }
    private var glyphColor: Color {
        switch kind {
        case .info: tokens.statusInfo
        case .warning: tokens.statusWarning
        case .danger: tokens.statusDanger
        case .success: tokens.statusSuccess
        }
    }
    private var fill: Color {
        switch kind {
        case .info: tokens.bannerInfoBackground
        case .warning: tokens.bannerWarningBackground
        case .danger: tokens.bannerDangerBackground
        case .success: tokens.bannerSuccessBackground
        }
    }
    private var border: Color {
        switch kind {
        case .info: tokens.bannerInfoBorder
        case .warning: tokens.bannerWarningBorder
        case .danger: tokens.bannerDangerBorder
        case .success: tokens.bannerSuccessBorder
        }
    }
}

extension Banner where Actions == EmptyView {
    public init(kind: BannerKind, text: String, file: StaticString = #fileID, line: UInt = #line) {
        self.init(kind: kind, text: text, file: file, line: line) { EmptyView() }
    }
}
