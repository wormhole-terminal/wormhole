# JetBrains Mono

The default terminal and code font, compiled into the app
(`wormhole_core::fonts::bundled`).

- Version 2.304, from the official release
  <https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304>
  (`JetBrainsMono-2.304.zip`, SHA-256
  `6f6376c6ed2960ea8a963cd7387ec9d76e3f629125bc33d1fdcd7eb7012f7bbf`). The four
  static TTFs from `fonts/ttf/` are unmodified and byte-identical to the files in
  the repository at tag `v2.304`.
- Only Regular, Bold, Italic and Bold Italic: the weights terminal and code text
  use. Other weights and the webfonts are left out to keep the app small.
- Copyright 2020 The JetBrains Mono Project Authors (see `AUTHORS.txt`),
  licensed under the SIL Open Font License 1.1 (`OFL.txt`), which allows
  bundling and redistribution with software. `scripts/bundle.sh` copies
  `OFL.txt` into the app at `Contents/Resources/Licenses/JetBrainsMono-OFL.txt`.
