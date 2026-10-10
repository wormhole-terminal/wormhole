#!/usr/bin/env python3
"""Commit messages: Conventional Commits 1.0 with Wormhole's footers (CONTRIBUTING.md, "Commits").

  scripts/commits.py check-message <file|->        one message (a file, or - for stdin)
      --strip-comments                             drop git's `#` lines and the scissors section (the commit-msg hook)
      --allow-fixup                                accept `fixup! …` / `squash! …` (the hook; a range may not have them)
      --max-length N                               longest header (default 100; a PR title: 92, GitHub adds ` (#1234)`)
  scripts/commits.py check-range <rev-range>       every commit in the range: well formed, and no merge commits
  scripts/commits.py changelog-needed <rev-range>  fail when a feat/fix/perf, breaking or Security commit is in
                                                   the range and the range doesn't touch CHANGELOG.md
      --include-worktree                           also count uncommitted changes to CHANGELOG.md
  scripts/commits.py selftest

Header: `type(scope,scope)!: subject`, at most 100 characters.
  type    feat fix perf refactor docs test build ci chore style revert
  scope   optional, lowercase a-z 0-9 and -, several separated by commas without spaces
  !       before the colon: a breaking change (or a `BREAKING CHANGE:` footer)
  subject starts lowercase (a name such as Wormhole or an acronym such as GPUI may keep its capitals), no final period
Footers (the last paragraph):
  Security: GHSA-xxxx-xxxx-xxxx   a security fix (or `Security: <short text>` before the advisory exists)
  Changelog: none                 a feat/fix/perf that needs no CHANGELOG.md line (never with Security)

Python 3.9, standard library only.
"""
from __future__ import annotations

import argparse
import contextlib
import io
import os
import re
import subprocess
import sys
import tempfile
from typing import List, NamedTuple, Optional, Tuple

TYPES = ["feat", "fix", "perf", "refactor", "docs", "test", "build", "ci", "chore", "style", "revert"]
# These end up in the changelog, so a range with one needs a CHANGELOG.md entry (or `Changelog: none`).
USER_VISIBLE = ["feat", "fix", "perf"]
MAX_HEADER = 100
# Capitalised words a subject may start with. Anything with a second capital or a digit (GPUI,
# OpenCode, F-36) is allowed without being listed.
PROPER = {
    "Wormhole", "Claude", "Codex", "Gemini", "Copilot", "Cursor", "Dependabot", "Intel", "Apple", "GitHub",
    "Homebrew", "Ghostty", "Touch", "Metal", "Xcode", "Sixel", "Unicode", "Rust", "Cargo", "Linux", "Python",
}
HEADER_RE = re.compile(r"(?P<type>[^\s(!:]+)(?:\((?P<scope>[^)]*)\))?(?P<bang>!)?: (?P<subject>.*)")
SCOPE_RE = re.compile(r"[a-z0-9-]+(?:,[a-z0-9-]+)*")
FOOTER_RE = re.compile(r"(?P<token>[A-Za-z][A-Za-z0-9-]*|BREAKING CHANGE):(?: (?P<value>.*))?")
SIGNIFICANT_RE = re.compile(r"(?:Security|Changelog|BREAKING[ -]CHANGE):")
GHSA_ID = re.compile(r"GHSA-[0-9a-z]{4}-[0-9a-z]{4}-[0-9a-z]{4}")
SCISSORS = "# ------------------------ >8 ------------------------"


class Parsed(NamedTuple):
    errors: List[str]
    type: Optional[str]
    breaking: bool
    security: bool
    changelog_none: bool

    @property
    def needs_changelog(self) -> bool:
        if self.changelog_none and not self.security:
            return False
        return self.security or self.breaking or self.type in USER_VISIBLE


def strip_comments(message: str) -> str:
    lines = []
    for line in message.split("\n"):
        if line == SCISSORS:
            break
        if not line.startswith("#"):
            lines.append(line)
    return "\n".join(lines)


def check(message: str, max_length: int = MAX_HEADER, allow_fixup: bool = False) -> Parsed:
    errors: List[str] = []
    lines = message.replace("\r\n", "\n").strip("\n").split("\n")
    lines = [line.rstrip() for line in lines]
    header = lines[0] if lines and lines[0] else ""

    def bad(text: str) -> Parsed:
        return Parsed(errors + [text], None, False, False, False)

    if not header:
        return bad("the message is empty")
    if re.match(r"(fixup|squash|amend)! ", header):
        if not allow_fixup:
            return bad("`%s` commits are squashed before they are pushed (git rebase -i --autosquash)" % header.split("!")[0] + "!")
        return Parsed([], None, False, False, False)
    if re.match(r"Merge ", header):
        return bad("merge commit: history is linear, rebase onto origin/main instead of merging")
    if header.startswith('Revert "'):
        return bad("write a revert as `revert: <what it undoes>`, not git's default message")
    if len(header) > max_length:
        errors.append("header is %d characters, the limit is %d" % (len(header), max_length))
    m = HEADER_RE.fullmatch(header)
    if not m:
        return bad("header must read `type(scope): subject` (types: %s)" % ", ".join(TYPES))
    ctype, scope, bang, subject = m.group("type"), m.group("scope"), m.group("bang"), m.group("subject")
    if ctype not in TYPES:
        hint = " (lowercase)" if ctype.lower() in TYPES else ""
        errors.append("unknown type `%s`%s: use %s" % (ctype, hint, ", ".join(TYPES)))
    if scope is not None and not SCOPE_RE.fullmatch(scope):
        errors.append("scope `%s` must be lowercase a-z, 0-9 and -, several separated by commas without spaces" % scope)
    if not subject.strip():
        errors.append("the subject is empty")
    else:
        if subject != subject.strip():
            errors.append("the subject has stray spaces")
        first = subject.strip().split(" ")[0]
        if (
            first[:1].isupper()
            and first not in PROPER
            and first.rstrip(".,:;'’s") not in PROPER
            and not re.search(r"[A-Z0-9]", first[1:])
        ):
            errors.append("the subject starts lowercase (`%s` -> `%s`)" % (first, first[:1].lower() + first[1:]))
        if subject.rstrip().endswith("."):
            errors.append("the subject has no final period")

    if len(lines) > 1 and lines[1]:
        errors.append("line 2 must be blank (header, blank line, body)")

    # Footers: the last paragraph after the header.
    paragraphs = [p for p in "\n".join(lines[1:]).split("\n\n") if p.strip()]
    footers: List[Tuple[str, str]] = []
    for i, paragraph in enumerate(paragraphs):
        for line in paragraph.split("\n"):
            fm = FOOTER_RE.fullmatch(line.strip())
            if i == len(paragraphs) - 1:
                if fm:
                    footers.append((fm.group("token"), fm.group("value") or ""))
            elif SIGNIFICANT_RE.match(line):
                errors.append("`%s` is a footer: put it in the last paragraph of the message" % line.split(":")[0])

    breaking = bool(bang)
    security = changelog_none = False
    for token, value in footers:
        value = value.strip()
        if token in ("BREAKING CHANGE", "BREAKING-CHANGE"):
            breaking = True
            if not value:
                errors.append("`BREAKING CHANGE:` needs a description")
        elif token == "Security":
            security = True
            if not value:
                errors.append("`Security:` needs a GHSA id (Security: GHSA-xxxx-xxxx-xxxx) or a short description")
            for word in re.findall(r"GHSA-\S*", value, flags=re.I):
                if not GHSA_ID.fullmatch(word.rstrip(".,;)")):
                    errors.append("`%s` is not an advisory id (GHSA-xxxx-xxxx-xxxx, lowercase)" % word)
        elif token == "Changelog":
            if value == "none":
                changelog_none = True
            else:
                errors.append("`Changelog:` only takes `none` (otherwise the entry goes in CHANGELOG.md)")
    if security and changelog_none:
        errors.append("a Security fix always gets a CHANGELOG.md entry: drop `Changelog: none`")
    return Parsed(errors, ctype if ctype in TYPES else None, breaking, security, changelog_none)


# --------------------------------------------------------------------------- git


def git(*args: str) -> str:
    try:
        out = subprocess.run(["git", *args], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    except subprocess.CalledProcessError as e:
        sys.exit("error: git %s: %s" % (" ".join(args), e.stderr.decode("utf-8", "replace").strip()))
    return out.stdout.decode("utf-8", "replace")


class Commit(NamedTuple):
    sha: str
    parents: List[str]
    message: str


def commits_in(rev_range: str) -> List[Commit]:
    if rev_range.startswith("-"):
        sys.exit("error: %r is not a revision range" % rev_range)
    raw = git("log", "--reverse", "--format=%H%x1f%P%x1f%B%x1e", rev_range, "--")
    found = []
    for record in raw.split("\x1e"):
        if not record.strip():
            continue
        sha, parents, message = record.strip("\n").split("\x1f", 2)
        found.append(Commit(sha, parents.split(), message))
    return found


def report(commit: Commit, errors: List[str]) -> None:
    subject = commit.message.strip().split("\n")[0]
    for e in errors:
        print("%s %s\n    %s" % (commit.sha[:9], subject, e), file=sys.stderr)


def cmd_check_message(args) -> int:
    try:
        text = sys.stdin.read() if args.file == "-" else open(args.file, encoding="utf-8").read()
    except OSError as e:
        sys.exit("error: cannot read %s: %s" % (args.file, e))
    if args.strip_comments:
        text = strip_comments(text)
    result = check(text, args.max_length, args.allow_fixup)
    for e in result.errors:
        print("commit message: %s" % e, file=sys.stderr)
    if result.errors:
        print("see CONTRIBUTING.md, \"Commits\": `type(scope): subject`, e.g. `fix(sidebar): no flicker while scrolling`", file=sys.stderr)
    return 1 if result.errors else 0


def cmd_check_range(args) -> int:
    commits = commits_in(args.range)
    bad = 0
    for commit in commits:
        if len(commit.parents) > 1:
            report(commit, ["merge commit: history is linear, rebase onto origin/main instead of merging"])
            bad += 1
            continue
        errors = check(commit.message).errors
        if errors:
            report(commit, errors)
            bad += 1
    if bad:
        print("%d of %d commit(s) in %s break the commit rules (CONTRIBUTING.md, \"Commits\")" % (bad, len(commits), args.range), file=sys.stderr)
        return 1
    return 0


def cmd_changelog_needed(args) -> int:
    commits = [c for c in commits_in(args.range) if len(c.parents) <= 1]
    needed = [(c, check(c.message)) for c in commits]
    needed = [(c, r) for c, r in needed if not r.errors and r.needs_changelog]
    if not needed:
        return 0
    if git("log", "--format=%H", args.range, "--", "CHANGELOG.md").strip():
        return 0
    if args.include_worktree:
        dirty = git("status", "--porcelain", "--", "CHANGELOG.md")
        if dirty.strip():
            return 0
    print("CHANGELOG.md has no entry for this change. These commits are user-visible:", file=sys.stderr)
    for c, r in needed:
        why = "Security" if r.security else "breaking" if r.breaking else r.type
        print("  %s %s  [%s]" % (c.sha[:9], c.message.strip().split("\n")[0], why), file=sys.stderr)
    print(
        "Add the entry under `## [Unreleased]` in the same change (CONTRIBUTING.md, \"Changelog\"), "
        "or end the commit with `Changelog: none` if users won't notice.",
        file=sys.stderr,
    )
    return 1


# --------------------------------------------------------------------------- selftest


def selftest() -> int:
    failures: List[str] = []

    def ok(message: str, **kw) -> None:
        errors = check(message, **kw).errors
        if errors:
            failures.append("should pass: %r -> %s" % (message, errors))

    def fails(message: str, needle: str, **kw) -> None:
        errors = check(message, **kw).errors
        if not any(needle in e for e in errors):
            failures.append("should fail with %r: %r -> %s" % (needle, message, errors))

    ok("feat(sidebar): terminals can be pinned")
    ok("fix: no flicker")
    ok("feat(a-b,c1)!: drop the old format")
    ok("refactor!: Wormhole everywhere, the andromeda codename retired")
    ok("chore(release): 0.3.1")
    ok("revert: feat(sidebar): terminals can be pinned")
    ok("feat(devctl): `scroll` turns the wheel over a point")
    ok("feat(terminal): ⌘V over a program's own selection replaces it")
    ok("feat(palette): F-36's scope row")
    ok("fix(git): GPUI's cache is dropped on reload")
    ok("ci: bump actions/checkout from 4 to 5")
    ok("fix(update): a bad signature is refused\n\nBody text.\n\nSecurity: GHSA-abcd-1234-wxyz")
    ok("fix(update): a bad signature is refused\n\nSecurity: the signature check could be skipped")
    ok("fix(update): never released\n\nChangelog: none")
    ok("feat: x\n\nBREAKING CHANGE: the file moved")
    ok("fixup! feat: x", allow_fixup=True)
    fails("fixup! feat: x", "squashed")
    fails("", "empty")
    fails("Added a thing", "header must read")
    fails("Feat: a thing", "lowercase")
    fails("feature: a thing", "unknown type")
    fails("feat(Sidebar): a thing", "scope")
    fails("feat(a, b): a thing", "scope")
    fails("feat(): a thing", "scope")
    fails("feat:a thing", "header must read")
    fails("feat: Add a thing", "starts lowercase")
    fails("feat: a thing.", "final period")
    fails("feat: ", "header must read")
    fails("feat: " + "x" * 100, "limit is 100")
    fails("feat: " + "x" * 90, "limit is 92", max_length=92)
    fails("feat: a thing\nbody right below", "line 2")
    fails('Revert "feat: a thing"', "revert:")
    fails("Merge branch 'x'", "merge commit")
    fails("fix: x\n\nSecurity: GHSA-12", "advisory id")
    fails("fix: x\n\nSecurity: ", "needs a GHSA")
    fails("fix: x\n\nChangelog: skip", "only takes `none`")
    fails("fix: x\n\nSecurity: GHSA-abcd-1234-wxyz\nChangelog: none", "always gets")
    fails("fix: x\n\nSecurity: GHSA-abcd-1234-wxyz\n\nMore body after.", "last paragraph")
    fails("feat: x\n\nBREAKING CHANGE: ", "needs a description")

    for message, expect in [
        ("feat: a", True), ("fix: a", True), ("perf: a", True), ("docs: a", False), ("chore: a", False),
        ("fix: a\n\nChangelog: none", False), ("feat!: a", True), ("chore: a\n\nBREAKING CHANGE: b", True),
        ("chore: a\n\nSecurity: something", True), ("fix: a\n\nSecurity: something", True),
    ]:
        if check(message).needs_changelog != expect:
            failures.append("needs_changelog(%r) should be %s" % (message, expect))
    if strip_comments("feat: a\n\n# Please enter\nbody\n" + SCISSORS + "\ndiff") != "feat: a\n\nbody":
        failures.append("strip_comments")

    # git: a throwaway repository.
    def run_cli(argv: List[str], cwd: str) -> Tuple[int, str]:
        err = io.StringIO()
        old = os.getcwd()
        os.chdir(cwd)
        try:
            with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
                try:
                    code = main(argv)
                except SystemExit as e:
                    code = 2 if isinstance(e.code, str) else (e.code or 0)
                    if isinstance(e.code, str):
                        print(e.code, file=sys.stderr)
        finally:
            os.chdir(old)
        return code, err.getvalue()

    with tempfile.TemporaryDirectory() as tmp:
        env = dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_SYSTEM=os.devnull,
                   GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@example.com", GIT_COMMITTER_NAME="t", GIT_COMMITTER_EMAIL="t@example.com")

        def g(*a: str) -> str:
            return subprocess.run(["git", *a], cwd=tmp, env=env, check=True, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE).stdout.decode()

        def commit(message: str, path: str = "f.txt") -> None:
            with open(os.path.join(tmp, path), "a") as f:
                f.write(message + "\n")
            g("add", path)
            g("commit", "-q", "--no-verify", "-m", message)

        old_env = dict(os.environ)
        os.environ.update(env)
        try:
            g("init", "-q", "-b", "main")
            commit("chore: start")
            g("tag", "base")
            code, _ = run_cli(["check-range", "base..HEAD"], tmp)
            if code != 0:
                failures.append("an empty range passes")
            commit("docs: words")
            commit("fix(git): a thing")
            code, err = run_cli(["check-range", "base..HEAD"], tmp)
            if code != 0:
                failures.append("good commits pass: " + err)
            code, err = run_cli(["changelog-needed", "base..HEAD"], tmp)
            if code != 1 or "fix(git): a thing" not in err:
                failures.append("changelog-needed fails without an entry: %d %s" % (code, err))
            commit("fix(git): quiet one\n\nChangelog: none", "g.txt")
            commit("## [Unreleased]", "CHANGELOG.md")  # not a valid message, but a commit touching the file
            code, _ = run_cli(["changelog-needed", "base..HEAD"], tmp)
            if code != 0:
                failures.append("changelog-needed passes when CHANGELOG.md is touched")
            code, _ = run_cli(["check-range", "base..HEAD"], tmp)
            if code != 1:
                failures.append("check-range rejects a bad message")
            code, _ = run_cli(["changelog-needed", "HEAD..HEAD"], tmp)
            if code != 0:
                failures.append("changelog-needed passes on an empty range")
            g("tag", "mid")
            commit("feat: later")
            code, err = run_cli(["changelog-needed", "mid..HEAD"], tmp)
            if code != 1:
                failures.append("changelog-needed per range")
            with open(os.path.join(tmp, "CHANGELOG.md"), "a") as f:
                f.write("more\n")
            code, _ = run_cli(["changelog-needed", "--include-worktree", "mid..HEAD"], tmp)
            if code != 0:
                failures.append("--include-worktree counts an uncommitted entry")
            g("checkout", "-q", "-b", "side", "mid")
            commit("fix: on a side branch", "s.txt")
            g("checkout", "-q", "main")
            g("merge", "--no-ff", "-q", "-m", "Merge side", "side")
            code, err = run_cli(["check-range", "mid..HEAD"], tmp)
            if code != 1 or "merge commit" not in err:
                failures.append("check-range rejects merges: " + err)
            msg = os.path.join(tmp, "msg")
            with open(msg, "w") as f:
                f.write("fix: ok\n\n# Please enter the commit message\n")
            code, _ = run_cli(["check-message", "--strip-comments", msg], tmp)
            if code != 0:
                failures.append("check-message from a file")
            code, _ = run_cli(["check-range", "--bogus"], tmp)
            if code == 0:
                failures.append("a flag is not a range")
        finally:
            os.environ.clear()
            os.environ.update(old_env)

    if failures:
        for f in failures:
            print("FAILED: " + f, file=sys.stderr)
        return 1
    print("commits.py selftest: ok")
    return 0


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Commit message rules", formatter_class=argparse.RawDescriptionHelpFormatter,
                                     epilog=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("check-message", help="check one message")
    p.add_argument("file", help="a file, or - for stdin")
    p.add_argument("--strip-comments", action="store_true")
    p.add_argument("--allow-fixup", action="store_true")
    p.add_argument("--max-length", type=int, default=MAX_HEADER)
    p = sub.add_parser("check-range", help="check every commit in a revision range")
    p.add_argument("range")
    p = sub.add_parser("changelog-needed", help="fail when user-visible commits come without a CHANGELOG.md change")
    p.add_argument("range")
    p.add_argument("--include-worktree", action="store_true")
    sub.add_parser("selftest", help="run the built-in tests")
    args = parser.parse_args(argv)
    return {"check-message": cmd_check_message, "check-range": cmd_check_range,
            "changelog-needed": cmd_changelog_needed, "selftest": lambda a: selftest()}[args.command](args)


if __name__ == "__main__":
    sys.exit(main())
