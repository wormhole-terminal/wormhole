# Exit codes, error codes and reasons

With `--json` a failure is `{"version":1,"ok":false,"error":{"code":…,"message":…,"reason":…,"block":…}}`
(`reason` and `block` only when there is one). Act on `code` first, then `reason`; the `message` is
words for the user. A code or reason you do not know is a newer Wormhole's: treat it as a refusal and
read the message. `wh help --json` lists them as data.

## Exit codes

| Code | Means | Do |
|---|---|---|
| 0 | it worked | – |
| 1 | the app would not or could not | read `error.code` |
| 2 | the arguments are wrong, or the protocol version is | fix the call; for `version`, tell the user to update |
| 3 | Wormhole is not running or did not answer | tell the user; `wh doctor` says more; `list` and `status` still show background terminals |
| 124 | a `wait` or `run` ran out of time | `reason` is `timeout`; the command still runs; wait again or follow its block |

`run --exit-code` exits with the command's own code (1 to 255), so there 1, 2 and 124 may also be a
refusal, bad arguments or a timeout: use `--json` when it matters.

## `error.code`

- `version` (exit 2): the command line and the app speak different protocols. Ask the user to update Wormhole; do not retry.
- `bad_request` (2): wrong arguments. The message says which; fix them.
- `not_found` (1): no such terminal, window or folder, or its shell ended (`reason: exited`). Re-read ids with `wh list --json`.
- `not_allowed` (1): a Settings switch is off; the message names it. Ask the user; never try to get around it.
- `refused` (1): allowed in general, not into this terminal now; see the reasons.
- `rate_limited` (1): too many requests of this kind; the message says how many seconds to wait.
- `too_large` (1): over a size cap (16 KiB of text, 64 keys, 5000 lines). Send or ask for less.
- `unsupported` (1): this Wormhole cannot do it now: `flag_off` or `no_integration`.
- `failed` (1): the app tried and could not, or a wait ran out (`reason: timeout`).

## `error.reason`

- `timeout`: ran out of time (exit 124). For a `run`, `error.block` names the command still running.
- `no_integration`: the shell reports no command blocks. Use `send-text`/`send-keys` with `wait --match` or `--stable`.
- `not_at_prompt`: the shell is busy or a program has the terminal. Wait for it; do not interrupt a command you did not start.
- `no_mouse`: the program did not ask for the mouse; use keys.
- `no_room`: the window is too small for another pane; use `new-window`.
- `question_open`: a close question is already up in that window; do not ask again.
- `exited`: the terminal's shell ended; nothing more will happen in it.
- `flag_off`: the Automation flag is off in this Wormhole; `wh doctor` says how to turn it on.
- `run_pending`: a `run` is under way in that terminal; wait for it.
- `hidden_profile`: the terminal is in a window of a profile that is not shown; leave it alone.

## The refusals that protect the user

`refused` into your own terminal, into a terminal waiting for the user's answer (an agent's permission
prompt or a question), and into a terminal whose shell ended, whatever the switches say. They are
meant. Stop, report, and let the user act.
