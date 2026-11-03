import SwiftUI

/// Notifications: the banners that are turned on, newest first, at most two; bursts collapse into one.
struct BannerPreview: View {
    @Environment(\.tok) var t
    @Environment(SettingsModel.self) var m
    var body: some View {
        var banners: [(String, String)] = []
        if m.on("Notifications", 0, 0) { banners.append(("api needs input", "Allow edit of src/limits.rs?")) }
        if m.on("Notifications", 0, 1) { banners.append(("Release notes finished", "Claude Code · 12m")) }
        if m.str("Notifications", 0, 2) != "settings_notifications_off" { banners.append(("cargo build finished", "2m 14s · ~/Projects/api")) }
        if m.on("Notifications", 0, 3) { banners.append(("cargo test failed", "Exit code 101 · ~/Projects/api")) }
        if m.on("Notifications", 0, 4) { banners.append(("scratch rang the bell", "At most once a minute per terminal")) }
        if m.on("Notifications", 1, 3) && banners.count >= 3 { banners = [("\(banners.count) terminals need you", banners.map(\.0).joined(separator: " · "))] }
        banners = Array(banners.prefix(2))
        return VStack(alignment: .trailing, spacing: Space.s2) {
            ForEach(Array(banners.enumerated()), id: \.offset) { _, b in
                HStack(spacing: 10) {
                    RoundedRectangle(cornerRadius: Radius.tile).fill(t.primary).frame(width: 28, height: 28).overlay { GlyphIcon(d: GlyphPath.prompt, size: 14, colour: t.onPrimary) }
                    VStack(alignment: .leading, spacing: 1) {
                        Text(b.0).role(.body, weight: .semibold).lineLimit(1)
                        Text(b.1).role(.body).foregroundStyle(t.muted)
                    }
                    Spacer(minLength: 0)
                    Text(m.w("now")).role(.micro).foregroundStyle(t.muted).frame(maxHeight: .infinity, alignment: .top)
                }.padding(.horizontal, Space.s3).padding(.vertical, 10).frame(width: 340)
                    .background(RoundedRectangle(cornerRadius: Radius.banner).fill(t.elevated))
                    .shadow(color: .black.opacity(0.35), radius: 9, y: 6)
            }
            if banners.isEmpty { Text(m.w("Nothing turned on: no banners.")).role(.body).foregroundStyle(t.muted).frame(maxWidth: .infinity).padding(.vertical, Space.s3) }
        }.padding(Space.s3).frame(maxWidth: .infinity, minHeight: 64, alignment: .trailing)
    }
}
