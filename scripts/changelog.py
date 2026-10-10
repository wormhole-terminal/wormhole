#!/usr/bin/env python3
"""CHANGELOG.md: the one parser behind the release notes, the update feed and the checks.

  scripts/changelog.py check                       validate the file (header, order, subsections, security bullets)
  scripts/changelog.py notes <version>             a version's entries as Markdown, without its heading
  scripts/changelog.py severity <version>          the highest severity among its Security bullets (lowercase)
  scripts/changelog.py severity <version> --advisories   its GHSA ids, one per line
  scripts/changelog.py release <version> [--date YYYY-MM-DD]
                                                   turn [Unreleased] into the version, start a new [Unreleased]
  scripts/changelog.py selftest                    run the built-in tests

--file picks another file (default: CHANGELOG.md at the repository root). The format is Keep a
Changelog 1.1 with one extra rule for `### Security`, see CONTRIBUTING.md ("Changelog"):

  - **High** · [GHSA-xxxx-xxxx-xxxx](https://github.com/wormhole-terminal/wormhole/security/advisories/GHSA-xxxx-xxxx-xxxx) · affects 0.2.0–0.3.0: what was wrong.

Python 3.9, standard library only.
"""
from __future__ import annotations

import argparse
import contextlib
import datetime
import io
import os
import re
import sys
import tempfile
from typing import Dict, List, Optional, Tuple

REPO_URL = "https://github.com/wormhole-terminal/wormhole"
SUBSECTIONS = ["Added", "Changed", "Deprecated", "Removed", "Fixed", "Security"]
SEVERITIES = ["Low", "Medium", "High", "Critical"]  # ascending
# Versions before this one were tagged in the private repository: they have no link references.
FIRST_LINKED = (0, 3, 1)
DEFAULT_FILE = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "CHANGELOG.md")

_PRE = r"[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*"
VERSION = r"\d+\.\d+\.\d+(?:-" + _PRE + r")?"
VERSION_RE = re.compile(r"(\d+)\.(\d+)\.(\d+)(?:-(" + _PRE + r"))?")
HEADING_RE = re.compile(r"## \[(" + VERSION + r")\] - (\d{4}-\d{2}-\d{2})")
UNRELEASED = "## [Unreleased]"
REF_RE = re.compile(r"\[([^\]]+)\]: (\S+)")
# In a security bullet a prerelease starts with a letter (0.4.0-beta.1), so `0.2.0-0.3.0` is not read as one version.
AFFECTED = r"\d+\.\d+\.\d+(?:-[A-Za-z][0-9A-Za-z.-]*)?"
GHSA = r"GHSA-[0-9a-z]{4}-[0-9a-z]{4}-[0-9a-z]{4}"
SECURITY_RE = re.compile(
    r"- \*\*(" + "|".join(SEVERITIES) + r")\*\* · "
    r"(?:\[(" + GHSA + r")\]\(" + re.escape(REPO_URL) + r"/security/advisories/(" + GHSA + r")\)|no advisory)"
    r" · affects (" + AFFECTED + r"(?:–" + AFFECTED + r")?): \S.*"
)


class Section:
    """One `## ` block: Unreleased or a version."""

    def __init__(self, name: str, date: Optional[str], index: int) -> None:
        self.name = name
        self.date = date
        self.index = index  # the heading's line, 0-based
        self.end = index + 1  # one past its last line
        self.subs: List[Sub] = []
        self.stray: List[int] = []  # non-blank lines before the first ### (0-based)


class Sub:
    def __init__(self, name: str, index: int) -> None:
        self.name = name
        self.index = index
        self.end = index + 1

    def body(self, lines: List[str]) -> List[str]:
        return strip_blank(lines[self.index + 1 : self.end])


class Doc:
    def __init__(self, path: str, lines: List[str]) -> None:
        self.path = path
        self.lines = lines
        self.sections: List[Section] = []
        self.refs: Dict[str, Tuple[int, str]] = {}
        self.refs_start = len(lines)
        self.problems: List[str] = []
        self.first_section = len(lines)

    def err(self, index: int, message: str) -> None:
        self.problems.append("%s:%d: %s" % (self.path, index + 1, message))

    def find(self, name: str) -> Optional[Section]:
        for section in self.sections:
            if section.name.lower() == name.lower():
                return section
        return None


def strip_blank(lines: List[str]) -> List[str]:
    lines = list(lines)
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    return lines


def sem_key(version: str):
    """An ordering key for SemVer 2.0: a prerelease sorts before its release."""
    m = VERSION_RE.fullmatch(version)
    if not m:
        raise ValueError("not a version: %r" % version)
    major, minor, patch, pre = int(m.group(1)), int(m.group(2)), int(m.group(3)), m.group(4)
    if pre is None:
        return (major, minor, patch, 1, ())
    ids = tuple((0, int(i), "") if i.isdigit() else (1, 0, i) for i in pre.split("."))
    return (major, minor, patch, 0, ids)


def core(version: str) -> Tuple[int, int, int]:
    m = VERSION_RE.fullmatch(version)
    return (int(m.group(1)), int(m.group(2)), int(m.group(3))) if m else (0, 0, 0)


def parse(path: str, text: str) -> Doc:
    lines = text.split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    doc = Doc(path, lines)

    # The link references are the last block of the file.
    i = len(lines)
    while i > 0 and not lines[i - 1].strip():
        i -= 1
    while i > 0 and REF_RE.fullmatch(lines[i - 1]):
        i -= 1
        m = REF_RE.fullmatch(lines[i])
        if m.group(1) in doc.refs:
            doc.err(i, "duplicate link reference [%s]" % m.group(1))
        doc.refs[m.group(1)] = (i, m.group(2))
    doc.refs_start = i

    section: Optional[Section] = None
    sub: Optional[Sub] = None
    for n in range(doc.refs_start):
        line = lines[n]
        if line.startswith("## "):
            if line == UNRELEASED:
                section = Section("Unreleased", None, n)
            else:
                m = HEADING_RE.fullmatch(line)
                if not m:
                    doc.err(n, "heading must be `## [Unreleased]` or `## [X.Y.Z] - YYYY-MM-DD`, got %r" % line)
                    section = None
                    sub = None
                    continue
                try:
                    datetime.date.fromisoformat(m.group(2))
                except ValueError:
                    doc.err(n, "%s is not a date" % m.group(2))
                section = Section(m.group(1), m.group(2), n)
            doc.sections.append(section)
            sub = None
            if doc.first_section == len(lines):
                doc.first_section = n
        elif line.startswith("### "):
            name = line[4:].strip()
            if section is None:
                doc.err(n, "`###` outside a version section")
                continue
            if name not in SUBSECTIONS:
                doc.err(n, "unknown subsection %r (use %s)" % (name, ", ".join(SUBSECTIONS)))
            sub = Sub(name, n)
            section.subs.append(sub)
        elif re.match(r"#{1,6} ", line) and not (n == 0 and line.startswith("# ")):
            doc.err(n, "only `## ` versions and `### ` subsections are allowed here")
        elif line.strip() and section is not None and sub is None:
            section.stray.append(n)
        if section is not None:
            section.end = n + 1
            if sub is not None:
                sub.end = n + 1
    return doc


def validate(doc: Doc) -> List[str]:
    """Every problem in the file, as `file:line: message`; the parse's own come first."""
    problems = list(doc.problems)

    def err(index: int, message: str) -> None:
        problems.append("%s:%d: %s" % (doc.path, index + 1, message))

    # Header.
    header = doc.lines[: doc.first_section]
    if not header or header[0] != "# Changelog":
        err(0, "the file starts with `# Changelog`")
    text = " ".join(header)
    if "Keep a Changelog" not in text or "Semantic Versioning" not in text:
        err(0, "the header says the format is Keep a Changelog and versions follow Semantic Versioning")
    if len(strip_blank(header[1:])) == 0:
        err(0, "the header needs a paragraph under the title")

    if not doc.sections:
        err(0, "no `## [Unreleased]` section")
        return problems
    first = doc.sections[0]
    if first.name != "Unreleased":
        err(first.index, "`## [Unreleased]` comes first")

    # Versions: unique, newest first.
    previous: Optional[Section] = None
    for section in doc.sections:
        if section.name == "Unreleased":
            if section is not first:
                err(section.index, "`## [Unreleased]` appears twice")
            continue
        if previous is not None and sem_key(section.name) >= sem_key(previous.name):
            what = "twice" if section.name == previous.name else "after %s: newest first" % previous.name
            err(section.index, "version %s listed %s" % (section.name, what))
        previous = section

    # Subsections.
    for section in doc.sections:
        if section.stray:
            err(section.stray[0], "entries go under a `###` subsection (%s)" % ", ".join(SUBSECTIONS))
        if section.name != "Unreleased" and not section.subs and not section.stray:
            err(section.index, "version %s has no entries" % section.name)
        seen: List[str] = []
        for sub in section.subs:
            if sub.name not in SUBSECTIONS:
                continue
            if not sub.body(doc.lines):
                err(sub.index, "empty subsection `### %s`: remove it" % sub.name)
            if sub.name in seen:
                err(sub.index, "`### %s` appears twice in %s" % (sub.name, section.name))
            elif seen and SUBSECTIONS.index(sub.name) < max(SUBSECTIONS.index(s) for s in seen):
                err(sub.index, "`### %s` is out of order (%s)" % (sub.name, ", ".join(SUBSECTIONS)))
            seen.append(sub.name)
            if sub.name == "Security":
                for n in range(sub.index + 1, sub.end):
                    line = doc.lines[n]
                    if line.strip() and not line.startswith(" "):  # an indented line continues a bullet
                        check_security_line(line, n, err)

    # Link references: [Unreleased], and every version tagged in the public repository.
    if "Unreleased" not in doc.refs:
        err(doc.refs_start, "missing the link reference `[Unreleased]: %s/commits/main`" % REPO_URL)
    for section in doc.sections:
        if section.name != "Unreleased" and core(section.name) >= FIRST_LINKED and section.name not in doc.refs:
            err(doc.refs_start, "missing the link reference for [%s]" % section.name)
    return problems


def check_security_line(line: str, n: int, err) -> None:
    m = SECURITY_RE.fullmatch(line)
    if not m:
        err(
            n,
            "security bullet must read `- **High** · [GHSA-xxxx-xxxx-xxxx](%s/security/advisories/GHSA-xxxx-xxxx-xxxx)"
            " · affects 0.2.0–0.3.0: what was wrong` (severity Critical, High, Medium or Low;"
            " `no advisory` instead of the link when there is none)" % REPO_URL,
        )
        return
    if m.group(2) != m.group(3):
        err(n, "the advisory id in the link text (%s) and in its URL (%s) differ" % (m.group(2), m.group(3)))
    versions = m.group(4).split("–")
    if len(versions) == 2 and sem_key(versions[0]) > sem_key(versions[1]):
        err(n, "affected versions run backwards: %s" % m.group(4))


def security_bullets(doc: Doc, section: Section) -> List[Tuple[str, Optional[str]]]:
    """(severity, ghsa id or None) for each well-formed Security bullet of a section."""
    found = []
    for sub in section.subs:
        if sub.name != "Security":
            continue
        for n in range(sub.index + 1, sub.end):
            m = SECURITY_RE.fullmatch(doc.lines[n])
            if m:
                found.append((m.group(1), m.group(2)))
    return found


# --------------------------------------------------------------------------- commands


def load(path: str) -> Doc:
    try:
        with open(path, encoding="utf-8") as f:
            return parse(path, f.read())
    except OSError as e:
        sys.exit("error: cannot read %s: %s" % (path, e))


def clean_version(arg: str) -> str:
    return arg[1:] if arg.startswith("v") else arg


def cmd_check(args) -> int:
    doc = load(args.file)
    problems = validate(doc)
    for p in problems:
        print(p, file=sys.stderr)
    if problems:
        print("%d problem(s) in %s" % (len(problems), args.file), file=sys.stderr)
        return 1
    return 0


def section_or_die(doc: Doc, version: str) -> Section:
    section = doc.find(clean_version(version))
    if section is None:
        sys.exit("error: %s has no section for %s" % (doc.path, clean_version(version)))
    return section


def cmd_notes(args) -> int:
    doc = load(args.file)
    section = section_or_die(doc, args.version)
    body = strip_blank(doc.lines[section.index + 1 : section.end])
    if not body:
        sys.exit("error: the section for %s is empty" % section.name)
    print("\n".join(body))
    return 0


def cmd_severity(args) -> int:
    doc = load(args.file)
    section = section_or_die(doc, args.version)
    found = security_bullets(doc, section)
    if args.advisories:
        for _, ghsa in found:
            if ghsa:
                print(ghsa)
    elif found:
        print(max((s for s, _ in found), key=SEVERITIES.index).lower())
    return 0


def release_text(doc: Doc, version: str, date: str) -> str:
    if not VERSION_RE.fullmatch(version):
        sys.exit("error: %r is not a version (X.Y.Z or X.Y.Z-beta.N)" % version)
    try:
        datetime.date.fromisoformat(date)
    except ValueError:
        sys.exit("error: %r is not a date (YYYY-MM-DD)" % date)
    problems = validate(doc)
    if problems:
        sys.exit("error: fix the changelog first:\n" + "\n".join(problems))
    unreleased = doc.sections[0]
    if not any(sub.body(doc.lines) for sub in unreleased.subs):
        sys.exit("error: [Unreleased] is empty: nothing to release")
    released = doc.sections[1:]
    if released and sem_key(version) <= sem_key(released[0].name):
        sys.exit("error: %s is not newer than %s, the latest in %s" % (version, released[0].name, doc.path))
    previous = released[0].name if released else None

    lines = doc.lines[: unreleased.index + 1] + ["", "## [%s] - %s" % (version, date)]
    lines += doc.lines[unreleased.index + 1 : doc.refs_start]
    while lines and not lines[-1].strip():
        lines.pop()

    refs = [("Unreleased", "%s/compare/v%s...HEAD" % (REPO_URL, version))]
    if previous is not None and core(previous) >= FIRST_LINKED:
        refs.append((version, "%s/compare/v%s...v%s" % (REPO_URL, previous, version)))
    else:
        refs.append((version, "%s/releases/tag/v%s" % (REPO_URL, version)))
    refs += [(name, url) for name, (_, url) in sorted(doc.refs.items(), key=lambda kv: kv[1][0]) if name != "Unreleased"]
    lines += [""] + ["[%s]: %s" % r for r in refs]
    return "\n".join(lines) + "\n"


def cmd_release(args) -> int:
    version = clean_version(args.version)
    doc = load(args.file)
    date = args.date or datetime.date.today().isoformat()
    text = release_text(doc, version, date)
    tmp = args.file + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        f.write(text)
    os.replace(tmp, args.file)
    print("%s: [Unreleased] is now %s (%s)" % (args.file, version, date), file=sys.stderr)
    return 0


# --------------------------------------------------------------------------- selftest

SAMPLE = """# Changelog

All notable changes to Wormhole are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) as described in CONTRIBUTING.md.

## [Unreleased]

### Fixed
- A fix
  that wraps.

### Security
- **Medium** · no advisory · affects 0.3.1: something small.

## [0.3.1] - 2026-10-12

### Security
- **Low** · [GHSA-aaaa-bbbb-cccc](https://github.com/wormhole-terminal/wormhole/security/advisories/GHSA-aaaa-bbbb-cccc) · affects 0.2.0–0.3.0: low one.
- **Critical** · [GHSA-dddd-eeee-ffff](https://github.com/wormhole-terminal/wormhole/security/advisories/GHSA-dddd-eeee-ffff) · affects 0.3.0: bad one.

## [0.3.0] - 2026-10-10

### Added
- Something.

[Unreleased]: https://github.com/wormhole-terminal/wormhole/commits/main
[0.3.1]: https://github.com/wormhole-terminal/wormhole/releases/tag/v0.3.1
"""


def selftest() -> int:
    failures: List[str] = []

    def expect(name: str, ok: bool) -> None:
        if not ok:
            failures.append(name)

    def problems_of(text: str) -> List[str]:
        return validate(parse("t.md", text))

    def has(text: str, needle: str) -> bool:
        return any(needle in p for p in problems_of(text))

    ok_text = SAMPLE.replace("[0.3.1]: https://github.com/wormhole-terminal/wormhole/releases/tag/v0.3.1",
                             "[0.3.1]: https://github.com/wormhole-terminal/wormhole/compare/v0.3.0...v0.3.1")
    expect("sample is valid", problems_of(ok_text) == [])
    expect("missing header", has(ok_text.replace("# Changelog", "# Log", 1), "starts with `# Changelog`"))
    expect("missing format words", has(ok_text.replace("Semantic Versioning", "SemVer"), "Semantic Versioning"))
    expect("unreleased first", has(ok_text.replace("## [Unreleased]", "## [0.3.2] - 2026-10-13", 1), "comes first"))
    expect("order", has(ok_text.replace("## [0.3.1] - 2026-10-12", "## [0.2.9] - 2026-10-12"), "newest first"))
    expect("duplicate version", has(ok_text.replace("## [0.3.1] - 2026-10-12", "## [0.3.0] - 2026-10-12"), "twice"))
    expect("prerelease order", not has(ok_text.replace("## [0.3.1] - 2026-10-12", "## [0.3.1-beta.1] - 2026-10-12")
                                       .replace("[0.3.1]:", "[0.3.1-beta.1]:"), "newest first"))
    expect("bad heading", has(ok_text.replace("## [0.3.0] - 2026-10-10", "## 0.3.0 (2026-10-10)"), "heading must be"))
    expect("bad date", has(ok_text.replace("2026-10-10", "2026-13-10"), "not a date"))
    expect("unknown subsection", has(ok_text.replace("### Added", "### Improved"), "unknown subsection"))
    expect("empty subsection", has(ok_text.replace("- Something.\n", ""), "empty subsection"))
    expect("subsection order", has(ok_text.replace("### Fixed\n- A fix\n  that wraps.\n\n### Security\n- **Medium** · no advisory · affects 0.3.1: something small.",
                                                    "### Security\n- **Medium** · no advisory · affects 0.3.1: something small.\n\n### Fixed\n- A fix"), "out of order"))
    expect("stray text", has(ok_text.replace("## [0.3.0] - 2026-10-10\n", "## [0.3.0] - 2026-10-10\nloose text\n"), "go under a `###`"))
    expect("missing ref", has(ok_text.replace("[0.3.1]: ", "[0.3.2]: "), "link reference for [0.3.1]"))
    expect("missing unreleased ref", has(ok_text.replace("[Unreleased]: ", "[Other]: "), "[Unreleased]"))
    for name, bullet in {
        "lowercase severity": "- **high** · no advisory · affects 0.3.0: x",
        "unknown severity": "- **Severe** · no advisory · affects 0.3.0: x",
        "no severity": "- no advisory · affects 0.3.0: x",
        "no affects": "- **High** · no advisory: x",
        "hyphen range": "- **High** · no advisory · affects 0.2.0-0.3.0: x",
        "bad ghsa": "- **High** · [GHSA-1](https://github.com/wormhole-terminal/wormhole/security/advisories/GHSA-1) · affects 0.3.0: x",
        "other repo": "- **High** · [GHSA-aaaa-bbbb-cccc](https://github.com/x/y/security/advisories/GHSA-aaaa-bbbb-cccc) · affects 0.3.0: x",
        "no description": "- **High** · no advisory · affects 0.3.0:",
        "plain text": "- Fixed a hole.",
    }.items():
        expect("security format: " + name, has(ok_text.replace("- **Medium** · no advisory · affects 0.3.1: something small.", bullet), "security bullet must read"))
    expect("ghsa mismatch", has(ok_text.replace("advisories/GHSA-aaaa-bbbb-cccc", "advisories/GHSA-aaaa-bbbb-zzzz"), "differ"))
    expect("range backwards", has(ok_text.replace("0.2.0–0.3.0", "0.3.0–0.2.0"), "backwards"))
    expect("accepts en dash range and single", not has(ok_text, "security bullet"))

    def run(argv: List[str]):
        out, err = io.StringIO(), io.StringIO()
        code = 0
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                code = main(argv)
            except SystemExit as e:
                code = 1 if isinstance(e.code, str) else (e.code or 0)
                if isinstance(e.code, str):
                    print(e.code, file=sys.stderr)
        return code, out.getvalue(), err.getvalue()

    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "CHANGELOG.md")
        with open(path, "w", encoding="utf-8") as f:
            f.write(ok_text)
        code, _, _ = run(["--file", path, "check"])
        expect("check passes", code == 0)
        with open(path, "w", encoding="utf-8") as f:
            f.write(ok_text.replace("**Low**", "**Lo**"))
        code, _, err = run(["--file", path, "check"])
        expect("check fails with file:line", code != 0 and (path + ":") in err)
        with open(path, "w", encoding="utf-8") as f:
            f.write(ok_text)

        code, out, _ = run(["--file", path, "notes", "0.3.0"])
        expect("notes", code == 0 and out == "### Added\n- Something.\n")
        code, out, _ = run(["--file", path, "notes", "v0.3.1"])
        expect("notes with v", code == 0 and out.startswith("### Security\n- **Low**") and out.endswith("bad one.\n"))
        code, _, _ = run(["--file", path, "notes", "9.9.9"])
        expect("notes of a missing version fails", code != 0)

        code, out, _ = run(["--file", path, "severity", "0.3.1"])
        expect("severity is the highest", code == 0 and out == "critical\n")
        code, out, _ = run(["--file", path, "severity", "0.3.1", "--advisories"])
        expect("advisories", out == "GHSA-aaaa-bbbb-cccc\nGHSA-dddd-eeee-ffff\n")
        code, out, _ = run(["--file", path, "severity", "0.3.0"])
        expect("severity of a version without any is empty", code == 0 and out == "")
        code, out, _ = run(["--file", path, "severity", "Unreleased"])
        expect("severity of Unreleased", out == "medium\n")

        code, _, _ = run(["--file", path, "release", "0.3.1"])
        expect("release refuses a version that is not newer", code != 0)
        code, _, _ = run(["--file", path, "release", "0.4.0", "--date", "2026-13-01"])
        expect("release refuses a bad date", code != 0)
        code, _, _ = run(["--file", path, "release", "0.4.0", "--date", "2026-10-20"])
        expect("release", code == 0)
        with open(path, encoding="utf-8") as f:
            released = f.read()
        expect("release keeps the file valid", problems_of(released) == [])
        expect("release heading", "## [Unreleased]\n\n## [0.4.0] - 2026-10-20\n\n### Fixed" in released)
        expect("release refs", "[Unreleased]: %s/compare/v0.4.0...HEAD\n[0.4.0]: %s/compare/v0.3.1...v0.4.0\n[0.3.1]: " % (REPO_URL, REPO_URL) in released)
        code, out, _ = run(["--file", path, "notes", "0.4.0"])
        expect("release notes", out.startswith("### Fixed\n- A fix\n  that wraps."))
        code, _, _ = run(["--file", path, "release", "0.4.1"])
        expect("release refuses an empty Unreleased", code != 0)

        # The first release after the unlinked ones gets a tag link, not a compare link.
        old = ok_text.split("## [0.3.1]")[0].replace("### Security\n- **Medium** · no advisory · affects 0.3.1: something small.\n", "")
        old = old.rstrip("\n") + "\n\n## [0.3.0] - 2026-10-10\n\n### Added\n- Something.\n\n[Unreleased]: %s/commits/main\n" % REPO_URL
        with open(path, "w", encoding="utf-8") as f:
            f.write(old)
        code, _, _ = run(["--file", path, "release", "0.3.1", "--date", "2026-10-12"])
        with open(path, encoding="utf-8") as f:
            text = f.read()
        expect("first linked release", code == 0 and "[0.3.1]: %s/releases/tag/v0.3.1\n" % REPO_URL in text and problems_of(text) == [])

    if failures:
        for name in failures:
            print("FAILED: " + name, file=sys.stderr)
        return 1
    print("changelog.py selftest: ok")
    return 0


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="CHANGELOG.md tools", formatter_class=argparse.RawDescriptionHelpFormatter,
                                     epilog=__doc__)
    parser.add_argument("--file", default=DEFAULT_FILE, help="the changelog (default: CHANGELOG.md at the repository root)")
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("check", help="validate the file")
    p = sub.add_parser("notes", help="a version's entries as Markdown")
    p.add_argument("version")
    p = sub.add_parser("severity", help="the highest severity among a version's Security bullets")
    p.add_argument("version")
    p.add_argument("--advisories", action="store_true", help="print the GHSA ids, one per line, instead")
    p = sub.add_parser("release", help="turn [Unreleased] into a version")
    p.add_argument("version")
    p.add_argument("--date", help="YYYY-MM-DD (default: today)")
    sub.add_parser("selftest", help="run the built-in tests")
    args = parser.parse_args(argv)
    return {"check": cmd_check, "notes": cmd_notes, "severity": cmd_severity, "release": cmd_release,
            "selftest": lambda a: selftest()}[args.command](args)


if __name__ == "__main__":
    sys.exit(main())
