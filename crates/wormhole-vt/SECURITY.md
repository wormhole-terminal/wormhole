# Security

`wormhole-vt` reads output from programs nobody has vetted: a `cat` of a file from the internet, the
banner of a server, the build log of a repository you just cloned. Reports about that are welcome.

The threat model, what the engine guarantees and what the embedding application must still do are in
[docs/security.md](docs/security.md).

## Reporting a vulnerability

Please report privately through GitHub: **Security → Report a vulnerability** on
[wormhole-terminal/wormhole](https://github.com/wormhole-terminal/wormhole/security/advisories/new). The
engine lives in that repository, in `crates/wormhole-vt`, and shares its advisories. Start the title with
`wormhole-vt:`. Don't open a public issue for it.

Include the bytes that cause it (a hex dump or a file is best), the host calls around them if any, what
the attacker gains, and the version or commit.

## Supported versions

The latest 0.x release is supported. Fixes ship as a new patch release of it (0.1.1, not 0.2.0) as soon as
they are ready. Older 0.x releases do not get fixes; upgrade.

## How a report is handled

1. The report arrives privately, as a draft [GitHub Security Advisory](https://docs.github.com/en/code-security/security-advisories).
2. The fix is written in the advisory's temporary private fork. It is not pushed to the public repository,
   and no public issue or pull request mentions it, before the release.
3. It ships as a patch release. The advisory is published with the release, and a CVE is requested through
   GitHub. The reporter is credited if they want to be.
4. [CHANGELOG.md](CHANGELOG.md) lists the fix under **Security**, with the affected versions.

## What counts

For the engine, these are vulnerabilities:

- **A crash or a hang.** A panic, an endless loop or a stall that some input to `Terminal::feed` (or to
  the parser alone) causes, whatever the host does.
- **Unbounded memory.** Input that makes the terminal's memory grow without limit, or past the caps in
  [docs/security.md](docs/security.md), or a cap that does not hold.
- **Report injection.** A reply to the program that contains bytes the program chose, other than the few
  documented cases.
- **A request that bypasses its switch.** A window operation, a title report or a screen read-back that
  works while `Policy` says no, or a clipboard or notification request that reaches the host any other way
  than as an event.
- **A sequence that does something it must never do.** Reading or writing a file, starting a program,
  loading a font, or opening a control channel such as tmux's.
- **State that misleads the host about safety.** For example a hyperlink target the engine reports
  differently from what the program sent, or a picture kept beyond the documented budgets.

These are not vulnerabilities of the engine, though they may be worth a report to the application using it:
a host that answers an OSC 52 read without asking, opens a link without showing its target, or passes a
notification's text to a shell. A malicious host application, another process on the same machine and bugs
in an image decoder the host chose are out of scope.

Plain bugs (a wrong cell, a wrong colour) are ordinary issues.
