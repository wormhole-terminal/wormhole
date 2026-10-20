# vttest screens

The engine's screen at every `Push <RETURN>` of [vttest](https://invisible-island.net/vttest/) 20251205, written by
`vttest-host` (`crates/wormhole-vt-bench`) and compared with a fresh run by `scripts/vttest-gate.sh`. See
`tools/vt-compare/README.md`, section vttest, for how it runs and what it finds.

- `<nn>-<menu>/<nnn>.txt`: one screen. The header says where it came from (the path of choices on vttest's menus, vttest's
  own names), the size, the cursor (row and column from 1, hidden, the style a program chose), and the modes that differ from
  where vttest starts. Then `--- text`, the 24 rows as `NN|text` with trailing blanks cut, and `--- attributes`, per row the runs of
  columns that carry an attribute (bold, dim, italic, underline, blink, inverse, hidden, strikeout) or a colour (`fg=`, `bg=`, a
  palette number or `#rrggbb`). A screen headed `state: silent without a Push <RETURN>` is one vttest was waiting at for a key
  rather than a `Push <RETURN>`; the host then typed what `plan.rs` says or pressed Return.
- `<nn>-<menu>/review.md`: each screen judged against what vttest says it should show: `pass`, `fail: <why>` or
  `unsupported: <feature>`. The gate does not read it; update it with the screens when the engine's behaviour changes on purpose.
- `notes.txt`: the entries that were not run or not walked again, and why.

Regenerate the screens with `scripts/vttest-gate.sh --update`, and read the diff.
