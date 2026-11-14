import Foundation
import Testing

/// The colours a program asks for and sets beyond the background `colourQueries` covers (the foreground, the cursor, a palette
/// slot: asked, set, asked again, reset), a program's background as the terminal draws it, and the two engine switches of
/// Settings → Terminal that let a program ask for the title and for a checksum of the screen. With `cat`: a reply the
/// terminal writes to the program is echoed on the screen.
extension EndToEnd {
    private static let escape = "\u{1b}", bell = "\u{07}"

    /// A query sent on a clean screen, and the last `rgb:rrrr/gggg/bbbb` the terminal answered with.
    @MainActor private static func colour(_ app: Instance, asking query: String) async -> String? {
        await app.clearScreen()
        await app.discardLine()
        await app.emit("\(escape)]\(query);?\(bell)")
        let screen = await app.screen { $0.contains("rgb:") }
        await app.discardLine()
        guard let range = screen.range(of: "rgb:", options: .backwards) else { return nil }
        return String(screen[range.lowerBound...].prefix(18)).lowercased()
    }

    @Test("a program asks the foreground, the cursor and a palette slot, sets each and gets its own colour back, resets each to the theme's; its background is drawn")
    @MainActor func programColoursAskedSetAndDrawn() async throws {
        try await Instance.with { app in
            await app.send(["appearance dark"])
            for (query, set, reset, name) in [("10", "10;#a1b2c3", "110", "foreground"), ("12", "12;#c3b2a1", "112", "cursor"), ("4;1", "4;1;#123456", "104;1", "palette slot 1")] {
                guard let theme = await Self.colour(app, asking: query) else { app.fail("the \(name) is answered"); continue }
                await app.emit("\(Self.escape)]\(set)\(Self.bell)")
                let own = await Self.colour(app, asking: query)
                let hex = String(set.suffix(6))
                let wanted = "rgb:" + stride(from: 0, to: 6, by: 2).map { index -> String in
                    let pair = String(hex.dropFirst(index).prefix(2))
                    return pair + pair
                }.joined(separator: "/")
                app.expect(own == wanted, "the program's \(name) is answered: \(own ?? "-"), wanted \(wanted)")
                await app.emit("\(Self.escape)]\(reset)\(Self.bell)")
                let back = await Self.colour(app, asking: query)
                app.expect(back == theme, "the reset gives the theme's \(name) back: \(back ?? "-") (theme \(theme))")
            }

            // A background a program sets is the one the terminal is drawn with, until the reset.
            let theme = Readout(await app.reply("window-look")).first("window-look")?["terminal"] ?? ""
            await app.emit("\(Self.escape)]11;#203040\(Self.bell)")
            await app.until("window-look") { Readout($0).first("window-look")?["terminal"] == "#203040" }
            await app.emit("\(Self.escape)]111\(Self.bell)")
            await app.until("window-look") { Readout($0).first("window-look")?["terminal"] == theme }
            app.expect(theme != "#203040" && theme.hasPrefix("#"), "the theme's own background was read: \(theme)")
        }
    }

    @Test("a program is told the window's title and a checksum of the screen only with their switches in Settings clicked on, in a terminal started after")
    @MainActor func titleReportAndChecksumFollowTheirSwitches() async throws {
        try await Instance.with { app in
            let title = "\(Self.escape)[21t", checksum = "\(Self.escape)[1;1;1;1;1;1*y"
            // Off, as they come: neither is answered (the echo of what was typed is all there is).
            await app.emit(title)
            await app.emit(checksum)
            await app.send(["wait-idle 2000", "delay 500"])
            let silent = await app.reply("screen")
            app.expect(!silent.contains("]l") && !silent.contains("P1!~"), "nothing is answered while the switches are off:\n\(silent)")

            // On, by the switches of Settings → Terminal themselves, clicked: a terminal takes them when it starts.
            for (row, key) in [("title-report", "allow_title_report"), ("screen-checksum", "allow_screen_checksum")] {
                await app.clickSwitch(row, key: key, becomes: true)
                _ = await app.waitForConfig { $0.range(of: "\"\(key)\"\\s*:\\s*true", options: .regularExpression) != nil }
            }
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            var answered = ""
            let start = Date()
            repeat {
                // Asked again until answered: the new terminal reads the settings as it starts, a moment after the file changed.
                await app.emit(title)
                await app.emit(checksum)
                answered = (await app.send(["wait-idle 1500", "screen"]))[1]
                await app.discardLine()
            } while !(answered.contains("]l") && answered.contains("P1!~")) && Date().timeIntervalSince(start) < 15
            app.expect(answered.contains("]l"), "the title is reported:\n\(answered)")
            app.expect(answered.contains("P1!~"), "the checksum is reported:\n\(answered)")
        }
    }
}
