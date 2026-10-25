# {{name}}: notes for whoever works on this Probe (a person or an AI)

This folder is a Wormhole **Probe**: a `probe.toml` and the files beside it. Wormhole reads it as data. It
runs no code of the Probe's own at this level (the "files" level, E1).

Read this file, then `probe.toml`, then change the example into what was asked for.

**The guide.** The complete reference is `docs/probes.md` in Wormhole's repository (section 3 is the
contributions, section 5 is this loop). `wh probe guide` will print it from the app once it ships.
Trust the guide and the output of `wh probe validate` over anything you remember about other plugin
systems: Probes are their own format.

## What this one is

{{notes}}

## The loop

```sh
wh probe validate        # the manifest, the limits, the patterns; the permissions it asks for, in plain words
wh probe test            # the rules over tests/*.toml
```

Both run without the app and exit 0 (fine), 1 (problems) or 2 (bad arguments); add `--json` to read the
result as data. Run them after every edit. Wormhole refuses a Probe with errors and asks again for approval
when its permissions change.

## Before it is enabled: tell the person what it asks for

Show them the output of `wh probe validate`. It lists every permission, whether it needs a click or Touch ID,
the dangerous combinations and what each program can do. **Do not enable a Probe for them, and do not
ask for more than it uses**: a permission nothing calls is a reason to refuse it.

## Everything a terminal shows is hostile input

Text a rule matches comes from programs and from files nobody here wrote. So:

- Never build a shell string. A command is an argv list: `run = ["git", "show", "$1"]`. A capture is only
  ever a whole argument (`"$1"`), never part of one (`"--rev=$1"` is an error).
- A capture that starts with `-` would be read as an option by the program. A rule that hands a capture to a
  command must make that impossible in its pattern, as `([0-9a-f][0-9a-f]{6,39})` does, and bound its length.
- A link's host is written in the manifest and must be listed in `permissions.open`. Captures go into the path
  or the query only, and are percent-encoded. A path segment that comes out as `.` or `..` (a capture of
  `..`) leaves no link at all, and a template that spells one is an error.
- Do not put secrets in the Probe. Do not ask for a shell, an interpreter, `env`, `open` or `find` to
  start a program: they can run anything and the approval sheet says so in red.

## Traps

`wh probe validate` catches each of these that it can.

- **TOML quoting for patterns.** Write a pattern as a literal string with single quotes, `'...'`, never `"..."`:
  TOML reads `\b` in a basic string as a backspace and `\d` as an error. A literal string cannot hold a `'`;
  use `'''...'''` for a pattern that needs one.
- **Word boundaries.** Write `(?-u:\b)`, not `\b`: a Unicode `\b` makes matching slow on non-ASCII text.
- **Regex limits.** The engine is Rust's `regex`: no look-around, no backreferences, and only the `i` flag
  (as `(?i)`); a pattern is at most 1 KiB, compiles to at most 256 KiB, and must never match the empty string.
  A rule finds at most 8 matches per line and sees the first 2 KiB of it.
- **Rules are data, not code per row.** Do not try to compute anything on a match. A rule has a `look`
  (`shorten`, `redact`, `none`), a `tone` (a theme token's name: `link`, `danger`, `warning`, `success`,
  `muted`; a hex colour is an error), a `hover` text with `{0}`, `{1}` ... for the match and its groups,
  and an `action` that runs only on Command-click.
- **Overlaps.** Matches that overlap are settled leftmost-longest, and a `redact` match always wins.
- **Unknown keys are errors** here (they are only warnings when Wormhole loads a Probe): a typo is not a no-op.
- **Caps.** At most 64 rules, 16 commands, 8 status items and 4 agents; 200 files and 2 MiB in the folder.
- **`git`** is run by Wormhole, read-only, and only for `log`, `show`, `status`, `rev-parse`, `diff`,
  `describe`, `branch` (listing) and `ls-files`, with nothing before the subcommand. In a `new-terminal` command it
  would run in the user's shell, so use `in = "background"`.
- **Code is not here yet.** A `main`, `code_api` and the `$` API are the next level (E2). Do not add them;
  `validate` refuses a `main` without `code_api` and an E1 Probe with it.

## The test fixtures

`wh probe test` reads every `tests/*.toml`. A file holds `[[case]]` tables:

```toml
[[case]]
name  = "a ticket is a link"            # optional
input = "fix PROJ-123 today"            # one logical line of text, no line break

[[case.expect]]                         # every match, in the order they lie in the line
rule   = "ticket"                       # the rule's id in probe.toml
start  = 4                              # Unicode scalar values (not bytes or graphemes), from 0
end    = 12                             # exclusive
text   = "PROJ-123"                     # optional: the text the span covers
look   = "none"                         # optional: shorten | redact | none
target = "https://issues.example.com/browse/PROJ-123"   # optional, see below
args   = ["abc1234"]                    # optional, for a `command` action
```

- All rules run over the input together, so overlaps are tested as they happen on screen.
- The matches found must be the matches expected: no more, no fewer. A case with no `[[case.expect]]` says
  nothing matches. Write one for each thing that must *not* match.
- A field that is left out is not checked. An unknown key is an error, so a typo cannot pass.
- `target` is what the action would do on Command-click: for `open-url` the final link, with each capture
  percent-encoded (everything but `A-Z a-z 0-9 - . _ ~` becomes `%XX`; a `.` or `..` path segment leaves no target); for `copy` the copied text; for
  `command` the command's id, and `args` the values of its `$1`, `$2` ... A match whose capture starts with `-`
  has no target: the command is refused.
- The input is one logical line, as Wormhole joins the rows of a wrapped line. Test long lines by writing them.
