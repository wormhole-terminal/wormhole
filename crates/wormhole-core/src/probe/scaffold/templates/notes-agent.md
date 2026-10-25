A coding agent definition. `probe.toml` has an agent that Wormhole recognises by the program name it runs
as, with a monogram and an icon (`assets/icon.svg`: one colour, paths only). The agent reports its own status
(OSC 7501), so nothing is written to its configuration and no permission is needed. Change the names, the
`executables` the agent runs as (they cannot be the name of a built-in agent) and the icon. This Probe has no
text rules, so `wh probe test` has nothing to run yet.
