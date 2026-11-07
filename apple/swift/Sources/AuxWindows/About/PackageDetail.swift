// One package's license: its name and version, the declared license, the repository as a link, then the license
// files in the code font (a license hard-wrapped at about 80 columns fits the pane).

import AuxModel
import DesignSystem
import SwiftUI

struct PackageDetail: View {
    let package: LicenseList.Package
    let texts: [String]
    @Environment(\.tokens) private var tokens

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header.padding(.horizontal, Space._6).padding(.top, Space._3).padding(.bottom, Space._3)
            Rule()
            ScrollView {
                VStack(alignment: .leading, spacing: Space._4) {
                    if texts.isEmpty {
                        Text(AuxText.licenseNoFile).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
                    }
                    ForEach(Array(texts.enumerated()), id: \.offset) { _, text in
                        Text(text).font(.system(size: TextRole.caption.size, design: .monospaced)).textSelection(.enabled)
                    }
                }
                .padding(Space._6)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            // A different package starts at the top of its text, not where the last one was scrolled to.
            .id(package.id)
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: Space._1) {
            HStack(alignment: .firstTextBaseline, spacing: Space._2) {
                Text(package.name).font(TextRole.title.font)
                Text(package.version).font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            }
            .accessibilityElement(children: .combine).accessibilityAddTraits(.isHeader)
            Text(package.license.map(AuxText.licenseIs) ?? AuxText.licenseNoneDeclared)
                .font(TextRole.body.font).foregroundStyle(tokens.textMuted)
            if let repository = package.repository, let url = URL(string: repository) {
                Link(repository, destination: url).font(TextRole.body.font).foregroundStyle(tokens.textLink)
            }
        }
    }
}
