#!/usr/bin/env python3
"""The Studio's gallery must show every shared component (the GPUI app's design lint of the same name): each public
`View` of DesignSystem is named in a file of StudioUI/Gallery, GalleryUses.swift is up to date with the sources, and every
component it lists is outlined by a `.marked("id")` somewhere in the sources (and every mark names a listed component),
as the GPUI gallery's own test holds `Env::mark` to `TOKEN_USES`.

    python3 -I macos/scripts/gallery.py
"""
import re
import subprocess
import sys
from pathlib import Path

SOURCES = Path(__file__).resolve().parents[1] / "swift/Sources"
VIEW = re.compile(r"^public struct (\w+)(?:<[^>]*>)?\s*:\s*View", re.M)
# The id is the first argument; inspect mode's `style:`, `file:` and `line:` may follow it.
MARK = re.compile(r'\.marked\("([^"]+)"[,)]')
LISTED = re.compile(r'TokenUse\(id: "([^"]+)"')


def main() -> int:
    shown = "\n".join(p.read_text() for p in (SOURCES / "StudioUI/Gallery").glob("*.swift") if p.name != "GalleryUses.swift")
    components = sorted({n for p in (SOURCES / "DesignSystem").rglob("*.swift") for n in VIEW.findall(p.read_text())})
    missing = [n for n in components if not re.search(rf"\b{n}\b", shown)]
    stale = subprocess.run([sys.executable, "-I", str(Path(__file__).with_name("gen-gallery-uses.py")), "--check"]).returncode
    listed = set(LISTED.findall((SOURCES / "StudioUI/Gallery/GalleryUses.swift").read_text()))
    marked = {m for p in SOURCES.rglob("*.swift") if p.name != "GalleryUses.swift" for m in MARK.findall(p.read_text())}
    if missing:
        print(f"gallery: DesignSystem components the Studio gallery does not show: {', '.join(missing)}")
    if listed - marked:
        print(f"gallery: components listed in GalleryUses that no view marks: {', '.join(sorted(listed - marked))}")
    if marked - listed:
        print(f"gallery: marks that name no listed component: {', '.join(sorted(marked - listed))}")
    if missing or stale or listed != marked:
        return 1
    print(f"gallery: ok ({len(components)} shared components shown)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
