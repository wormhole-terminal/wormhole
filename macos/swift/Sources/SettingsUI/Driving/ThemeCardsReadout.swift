// text-file: the control folder's `theme-cards` (dev tooling), not interface text
// Appearance's theme picker as text (`theme-cards [filter]`): what `ThemeCards` draws, read from `ThemeShelves`, the same
// listing it draws from, with the filter box holding `filter`. The shape is `DesignSystem.Readout`'s:
//
//     theme-cards mode="system" filter="" placeholder="Filter 60 themes"
//     project "api" "This terminal shows api's theme. …"
//     shelf dark "In Dark Mode" total=31 collapsed=4
//       card nord "Nord" value="Nord" picked newer="2" file="/…/themes/mine.json"
//     none "No themes match the filter."
//     hidden "Soltari" kind="Dark"

import DesignSystem
import SettingsModel
import WordsModel

@MainActor
public enum ThemeCardsReadout {
    public static func text(_ model: SettingsModel, filter: String) -> String {
        let listing = ThemeShelves(model: model, filter: filter)
        var lines = [
            Readout.line(
                "theme-cards",
                pairs: [("mode", model.value("theme_mode").string ?? "system"), ("filter", filter), ("placeholder", listing.filterPlaceholder)])
        ]
        if let project = model.projectTheme {
            lines.append(
                Readout.line(
                    "project",
                    quoted: [
                        project.name,
                        Words.text("settings-appearance-project-theme-note", ["project": .text(project.name)], fallback: "This terminal shows \(project.name)'s theme. Your own is kept for everything else."),
                    ]))
        }
        for appearance in listing.shelves {
            let picked = model.value(ThemeShelves.key(appearance)).string
            let cards = listing.cards(appearance: appearance)
            let heading = appearance == .dark
                ? Words.text("settings-appearance-in-dark-mode", fallback: "In Dark Mode") : Words.text("settings-appearance-in-light-mode", fallback: "In Light Mode")
            lines.append(
                Readout.line(
                    "shelf", [appearance == .dark ? "dark" : "light"], quoted: [heading],
                    pairs: [("total", String(cards.count)), ("collapsed", String(ThemeShelves.collapsed))]))
            for card in cards {
                var pairs = [("value", card.value)]
                if let newer = card.newer { pairs.append(("newer", String(newer))) }
                if let path = card.path { pairs.append(("file", path)) }
                lines.append(Readout.under("card", quoted: [listing.label(card.theme)], pairs: pairs, flags: [Readout.flag("picked", card.isPicked(picked))]))
            }
        }
        // `ThemeCards`: the line under the shelves while a filter leaves every shelf empty.
        if !filter.isEmpty, listing.shelves.allSatisfy({ listing.cards(appearance: $0).isEmpty }) {
            lines.append(Readout.line("none", quoted: [Words.text("settings-appearance-no-themes-match-the-filter", fallback: "No themes match the filter.")]))
        }
        for value in listing.hidden {
            let known = listing.kind(ofHidden: value)
            lines.append(Readout.line("hidden", quoted: [known.name], pairs: [("kind", known.kind ?? "-"), ("value", value)]))
        }
        return lines.joined(separator: "\n")
    }
}
