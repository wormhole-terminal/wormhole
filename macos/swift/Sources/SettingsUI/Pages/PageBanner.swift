// The banners over a Settings page's groups (`render_section`), in their order and on their conditions: `SettingsPage`
// draws them and the control folder's `settings-page` reads them, from this one list.

import DesignSystem
import SettingsModel

enum PageBanner {
    /// A feature of the page is in preview: Feature flags.
    case preview(String)
    /// Zen mode overrides a row of the page: Leave Zen mode (`ZenNote`).
    case zen
    /// Advanced, while the settings file does not read: why, and Open.
    case broken(String)
    /// Any other page while the file does not read: changes are not saved.
    case unreadable
    case profiles(String)
    case records(String)

    @MainActor
    static func of(_ section: SettingsSchema.Section, _ model: SettingsModel) -> [PageBanner] {
        var banners: [PageBanner] = []
        if let note = model.host.previewNote(section: section.id) { banners.append(.preview(note)) }
        if model.zenOn, ZenNote.affects(section) { banners.append(.zen) }
        // The file that does not read says why on Advanced, with a way to open it; on every other page that changes are not saved.
        if section.id == "advanced", let broken = model.backend.banner() {
            banners.append(.broken(broken))
        } else if model.backend.unreadable {
            banners.append(.unreadable)
        }
        if section.id == "profile", let problem = model.host.profilesProblem() { banners.append(.profiles(problem)) }
        if let problem = model.host.panelRecords().problem { banners.append(.records(problem)) }
        return banners
    }

    var kind: BannerKind {
        switch self {
        case .preview, .zen: .info
        case .broken, .unreadable, .profiles: .warning
        case .records: .danger
        }
    }

    var text: String {
        switch self {
        case .preview(let text), .broken(let text), .profiles(let text), .records(let text): text
        case .zen: ZenNote.text
        case .unreadable: SettingsText.configUnreadable
        }
    }
}
