# Security

Wormhole runs programs, reads repositories nobody has vetted and lets agents and other programs
talk to it, so security reports are welcome.

## Reporting a vulnerability

Please report privately through GitHub: **Security → Report a vulnerability** on
[wormhole-terminal/wormhole](https://github.com/wormhole-terminal/wormhole/security/advisories/new).
Don't open a public issue for it. Include what an attacker controls, what they gain, and the steps
or a proof of concept.

## Supported versions

Only the latest release is supported: fixes ship as a new version through the built-in updater.

## How a report is handled

1. The report arrives privately, as a draft [GitHub Security Advisory](https://docs.github.com/en/code-security/security-advisories).
2. The fix is written in the advisory's temporary private fork. It is never pushed to the public
   repository, and no public issue or pull request mentions it, before the release.
3. It ships as a patch release on the latest version (0.3.1, not 0.4.0), as soon as it is ready.
4. The advisory is published with the release, and a CVE is requested through GitHub. The reporter is
   credited if they want to be.
5. [CHANGELOG.md](CHANGELOG.md) lists the fix under **Security** with its severity, the advisory and the
   affected versions, and the release is marked as a security update in the update feed: the app shows
   it as one.

## What counts

What Wormhole defends, and against whom:

- **A repository's author.** Everything in a cloned repository is hostile input: `.wormhole/`
  project folders, `.git/config`, file names and contents. Nothing in a project may run a command,
  change the environment or read outside the repository without the user's approval (Touch ID).
- **Output in a terminal.** Escape sequences from a program, a file or a remote shell must not run
  commands, write files or read the clipboard without the user.
- **Another process of the same user** talking to the agent-status socket or the `Wormhole` command
  line.
- **A network attacker** on the update feed: updates are verified with a minisign signature and
  Apple's code signature before they are installed.

Out of scope: another account on the Mac getting nothing (the socket is `0600` and checks the peer),
an attacker who already runs code as the user, and issues in the coding agents themselves.
