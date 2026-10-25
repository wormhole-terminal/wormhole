Text rules. `probe.toml` has three rules: a ticket id that opens a link, an email address that is shortened,
and a key that is hidden. Change the patterns, the tracker's host (in `permissions.open` and in the rule's
`url`), and the cases in `tests/example.toml`. Keep one case per rule that matches, and one that must not.
