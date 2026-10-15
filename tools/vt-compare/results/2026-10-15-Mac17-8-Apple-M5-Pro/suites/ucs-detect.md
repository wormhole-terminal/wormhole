# ucs-detect in each terminal

- date: 2026-10-15T11:48:57Z
- machine: Mac17,8, Apple M5 Pro, macOS 27.0.1
- checkout: 5e2f529c (with uncommitted changes)
- asked for: 80 x 25 cells, Menlo 13 pt (an app that can't be told from the command line has its own)
- load average at the start: 4.01 3.53 3.48
- suite: ucs-detect 50f011d991a2 (Jeff Quast, MIT), its default run (the contested codepoints and graphemes), wcwidth from suites.pin
- each cell is the share of the sequences tested that the terminal sized as Unicode says (`passed/tested`)

| terminal | wide | narrow | ZWJ emoji | VS16 | VS15 | flags (RI pairs) | lone RI | lone skin tone | languages | grapheme clustering (mode 2027) | ambiguous width | seconds |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Wormhole | 100.0% (506/506) | 100.0% (5595/5595) | 100.0% (1445/1445) | 100.0% (426/426) | 100.0% (158/158) | 100.0% (262/262) | 100.0% (26/26) | 100.0% (5/5) | 100.0% (6589/6589) | enabled | narrow | 0 |
| Ghostty | 99.8% (505/506) | 99.9% (5590/5595) | 100.0% (1445/1445) | 100.0% (426/426) | 100.0% (158/158) | 100.0% (262/262) | 100.0% (26/26) | 100.0% (5/5) | 98.4% (6485/6589) | enabled | narrow | 1 |
| _Ghostty published (1.3.1-arch2, Linux)_ | 99.8% (505/506) | 99.9% (5524/5529) | 100.0% (1445/1445) | 100.0% (426/426) | 100.0% (158/158) | 100.0% (262/262) | 100.0% (26/26) | 100.0% (5/5) | 99.5% (6490/6522) | enabled | narrow | 7 |
| Alacritty | 94.9% (480/506) | 99.6% (5570/5595) | 0.7% (10/1445) | 50.0% (213/426) | 0.0% (0/158) | 100.0% (262/262) | 0.0% (0/26) | 100.0% (5/5) | 84.4% (5558/6589) | no | narrow | 1 |
| _Alacritty published (, Linux)_ | 94.9% (480/506) | 99.5% (4975/5000) | 1.0% (10/1010) | 50.0% (213/426) | 0.0% (0/158) | 100.0% (262/262) | 0.0% (0/26) | 100.0% (5/5) | 85.3% (5563/6522) | no | narrow | 2 |
| kitty | 99.0% (501/506) | 99.6% (5570/5595) | 100.0% (1445/1445) | 97.9% (417/426) | 100.0% (158/158) | 100.0% (262/262) | 100.0% (26/26) | 0.0% (0/5) | 45.4% (2990/6589) | no | narrow | 47 |
| _kitty published (0.48.2, Linux)_ | 97.8% (43587/44568) | 99.9% (36227/36254) | 100.0% (1445/1445) | 97.9% (417/426) | 100.0% (158/158) | 100.0% (262/262) | 100.0% (26/26) | 0.0% (0/5) | 79.5% (19898/25043) | no | narrow | 629 |
| iTerm2 | 0.0% (0/506) | 0.0% (0/5595) | 0.0% (0/1445) | 49.8% (212/426) | 0.0% (0/158) | 0.0% (0/262) | 0.0% (0/26) | 0.0% (0/5) | 81.7% (5384/6589) | no | narrow | 468 |
| _iTerm2 published (3.7.3, Darwin)_ | 97.8% (43591/44568) | 100.0% (36237/36254) | 17.2% (249/1445) | 100.0% (426/426) | 0.0% (0/158) | 100.0% (262/262) | 100.0% (26/26) | 100.0% (5/5) | 93.7% (23462/25043) | no | narrow | 22377 |
| Terminal | 94.9% (480/506) | 98.3% (5500/5595) | 0.0% (0/1445) | 50.0% (213/426) | 0.0% (0/158) | 98.9% (259/262) | 0.0% (0/26) | 100.0% (5/5) | 81.0% (5334/6589) | no | narrow | 9 |
| _Terminal published (488, Darwin)_ | 97.8% (43566/44568) | 99.7% (36159/36254) | 0.0% (0/1445) | 50.0% (213/426) | 0.0% (0/158) | 98.9% (259/262) | 0.0% (0/26) | 100.0% (5/5) | 93.4% (23400/25043) | no | narrow | 37 |


Rows in italics are the results ucs-detect's author published for the same terminal (its data/ folder), made on another machine and often another version: they show whether this harness measures the same thing, not what this run found.

The yaml files in ucs-detect/ hold every failed sequence per terminal.

