#!/usr/bin/env python3
"""The Studio's gallery must show every shared component (the GPUI app's design lint of the same name): each public
`View` of DesignSystem is named in a file of StudioUI/Gallery, and GalleryUses.swift is up to date with the sources.

    python3 -I apple/scripts/gallery.py
"""
import re
import subprocess
import sys
from pathlib import Path

SOURCES = Path(__file__).resolve().parents[1] / "swift/Sources"
VIEW = re.compile(r"^public struct (\w+)(?:<[^>]*>)?\s*:\s*View", re.M)


def main() -> int:
    shown = "\n".join(p.read_text() for p in (SOURCES / "StudioUI/Gallery").glob("*.swift") if p.name != "GalleryUses.swift")
    components = sorted({n for p in (SOURCES / "DesignSystem").rglob("*.swift") for n in VIEW.findall(p.read_text())})
    missing = [n for n in components if not re.search(rf"\b{n}\b", shown)]
    stale = subprocess.run([sys.executable, "-I", str(Path(__file__).with_name("gen-gallery-uses.py")), "--check"]).returncode
    if missing:
        print(f"gallery: DesignSystem components the Studio gallery does not show: {', '.join(missing)}")
    if missing or stale:
        return 1
    print(f"gallery: ok ({len(components)} shared components shown)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
