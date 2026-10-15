### Throughput, MB/s (the fastest of the passes)

| corpus | alacritty | wormhole | wezterm | vt100 | ghostty |
|---|--:|--:|--:|--:|--:|
| ascii-flood | 233 | **2764** | 67 | 257 | 1800 |
| log-lines | 248 | **1528** | 66 | 241 | 1207 |
| sgr-churn | 232 | **379** | 49 | 253 | 165 |
| unicode-mix | 207 | 330 | 70 | 212 | **414** |
| tui-redraw | 244 | **754** | 42 | 210 | 465 |
| scroll-region | 212 | **809** | 57 | 220 | 648 |
| osc8-links | 191 | **469** | 70 | 299 | 27 |
| payloads | 422 | 569 | 291 | 389 | **1008** |
| agent-session | 320 | **534** | 71 | 353 | 231 |
| **vs the first engine** | 1.00x | 2.85x | 0.29x | 1.06x | 1.62x |

### Allocator calls per MB: the first pass / a second pass on the same engine

| corpus | alacritty | wormhole | wezterm | vt100 | ghostty |
|---|--:|--:|--:|--:|--:|
| ascii-flood | 1193 / 0 | 19 / 0 | 25211 / 25214 | 12499 / 12500 | 0 / 0 |
| log-lines | 1193 / 0 | 56 / 0 | 29234 / 29238 | 11165 / 11168 | 0 / 0 |
| sgr-churn | 2810 / 1617 | 173 / 0 | 467143 / 490230 | 13381 / 13383 | 0 / 0 |
| unicode-mix | 18075 / 16882 | 1212 / 50 | 49829 / 49832 | 10319 / 10320 | 0 / 0 |
| tui-redraw | 0 / 0 | 1 / 0 | 112 / 77 | 5 / 0 | 1 / 0 |
| scroll-region | 119 / 0 | 0 / 0 | 48625 / 48635 | 21633 / 21633 | 0 / 0 |
| osc8-links | 71995 / 70802 | 13806 / 13560 | 285969 / 285972 | 8306 / 8308 | 18912 / 18908 |
| payloads | 687 / 566 | 1673 / 1641 | 6356 / 6374 | 160 / 162 | 483 / 483 |
| agent-session | 4324 / 3131 | 1421 / 1210 | 35370 / 36211 | 4298 / 4300 | 1301 / 1301 |
