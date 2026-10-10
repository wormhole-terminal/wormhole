# Visual check

Screenshots of the real app that come out the same on every run, so a UI change can be compared
with another revision pixel by pixel. Needs macOS and a display; Python 3 with Pillow and numpy for
the pictures of the differences (without them the runner only says whether files are identical).

```sh
scripts/visual-check.sh home changes-diff        # screenshots of two scenarios
scripts/visual-check.sh all --against origin/main # every scenario, this tree vs main, diffed
scripts/visual-check.sh all --repeat              # twice on this tree: is it deterministic?
```

It builds this tree once (`cargo build --features devctl`, in `$CARGO_TARGET_DIR`), copies the
binary out, and runs each scenario in its own isolated instance: scratch data dir, `HOME` and agent
home, the Finder's `PATH` (`/usr/bin:/bin:/usr/sbin:/sbin`) and nowhere else to find agents'
commands (`WORMHOLE_AGENT_PATH` empty, so no agent counts as installed whatever the Mac has; a
`@shell` copy is the one that does), `/bin/cat` (or that copy, named like an agent) for a shell,
Touch ID answered "cancel", notifications logged and update checks off, and background mode
(`WORMHOLE_BACKGROUND=1`): each instance opens behind your windows and never takes the focus, so
you can keep working during a run.
It stops only the processes it started, by PID. `--against <rev>` builds that commit once in a
temporary worktree (removed again; the binary is kept in `$CARGO_TARGET_DIR/visual-check-bins/<commit>`);
a revision from before background mode comes to the front.

Results go to `--out` (default `$TMPDIR/wormhole-visual/<time>`, never the repository):

| Path | What |
|---|---|
| `branch/<scenario>-<shot>.png` | this tree's screenshots (`rev-<rev>/`, `branch-again/` for the other runs) |
| `diff/<name>-side.png` | before, after and the changed pixels in red side by side, at 1x |
| `diff/<name>-overlay.png` | the changed pixels alone |
| `results.txt` | per shot: changed pixels, their bounding box in logical px, the scale |
| `summary.txt` | every shot with its file |
| `<label>/<scenario>/` | the instance: `devctl.log` (each batch and its answers), `app.log`, `data/`, `home/` |

Screenshots are at the display's scale (2 on Retina, recorded in `results.txt`); `bounds` and layout
numbers are logical pixels, so the side-by-side pictures are scaled to 1x.
Screenshots keep the window's translucency in their alpha channel; the pictures show it over black.

## Scenarios

`scenarios/<name>.txt`: devctl command lines (see the header of `crates/wormhole/src/devctl/mod.rs`)
and directives. Lines are sent in batches; the runner adds `sync` to each and waits for it.

| Directive | Meaning |
|---|---|
| `@fixture repo-basic` | `fixtures/<name>.sh` builds the repository the first terminal opens in (`~/<name>`) |
| `@shell claude` | the shell is a copy of `/bin/cat` with that name, so agent detection sees that agent, and it is the one agent Settings and the first-run sheet count as installed |
| `@theme Tokyo Night` | the theme, in `config.json` before launch |
| `@config key=<json>` | any other `config.json` key before launch |
| `@env KEY=VALUE` | extra environment for the instance |
| `@size 1440 900` | the main window's size in logical px (the default) |
| `@wait-idle [ms]` | wait until no git work is in flight and every window stopped changing |
| `@batch` | end the batch: what follows runs on a later tick (a window opened just now exists then) |
| `@sleep <ms>` | end the batch and wait |
| `@shot <name> [main\|settings\|studio]` | wait until idle, then a screenshot `<scenario>-<name>.png` |

Every instance starts with `reduce-motion on` (the needs-you pulse held still, the slides and the
chevrons' turns cut), `onboarding close` (a fresh data dir is a first launch, whose sheet would cover
the window), `appearance dark`, `reduce-transparency off` and `increase-contrast off` (the Mac's Light
Mode, Reduce Transparency or Increase Contrast would otherwise change the theme shown under Match
system, the window's translucency or the tokens) and `resize`. The keyboard's focus ring hides in a
window that isn't in front, as an instance's is: a scenario that shoots it says `window-front on`.

## Fixtures

`fixtures/<name>.sh <dir>` creates a git repository through `fixtures/lib.sh`: no global or system
git config, one identity, files written byte for byte, and commit dates whole days before the start
of the current UTC day (`commit <days> <message>`). History shows ages, so a relative date is what
keeps the picture still; anchoring it to the day keeps the commit hashes the same too. Two runs on
either side of midnight UTC differ in History.

## Determinism

What keeps two runs identical: fixed fixtures, a scratch `HOME` (paths read `~/…`, no user git
config), a fixed `PATH` and no agent folders (what the Mac has installed stays out of Settings and
the first-run sheet), `reduce-motion on`, `wait-idle` before every shot (no git refresh in flight, and every
window draws the same picture three times in a row: that also covers a diff being built, which has
no flag of its own), notifications off, and background mode, in which an instance's window is never
the active one whatever you do in other apps. What can still move a picture: a focused text field's
caret blinks regardless of Reduce Motion (keep the focus on the terminal), and a click on an
instance's window makes it the active one (don't click them during a run).

A build from before `sync` and `wait-idle` (an old `--against` revision) still runs: the runner
notices, waits by time instead and says so; its commands that don't exist yet are listed as missing.

`diff.py` on its own: `diff.py pair a.png b.png --out d --name x`, `diff.py dirs runA runB --out d`
(every PNG in both; exit 1 if any differs).
