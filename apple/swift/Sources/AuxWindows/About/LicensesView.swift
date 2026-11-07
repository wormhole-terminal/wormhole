// The Licenses window (Help > Licenses): every package Wormhole is built from on the left, searchable, and the
// selected one's license on the right (`about/licenses.rs`). The list runs under the traffic lights, like the
// settings window's sidebar. Rows are drawn in the theme's selection colours; ↑ and ↓ move the selection from the
// search field or the list, and the selection scrolls into view.

import AuxModel
import DesignSystem
import SwiftUI

enum LicensesMetrics {
    /// The list column's width (`LIST_WIDTH`) and the window's sizes (`SIZE`, `MIN_SIZE` of `about/licenses.rs`).
    static let listWidth: CGFloat = 300
    static let size = CGSize(width: 940, height: 660)
    static let minSize = CGSize(width: 640, height: 420)
}

struct LicensesView: View {
    @State var browser: LicenseBrowser
    @Environment(\.tokens) private var tokens
    @FocusState private var searching: Bool

    var body: some View {
        HStack(spacing: 0) {
            sidebar.frame(width: LicensesMetrics.listWidth)
            Rectangle().fill(tokens.divider).frame(width: tokens.metrics.dividerWidth)
            detail.padding(.top, Metric.titleRow)
        }
        .auxPage(titleRow: false)
        .onAppear { searching = true }
    }

    private var sidebar: some View {
        VStack(spacing: 0) {
            TextField(AuxText.licensesSearch, text: $browser.query)
                .textFieldStyle(.roundedBorder).focused($searching)
                .padding(.horizontal, Space._4).padding(.top, Metric.titleRow).padding(.bottom, Space._2)
                .accessibilityLabel(AuxText.licensesSearch)
                .onKeyPress(.downArrow) { move(1) }
                .onKeyPress(.upArrow) { move(-1) }
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 0) {
                        ForEach(browser.matches, id: \.self) { index in
                            PackageRow(package: browser.list.packages[index], selected: browser.selected == index)
                                .onTapGesture { browser.selected = index }
                                .id(index)
                        }
                    }
                    .padding(.horizontal, Space._2)
                }
                .onChange(of: browser.selected) { _, selected in
                    if let selected { proxy.scrollTo(selected) }
                }
            }
            Text(browser.countLine).font(TextRole.caption.font).foregroundStyle(tokens.textMuted)
                .padding(.horizontal, Space._4).padding(.vertical, Space._2).frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxHeight: .infinity)
        .background(tokens.surfacePanel.ignoresSafeArea())
    }

    @ViewBuilder private var detail: some View {
        if let package = browser.selectedPackage {
            PackageDetail(package: package, texts: browser.texts)
        } else {
            Text(AuxText.licensesNoMatch).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
                .padding(Space._3).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
    }

    private func move(_ step: Int) -> KeyPress.Result {
        guard let selected = browser.selected, let at = browser.matches.firstIndex(of: selected) else { return .ignored }
        browser.selected = browser.matches[min(max(at + step, 0), browser.matches.count - 1)]
        return .handled
    }
}

private struct PackageRow: View {
    let package: LicenseList.Package
    let selected: Bool
    @Environment(\.tokens) private var tokens

    var body: some View {
        let muted = selected ? tokens.listSelectedMuted : tokens.textMuted
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(package.name).font(TextRole.body.font).lineLimit(1)
                Spacer(minLength: Space._2)
                Text(package.version).font(TextRole.caption.font).foregroundStyle(muted)
            }
            Text(package.license ?? AuxText.licenseNone).font(TextRole.caption.font).foregroundStyle(muted).lineLimit(1)
        }
        .foregroundStyle(selected ? tokens.listSelectedText : tokens.textPrimary)
        .padding(.horizontal, Space._2).padding(.vertical, Space._1)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(selected ? tokens.listSelected : .clear, in: RoundedRectangle(cornerRadius: Radius.row))
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(selected ? [.isSelected, .isButton] : .isButton)
    }
}
