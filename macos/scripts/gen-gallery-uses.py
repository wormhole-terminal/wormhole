#!/usr/bin/env python3
"""Writes `StudioUI/Gallery/GalleryUses.swift`: which theme tokens each component of the Theme Studio's gallery draws
with, read from the Swift sources themselves, so the outline in the gallery cannot drift from what the views really
use. "Used by" and a metric's "n uses" tell core's `TOKEN_USES` (`studio_token_uses`), as the GPUI Studio does.

    python3 -I macos/scripts/gen-gallery-uses.py [--check]

A component is one of core's `TOKEN_USES` (ELEMENTS): its label and area are core's, its tokens those of the Swift source files that
draw it, or core's own list for a control that is the system's. A token is found as an accessor of the generated `ThemeTokens`
(`tokens.sidebarRowSelected`), a `Palette` reader (which names the accessor it forwards to), in the window's own files a
`ChromeColours` reader (`colours.bandHover`, the same way), a `named: "x"` string, or a corner and rule role (`Radius.card`, `Rule.border`). `--check` changes nothing and fails when the file is stale.
"""
import re
import sys
from fnmatch import fnmatch
from pathlib import Path

SWIFT = Path(__file__).resolve().parents[1] / "swift/Sources"
OUT = SWIFT / "StudioUI/Gallery/GalleryUses.swift"

# The gallery's components, by the id core's `TOKEN_USES` gives them (which `.marked(id)` names in the views): the Swift
# sources that draw one (its tokens are read from them), or none for a control that is the system's, tinted by the
# theme, whose tokens are core's. A component core has no entry for names its own (label, area) in SWIFT_ONLY.
ELEMENTS = {
    "sidebar.row": ["SidebarUI/Row/[!K]*"],
    "sidebar.card": ["SidebarUI/Group/CardSurface.swift"],
    "sidebar.band": ["SidebarUI/Group/GroupHeadView.swift", "SidebarUI/Group/HeadControls.swift", "SidebarUI/Group/FoldedSummary.swift"],
    "status.tile": ["SidebarUI/Tile/*"],
    "status.glyph": ["DesignSystem/StatusDot.swift"],
    "status.bar": ["WindowUI/StatusBar.swift"],
    "folder.color": ["PanelUI/Project/ColorChoice.swift", "PanelUI/Project/FolderColorCard.swift"],
    "branch.chip": ["PanesUI/Title/BranchCrumb.swift"],
    "panel.card": ["PanelUI/Shell/PanelCard.swift"],
    "panel.band": ["PanelUI/Shell/CardBand.swift", "PanelUI/Shell/BandButton.swift", "PanelUI/Shell/Disclosure.swift"],
    "panel.item": ["PanelUI/Notes/TaskRowView.swift", "PanelUI/Notes/TaskCheckbox.swift"],
    "file.row": ["PanelUI/Changes/FileRowView.swift", "PanelUI/Changes/LineCounts.swift"],
    "commit.row": ["PanelUI/History/CommitRowView.swift", "PanelUI/History/CommitRefs.swift"],
    "diff": ["PanelUI/Diff/Diff*"],
    "callout": ["PanelUI/Shell/Callout.swift"],
    "icon.button": ["DesignSystem/IconButton.swift"],
    "icon.tab": ["WindowUI/PanelTabBar.swift"],
    "title.tab": ["PanesUI/Tabs/TerminalTabView.swift"],
    "keycap": ["DesignSystem/KeyCap.swift", "SidebarUI/Row/Keycap.swift"],
    "badge": ["DesignSystem/CountBadge.swift", "DesignSystem/StatusBadge.swift", "AuxWindows/Shared/StatusBadge.swift"],
    "neutral.chip": ["PanesUI/Title/ExtraChip.swift"],
    "title.extras": ["PanesUI/Title/BranchStatusMark.swift"],
    "focus.ring": [],
    "segmented": ["DesignSystem/Segmented.swift"],
    "card": ["AuxWindows/Shared/AuxCard.swift"],
    "column.divider": ["PanesUI/Panes/PaneDividerView.swift"],
    "find": ["TerminalUI/Find/*"],
    "tint.chip": ["FontPickerUI/TintChip.swift"],
    "settings.nav": ["SettingsUI/Nav/*"],
    "setting.row": ["SettingsUI/Rows/SettingRow.swift"],
    "switch": [],
    "select": [],
    "stepper": ["SettingsUI/Controls/ValueStepper.swift"],
    "recorder": ["SettingsUI/Controls/ShortcutRecorder.swift"],
    "button": [],
    "button.primary": [],
    "input": [],
    "checkbox": [],
    "menu": ["PanelUI/Shell/RevPicker.swift"],
    "banner": ["DesignSystem/Banner.swift"],
    "palette.row": ["PaletteUI/ResultRow.swift"],
    "readout": ["WindowUI/Columns/ResizeReadout.swift", "TerminalUI/Scaling/SizeReadoutChip.swift", "TerminalUI/Support/ReadoutPill.swift"],
    "readout.block": ["TerminalUI/Blocks/BlockReadout.swift", "TerminalUI/Support/ReadoutPill.swift"],
    "resize.handle": ["WindowUI/Columns/ColumnHandle.swift"],
    "scrollbar": ["PanelUI/Diff/OverflowCue.swift"],
    "terminal": [],
    "code": [],
}
SWIFT_ONLY = {
    "fonts": ("font picker", "settings", ["FontPickerUI/*"], ["terminal.*"]),
    "sketch": ("window sketch", "ambient", ["AuxWindows/Onboarding/WindowSketch.swift", "AuxWindows/Onboarding/TailorStep.swift"], []),
}
CORE_USES = SWIFT.parents[2] / "crates/wormhole-core/src/theme/tokens/uses.rs"
AREAS = {"Sidebar": "sidebar", "Panel": "panel", "Chrome": "chrome", "Settings": "settings", "Ambient": "ambient", "Basics": "all"}
ROLES = {"Radius.sm": "metrics.radius_sm", "Radius.chip": "metrics.radius_chip", "Radius.control": "metrics.radius",
         "Radius.card": "metrics.radius_lg", "Rule.border": "metrics.border_width",
         "Rule.divider": "metrics.divider_width", "Rule.focusRing": "metrics.focus_ring_width"}


def accessors() -> dict[str, str]:
    table: dict[str, str] = {}
    for name in ("Tokens1.swift", "Tokens2.swift"):
        text = (SWIFT / "DesignSystem/Theme" / name).read_text()
        table.update({a: t for a, t in re.findall(r'public var (\w+): Color \{ color\(named: "([^"]+)"\) \}', text)})
    palette = (SWIFT / "DesignSystem/Scale/Palette.swift").read_text()
    for name, accessor in re.findall(r"public static var (\w+): Color \{ tokens\.(\w+) \}", palette):
        if accessor in table:
            table[name] = table[accessor]
    return table


def chrome(table: dict[str, str]) -> dict[str, str]:
    """The window's `ChromeColours` readers: name -> token, each a one-line forward to a token or a metric."""
    text = (SWIFT / "WindowUI/ChromeColours.swift").read_text()
    out = {name: table[accessor] for name, accessor in re.findall(r"var (\w+): Color \{ tokens\.(\w+) \}", text) if accessor in table}
    for name, metric in re.findall(r"var (\w+): CGFloat \{ tokens\.metrics\.(\w+) \}", text):
        out[name] = "metrics." + re.sub(r"(?<!^)([A-Z])", r"_\1", metric).lower()
    return out


def tokens_of(text: str, table: dict[str, str], colours: dict[str, str]) -> set[str]:
    found = {table[m] for m in re.findall(r"\b(?:tokens|Palette|ui)\.(\w+)", text) if m in table}
    found |= {colours[m] for m in re.findall(r"\bcolours\.(\w+)", text) if m in colours}
    found |= set(re.findall(r'(?:named|rgba\(named): "([a-z_.0-9]+)"', text))
    found |= {token for role, token in ROLES.items() if role in text}
    # The theme's metrics read through `tokens.metrics.radiusLg`: the token is `metrics.radius_lg`.
    metrics = {"radiusSm", "radiusChip", "radius", "radiusLg", "borderWidth", "dividerWidth", "focusRingWidth"}
    found |= {"metrics." + re.sub(r"(?<!^)([A-Z])", r"_\1", m).lower() for m in re.findall(r"\bmetrics\.(\w+)", text) if m in metrics}
    # The shared `Chip` (DesignSystem/Chip.swift) draws in chip corners, and an outline passed to it at the border width.
    if re.search(r"\bChip\(", text):
        found.add("metrics.radius_chip")
    if re.search(r"\bChip\([^{]*?\boutline:", text, re.S):
        found.add("metrics.border_width")
    return found


def core_uses() -> dict[str, tuple[str, str, list[str]]]:
    """Core's `TOKEN_USES`: id -> (label, area, tokens)."""
    text = CORE_USES.read_text()
    rows = re.findall(r'u\(\s*"([^"]+)",\s*"([^"]+)",\s*(\w+),\s*&\[(.*?)\],?\s*\)', text, re.S)
    return {i: (label, AREAS[area], re.findall(r'"([^"]+)"', tokens)) for i, label, area, tokens in rows}


def swift_tokens(patterns: list[str], table: dict[str, str]) -> set[str]:
    files = sorted(p for p in SWIFT.glob("*/**/*.swift") if any(fnmatch(str(p.relative_to(SWIFT)), g) for g in patterns))
    colours = chrome(table)
    return set().union(*(tokens_of(f.read_text(), table, colours if f.relative_to(SWIFT).parts[0] == "WindowUI" else {}) for f in files))


def main() -> int:
    table = accessors()
    core = core_uses()
    rows: list[tuple[str, str, str, list[str]]] = []
    for ident, patterns in ELEMENTS.items():
        label, area, own = core[ident]
        found = swift_tokens(patterns, table) | {t for t in own if t.endswith(".*")} if patterns else set(own)
        rows.append((ident, label, area, sorted(found)))
    for ident, (label, area, patterns, extra) in SWIFT_ONLY.items():
        rows.append((ident, label, area, sorted(swift_tokens(patterns, table) | set(extra))))
    lines = ["// Generated by macos/scripts/gen-gallery-uses.py from the Swift sources: do not edit.",
             "// The components of the gallery and the tokens each draws with (`TOKEN_USES`' ids, which the views mark with",
             "// `.marked(id)`): what the Studio outlines while a token is selected. \"Used by\" is core's `TOKEN_USES`.",
             "// text-file: generated; the labels are core's `TOKEN_USES` names, which the GPUI Studio shows untranslated too",
             "", "import StudioModel", "", "enum GalleryUses {", "    static let elements: [TokenUse] = ["]
    for ident, label, area, tokens in rows:
        lines.append(f'        TokenUse(id: "{ident}", label: "{label}", area: .{area}, tokens: [')
        line = "            "
        for token in tokens:
            piece = f'"{token}", '
            if len(line) + len(piece) > 118:
                lines.append(line.rstrip())
                line = "            "
            line += piece
        if line.strip():
            lines.append(line.rstrip())
        lines.append("        ]),")
    lines += ["    ]", "}", ""]
    text = "\n".join(lines)
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print("GalleryUses.swift is stale: run macos/scripts/gen-gallery-uses.py", file=sys.stderr)
            return 1
        return 0
    OUT.write_text(text)
    print({ident: len(tokens) for ident, _, _, tokens in rows})
    return 0


if __name__ == "__main__":
    sys.exit(main())
