# Conformance matrix

758 cases (655 from published text, 103 conventions) run on 5 engines. A cell is passes/cases asked; n/a is a case the adapter cannot ask (no replies, no hyperlinks, ...).

- alacritty: alacritty_terminal 0.26.0
- wormhole: wormhole
- wezterm: wezterm-term @e2a98e1b (WezTerm's terminal model)
- vt100: vt100 0.16.2
- ghostty: libghostty-vt @ad97d331 (Ghostty's terminal)

## Passes per area

| area               | cases | alacritty | wormhole | wezterm | vt100              | ghostty |
|--------------------|-------|-----------|----------|---------|--------------------|---------|
| altscreen          | 17    | 13/17     | 17/17    | 16/17   | 11/17              | 16/17   |
| charsets           | 44    | 13/44     | 44/44    | 16/44   | 5/41 (+3 n/a)      | 22/44   |
| checksum           | 17    | 2/17      | 17/17    | 2/17    | 0/0 (+17 n/a)      | 2/17    |
| conformance-levels | 14    | 0/14      | 14/14    | 2/14    | 0/4 (+10 n/a)      | 1/14    |
| cursor             | 56    | 45/56     | 56/56    | 53/56   | 41/56              | 55/56   |
| edit               | 33    | 22/33     | 33/33    | 23/33   | 17/33              | 27/33   |
| erase              | 31    | 18/31     | 31/31    | 19/31   | 17/31              | 30/31   |
| graphics           | 11    | 5/11      | 11/11    | 5/11    | 4/8 (+3 n/a)       | 10/11   |
| margins            | 31    | 5/31      | 31/31    | 25/31   | 5/31               | 23/31   |
| modes              | 13    | 9/13      | 13/13    | 9/13    | 6/6 (+7 n/a)       | 10/13   |
| origin             | 12    | 9/12      | 12/12    | 12/12   | 8/11 (+1 n/a)      | 12/12   |
| osc                | 29    | 26/29     | 29/29    | 20/29   | 8/11 (+18 n/a)     | 22/29   |
| parser             | 27    | 25/27     | 27/27    | 24/27   | 25/27              | 26/27   |
| rect-attributes    | 45    | 9/45      | 45/45    | 8/45    | 10/34 (+11 n/a)    | 8/45    |
| rect               | 56    | 15/56     | 56/56    | 15/56   | 15/55 (+1 n/a)     | 15/56   |
| reports            | 70    | 30/70     | 70/70    | 33/70   | 1/1 (+69 n/a)      | 47/70   |
| reset              | 22    | 14/22     | 22/22    | 19/22   | 14/20 (+2 n/a)     | 22/22   |
| resize             | 12    | 11/12     | 12/12    | 11/12   | 4/12               | 11/12   |
| save-restore       | 16    | 14/16     | 16/16    | 15/16   | 9/14 (+2 n/a)      | 14/16   |
| scroll             | 28    | 28/28     | 28/28    | 28/28   | 21/28              | 27/28   |
| sgr                | 65    | 62/65     | 65/65    | 65/65   | 42/46 (+19 n/a)    | 65/65   |
| tabs               | 23    | 21/23     | 23/23    | 22/23   | 10/23              | 23/23   |
| unicode            | 52    | 29/52     | 52/52    | 20/52   | 26/50 (+2 n/a)     | 46/52   |
| window             | 12    | 3/12      | 12/12    | 3/12    | 0/0 (+12 n/a)      | 3/12    |
| wrap               | 22    | 19/22     | 22/22    | 19/22   | 14/22              | 21/22   |
| **all**            | 758   | 447/758   | 758/758  | 484/758 | 313/581 (+177 n/a) | 558/758 |
| **spec**           | 655   | 378/655   | 655/655  | 411/655 | 267/491 (+164 n/a) | 470/655 |
| **convention**     | 103   | 69/103    | 103/103  | 73/103  | 46/90 (+13 n/a)    | 88/103  |

## Our conventions against the other engines

Of 103 convention cases (behaviour no published text spells out): 41 pass on every engine that can be asked, 20 fail on most engines (a convention the others do not follow: wrong or unusual, check it), the 62 below fail on at least one.

| case             |   | name                                                                         | alacritty | wormhole | wezterm | vt100 | ghostty |
|------------------|---|------------------------------------------------------------------------------|-----------|----------|---------|-------|---------|
| charsets:147     | C | the character set applies to repeated characters                             | ✓         | ✓        | ✓       | ✗     | ✓       |
| cursor:394       | C | a backspace in the delayed wrap with reverse wraparound only ends the wait   | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| edit:190         | C | REP wraps like typed text                                                    | ✓         | ✓        | ✓       | ✗     | ✓       |
| edit:223         | C | DL does not feed the history                                                 | ✗ (known) | ✓        | ✗       | ✓     | ✓       |
| edit:235         | C | ECH ends a pending wrap                                                      | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| edit:244         | C | ICH ends a pending wrap                                                      | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| edit:252         | C | DCH ends a pending wrap                                                      | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| edit:260         | C | SL shifts the columns left and blanks come in at the right                   | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| edit:270         | C | SR shifts the columns right and blanks come in at the left                   | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| edit:279         | C | SL and SR stay between the margins                                           | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| edit:293         | C | SL erases with the background color                                          | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| erase:72         | C | ED erased cells keep only the background of the current rendition            | ✓         | ✓        | ✗       | ✗     | ✓       |
| erase:220        | C | EL ends a pending wrap                                                       | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| erase:229        | C | ED ends a pending wrap                                                       | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| erase:237        | C | ED 2 keeps the text it clears as history                                     | ✓         | ✓        | ✗       | ✗     | ✗       |
| erase:247        | C | ED 2 on a blank screen adds nothing to the history                           | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| erase:254        | C | ED 3 before ED 2 leaves no history                                           | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:59       | C | wrapping goes from the right margin to the left margin                       | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:109      | C | IL moves only the columns between the margins                                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:120      | C | DL moves only the columns between the margins                                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:131      | C | line feed at the bottom margin scrolls only the margin columns               | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:142      | C | SU moves only the columns between the margins                                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| modes:88         | C | mode 41 makes a tab at the end of a row wrap first                           | ✗ (known) | ✓        | ✗       | –     | ✗       |
| osc:160          | C | an empty title is no title                                                   | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| reports:464      | C | DECRQLP reports the locator once DECELR enabled it                           | ✗ (known) | ✓        | ✗       | –     | ✗       |
| resize:7         | C | widening joins a wrapped row                                                 | ✓         | ✓        | ✓       | ✗     | ✓       |
| resize:17        | C | narrowing keeps the text of a long row                                       | ✓         | ✓        | ✓       | ✗     | ✓       |
| resize:26        | C | narrowing keeps the rows on screen when they fit                             | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| resize:54        | C | narrowing and widening again restores the line                               | ✓         | ✓        | ✓       | ✗     | ✓       |
| resize:74        | C | shrinking the height keeps the cursor row on screen                          | ✓         | ✓        | ✓       | ✗     | ✓       |
| resize:87        | C | the cursor stays inside after narrowing                                      | ✓         | ✓        | ✗       | ✓     | ✗       |
| resize:94        | C | resizing keeps the rendition of the text                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| resize:111       | C | resizing resets the scrolling region                                         | ✓         | ✓        | ✓       | ✗     | ✓       |
| resize:119       | C | wide characters are not split by narrowing                                   | ✓         | ✓        | ✓       | ✗     | ✓       |
| save-restore:103 | C | DECRC restores the hyperlink                                                 | ✓         | ✓        | ✓       | –     | ✗       |
| save-restore:109 | C | DECRC brings back the state of no hyperlink                                  | ✓         | ✓        | ✓       | –     | ✗       |
| scroll:68        | C | line feed in a region that starts at the top feeds the history               | ✓         | ✓        | ✓       | ✗     | ✓       |
| scroll:124       | C | reverse index above the region moves nowhere                                 | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:101         | C | CHT stops at the last column                                                 | ✓         | ✓        | ✓       | ✗     | ✓       |
| unicode:61       | C | a combining mark joins the cell before it                                    | ✓         | ✓        | ✗       | ✓     | ✓       |
| unicode:71       | C | several combining marks join one cell                                        | ✓         | ✓        | ✗       | ✓     | ✓       |
| unicode:78       | C | a combining mark joins a wide character                                      | ✓         | ✓        | ✗       | ✓     | ✓       |
| unicode:125      | C | erasing to the right from the second half removes the whole character        | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| unicode:133      | C | erasing characters over half of a wide character removes the whole character | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| unicode:203      | C | grapheme clusters: VS15 makes a default-emoji character narrow               | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:212      | C | grapheme clusters: VS15 gives the second cell of a wide emoji back           | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:222      | C | grapheme clusters: VS15 at the right margin steps the cursor back            | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:235      | C | grapheme clusters: VS15 does not change the width of a text symbol           | ✓         | ✓        | ✗       | ✓     | ✓       |
| unicode:242      | C | grapheme clusters: VS15 then VS16 is not a sequence                          | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| unicode:327      | C | without the mode each spacing mark takes a cell of its own                   | ✓         | ✓        | ✗       | ✓     | ✓       |
| unicode:416      | C | a character completed by a later feed, one character, then half of another   | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| unicode:426      | C | a combining mark with nothing before it is kept on a blank                   | ✓         | ✓        | ✗       | ✗     | ✗       |
| window:79        | C | the sizes in cells take effect at once when allowed                          | ✗ (known) | ✓        | ✗       | –     | ✗       |
| window:97        | C | a size past what the screen holds is cut                                     | ✗ (known) | ✓        | ✗       | –     | ✗       |
| wrap:26          | C | a line feed clears the pending wrap                                          | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| wrap:52          | C | cursor back clears the pending wrap                                          | ✓         | ✓        | ✓       | ✗     | ✓       |
| wrap:60          | C | cursor up clears the pending wrap                                            | ✓         | ✓        | ✓       | ✗     | ✓       |
| wrap:110         | C | a line feed in the pending state on the last row scrolls once                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| wrap:142         | C | a wide character that does not fit wraps as a whole                          | ✓         | ✓        | ✗       | ✗     | ✓       |
| wrap:155         | C | a wide character that fits exactly leaves the cursor waiting                 | ✓         | ✓        | ✗       | ✓     | ✓       |
| wrap:183         | C | a tab at the last column leaves a pending wrap                               | ✓         | ✓        | ✓       | ✗     | ✓       |
| wrap:193         | C | a tab with 'more fix' on wraps first                                         | ✗ (known) | ✓        | ✗       | ✗     | ✗       |

## Cases from published text that most engines fail (213)

| case                   |   | name                                                                                        | alacritty | wormhole | wezterm | vt100 | ghostty |
|------------------------|---|---------------------------------------------------------------------------------------------|-----------|----------|---------|-------|---------|
| charsets:84            | S | locking shift 2 invokes G2                                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| charsets:92            | S | locking shift 3 invokes G3                                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| charsets:100           | S | single shift 2 affects the next character only                                              | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| charsets:108           | S | single shift 3 affects the next character only                                              | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| charsets:116           | S | a single shift does not change the locking shift                                            | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| charsets:164           | S | DECNRCM makes the national sets available                                                   | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:172           | S | the Dutch set replaces twelve positions of ASCII                                            | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:180           | S | the Finnish set replaces twelve positions of ASCII                                          | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:188           | S | the French set replaces twelve positions of ASCII                                           | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:196           | S | the French Canadian set replaces twelve positions of ASCII                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:204           | S | the German set replaces twelve positions of ASCII                                           | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:212           | S | the Italian set replaces twelve positions of ASCII                                          | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:220           | S | the Norwegian and Danish set replaces twelve positions of ASCII                             | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:228           | S | the Portuguese set replaces twelve positions of ASCII                                       | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:236           | S | the Spanish set replaces twelve positions of ASCII                                          | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:244           | S | the Swedish set replaces twelve positions of ASCII                                          | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:252           | S | the Swiss set replaces twelve positions of ASCII                                            | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:260           | S | the alternate finals of the national sets                                                   | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:268           | S | a national set in G1 comes with SO                                                          | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:276           | S | a national set is not designated by a terminal that does not have it                        | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:287           | S | DECSTR returns to the multinational set                                                     | ✗ (known) | ✓        | ✗       | –     | ✗       |
| charsets:296           | S | DEC Technical maps the mathematical symbols                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:304           | S | DEC Technical leaves its unused positions blank                                             | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:312           | S | DEC Supplemental is the right half of the Multinational set                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:320           | S | the Latin-1 supplement is a 96-character set                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| charsets:328           | S | a single shift takes one character from G2                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| charsets:336           | S | the user-preferred supplemental set starts as DEC Supplemental                              | ✗ (known) | ✓        | ✗       | –     | ✗       |
| charsets:345           | S | DECAUPSS assigns ISO Latin-1 as the user-preferred set                                      | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:15            | S | DECRQCRA of one cell is its code point                                                      | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:23            | S | DECRQCRA of a rectangle is the sum of its cells                                             | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:31            | S | DECRQCRA counts a blank cell as a space                                                     | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:39            | S | DECRQCRA counts an erased cell as a space                                                   | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:47            | S | DECRQCRA without an area sums the page                                                      | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:55            | S | DECRQCRA without parameters sums the page and answers for request 0                         | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:63            | S | DECRQCRA echoes its request number                                                          | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:71            | S | DECRQCRA is cut to the screen                                                               | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:79            | S | DECRQCRA of an upside-down area sums nothing                                                | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:87            | S | DECRQCRA counts from the region in origin mode                                              | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:95            | S | DECRQCRA counts a wide character once                                                       | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:103           | S | DECRQCRA ignores combining marks                                                            | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:111           | S | DECRQCRA ignores the video attributes                                                       | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:119           | S | DECRQCRA sums modulo 65536                                                                  | ✗ (known) | ✓        | ✗       | –     | ✗       |
| checksum:135           | S | DECRQCRA of the first page is answered                                                      | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:4   | S | DA1 names the VT500 family and its features by default                                      | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:11  | S | DECSCL 62 takes the terminal down to VT200                                                  | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:18  | S | DECSCL 61 is VT100 mode                                                                     | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:25  | S | DECRQM is a VT300 feature                                                                   | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:34  | S | left and right margins are a VT400 feature                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| conformance-levels:47  | S | DECSCL is a hard reset                                                                      | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:58  | S | DECRQSS reports the level and the control mode                                              | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:67  | S | DECSCL without its second parameter chooses 8-bit controls                                  | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:80  | S | S8C1T makes the replies use one-byte C1 controls                                            | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:89  | S | C1 bytes are controls only in 8-bit mode                                                    | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| conformance-levels:100 | S | 8-bit IND, NEL, RI, HTS, DECID, SPA and EPA                                                 | ✗ (known) | ✓        | ✗       | –     | ✗       |
| conformance-levels:115 | S | an 8-bit ST ends a string                                                                   | ✗ (known) | ✓        | ✓       | ✗     | ✗       |
| conformance-levels:131 | S | DA1 lists what each family has                                                              | ✗ (known) | ✓        | ✗       | –     | ✗       |
| cursor:357             | S | BS and CUB go back to the line above with reverse wraparound (45) only if it wrapped        | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| cursor:368             | S | reverse wraparound (1045) goes back over a line that did not wrap and round the top         | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| cursor:379             | S | reverse wraparound goes to the right margin                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| edit:199               | S | DECIC inserts columns                                                                       | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| edit:211               | S | DECDC deletes columns                                                                       | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| erase:156              | S | DECSCA protection keeps a character from DECSEL                                             | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| erase:164              | S | DECSEL 0 erases unprotected characters to the right                                         | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| erase:172              | S | DECSEL 1 erases unprotected characters to the left                                          | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| erase:180              | S | DECSED 2 erases every unprotected character on the screen                                   | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| erase:188              | S | DECSED 0 erases unprotected characters below the cursor                                     | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| erase:211              | S | DECSCA 2 ends protection                                                                    | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| erase:263              | S | ED, EL and ECH leave a guarded area alone                                                   | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| erase:277              | S | DECSCA protection is not a guarded area                                                     | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| graphics:6             | S | a kitty image leaves the cursor right of its last row                                       | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| graphics:38            | S | a kitty image at the bottom scrolls the screen to make room                                 | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| graphics:53            | S | a kitty image wider than the rest of the line is cut and the cursor waits at the edge       | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| graphics:93            | S | an iTerm2 inline image leaves the cursor right of its last row                              | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| margins:25             | S | the left margin stops cursor backward                                                       | ✗ (known) | ✓        | ✓       | ✗     | ✗       |
| margins:32             | S | backspace at the left margin does nothing                                                   | ✗ (known) | ✓        | ✓       | ✗     | ✗       |
| margins:160            | S | DECIC inserts columns inside the margins                                                    | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| margins:171            | S | DECDC deletes columns inside the margins                                                    | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| margins:182            | S | forward index moves the cursor right                                                        | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| margins:189            | S | forward index at the right margin scrolls the margins left                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| margins:199            | S | back index moves the cursor left                                                            | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| margins:206            | S | back index at the left margin scrolls the margins right                                     | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| modes:70               | S | the VT510 modes a terminal emulator has no hardware for are kept and answered               | ✗ (known) | ✓        | ✗       | –     | ✗       |
| modes:81               | S | DECHCCM and the ECMA-48 modes without a terminal to apply to are permanently reset          | ✗ (known) | ✓        | ✗       | –     | ✗       |
| osc:48                 | S | a title can be pushed and popped                                                            | ✓         | ✓        | ✗       | ✗     | ✗       |
| osc:56                 | S | the title stack holds several titles                                                        | ✓         | ✓        | ✗       | ✗     | ✗       |
| osc:64                 | S | a window title push and pop works with Ps = 2                                               | ✓         | ✓        | ✗       | ✗     | ✗       |
| osc:181                | S | the special colors are OSC 4 from 256 and OSC 5                                             | ✗ (known) | ✓        | ✗       | –     | ✗       |
| osc:194                | S | OSC 10 sets the colors after it too, and OSC 110 to 119 reset them one by one               | ✗ (known) | ✓        | ✗       | –     | ✗       |
| parser:13              | S | CAN cancels a string                                                                        | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| parser:183             | S | a sixel image leaves the cursor under it                                                    | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:6      | S | DECCARA in rectangle extent changes the rectangle                                           | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:24     | S | DECCARA in stream extent changes the stream                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:37     | S | DECSACE 1 is the stream too                                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:46     | S | DECSACE 2 and back to the stream                                                            | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:53     | S | DECSACE with another value changes nothing                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:61     | S | DECCARA in stream extent on one line                                                        | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:84     | S | DECCARA 0 clears bold, underline, blink and negative image                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:92     | S | DECCARA without attributes is the same as 0                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:99     | S | DECCARA 22 clears bold                                                                      | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:105    | S | DECCARA 24 clears underline                                                                 | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:116    | S | DECCARA 27 clears inverse                                                                   | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:122    | S | DECCARA 1 sets bold                                                                         | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:130    | S | DECCARA 4 sets underline                                                                    | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:138    | S | DECCARA 5 sets blink                                                                        | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:146    | S | DECCARA 7 sets inverse                                                                      | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:154    | S | DECCARA 8 sets hidden                                                                       | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:162    | S | DECCARA 28 clears hidden                                                                    | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:169    | S | DECCARA applies the attributes in order                                                     | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:176    | S | DECCARA ignores an attribute it does not know                                               | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:183    | S | DECCARA replaces another underline style                                                    | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:190    | S | DECCARA changes blank cells in rectangle extent                                             | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:215    | S | DECCARA counts from the region in origin mode                                               | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:225    | S | DECCARA ignores the margins                                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:237    | S | DECCARA is cut to the screen                                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:244    | S | DECRARA reverses bold                                                                       | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:254    | S | DECRARA 4 reverses underline                                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:261    | S | DECRARA 5 reverses blink                                                                    | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:268    | S | DECRARA 7 reverses inverse                                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:275    | S | DECRARA 8 reverses hidden                                                                   | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:282    | S | DECRARA 0 reverses bold, underline, blink and negative image                                | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:290    | S | DECRARA without attributes is the same as 0                                                 | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect-attributes:301    | S | DECRARA reverses an underline style to none                                                 | ✗ (known) | ✓        | ✗       | ✓     | ✗       |
| rect-attributes:312    | S | DECRARA in stream extent reverses the stream                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:323    | S | DECRARA in rectangle extent reverses the rectangle                                          | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:333    | S | RIS selects the stream extent again                                                         | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect-attributes:340    | S | DECSTR leaves the extent as DECSACE set it                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:8                 | S | DECERA acts on the rectangle                                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:22                | S | DECERA without an area takes the whole page                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:62                | S | DECERA is cut to the screen                                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:76                | S | DECERA counts from the region in origin mode                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:90                | S | DECERA ignores the margins                                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:104               | S | DECERA leaves the cursor where it is                                                        | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:119               | S | DECERA also erases protected characters                                                     | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:133               | S | DECERA gives the cells the current background                                               | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:141               | S | DECERA through half a wide character blanks both halves                                     | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:147               | S | DECSERA acts on the rectangle                                                               | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:161               | S | DECSERA without an area takes the whole page                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:201               | S | DECSERA is cut to the screen                                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:215               | S | DECSERA counts from the region in origin mode                                               | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:229               | S | DECSERA ignores the margins                                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:243               | S | DECSERA leaves the cursor where it is                                                       | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:258               | S | DECSERA leaves protected characters                                                         | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:265               | S | DECSERA without an area erases the unprotected characters of the page                       | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:272               | S | DECFRA acts on the rectangle                                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:286               | S | DECFRA without an area takes the whole page                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:326               | S | DECFRA is cut to the screen                                                                 | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:340               | S | DECFRA counts from the region in origin mode                                                | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:354               | S | DECFRA ignores the margins                                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:368               | S | DECFRA leaves the cursor where it is                                                        | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:383               | S | DECFRA fills with the current rendition                                                     | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:391               | S | DECFRA takes a character from the right-hand half                                           | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:449               | S | DECFRA over half a wide character blanks the other half                                     | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:455               | S | DECCRA copies a rectangle                                                                   | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:469               | S | DECCRA copies as if the source were read whole first when the areas overlap downwards       | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:483               | S | DECCRA copies as if the source were read whole first when the areas overlap upwards         | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:497               | S | DECCRA copies within a row to the right                                                     | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:511               | S | DECCRA copies within a row to the left                                                      | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:525               | S | DECCRA is cut at the screen's edge                                                          | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:539               | S | DECCRA with the default source takes it from the corner                                     | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:553               | S | DECCRA with the default target copies to the corner                                         | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:580               | S | DECCRA with a source beyond the screen is cut to it                                         | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:594               | S | DECCRA counts both corners from the region in origin mode                                   | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:608               | S | DECCRA ignores the margins                                                                  | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:653               | S | DECCRA copies the rendition and the link                                                    | ✗ (known) | ✓        | ✗       | –     | ✗       |
| rect:666               | S | DECCRA copies a whole wide character                                                        | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:674               | S | DECCRA onto the second half of a wide character blanks the first                            | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| rect:680               | S | DECCRA onto the first half of a wide character blanks the second                            | ✗ (known) | ✓        | ✗       | ✗     | ✗       |
| reports:60             | S | DSR ? 15 reports the printer status                                                         | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:67             | S | DSR ? 25 reports the user defined key status                                                | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:221            | S | DECRQM of grapheme clustering                                                               | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:265            | S | DECRQSS reports the character protection                                                    | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:328            | S | DECRQSS reports the settings that are only kept                                             | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:336            | S | DECDSR 56, 62, 63 and 85 answer that the locator is a mouse, no macro space and no sessions | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:343            | S | DECDSR 53 and 55 answer that a locator is available                                         | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:350            | S | DECXCPR and the keyboard report grow with the conformance level                             | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:361            | S | DECRQPSR 1 reports the cursor information                                                   | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:368            | S | DECCIR reports the position, the rendition and the protection                               | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:375            | S | DECCIR reports origin mode, a pending wrap and a single shift                               | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:382            | S | DECCIR reports the character sets, their sizes and what is invoked                          | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:389            | S | DECRSPS puts the cursor information back                                                    | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:406            | S | DECRQPSR 2 reports the tab stops                                                            | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:415            | S | DECRSPS puts the tab stops back                                                             | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:426            | S | DECRQTSR 1 reports the state                                                                | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:433            | S | DECRSTS puts the state back                                                                 | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:450            | S | DECRQDE reports the displayed extent                                                        | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:457            | S | DECRQLP says there is no locator until DECELR                                               | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:471            | S | DECELR 2 enables the locator for one report                                                 | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:478            | S | DECELR 0 disables the locator                                                               | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reports:485            | S | the locator replaces mouse tracking                                                         | ✗ (known) | ✓        | ✗       | –     | ✗       |
| reset:122              | S | DECSTR shows the cursor                                                                     | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:141            | S | grapheme clusters: a ZWJ emoji sequence is one cluster two cells wide                       | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:151            | S | grapheme clusters: a skin tone modifier stays with its emoji                                | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:160            | S | grapheme clusters: two regional indicators are one flag                                     | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:177            | S | grapheme clusters: VS16 makes a text symbol wide                                            | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:186            | S | grapheme clusters: VS16 at the right margin moves the cluster to the next row               | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:250            | S | grapheme clusters: a spacing mark takes a cell                                              | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:259            | S | grapheme clusters: several spacing marks and a nonspacing mark stay two cells               | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:274            | S | grapheme clusters: a conjunct takes two cells                                               | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:282            | S | grapheme clusters: a longer conjunct with a vowel sign stays two cells                      | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:304            | S | grapheme clusters: Khmer coeng makes a conjunct                                             | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:312            | S | grapheme clusters: an Arabic number sign has a cell                                         | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:350            | S | grapheme clusters: a soft reset returns the mode to its default                             | ✗ (known) | ✓        | ✗       | –     | ✗       |
| unicode:357            | S | grapheme clusters: a full reset returns the mode to its default                             | ✗ (known) | ✓        | ✗       | –     | ✗       |
| unicode:364            | S | a lone continuation byte is one replacement character                                       | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:393            | S | an overlong encoding is a replacement character per byte                                    | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| unicode:401            | S | an encoded surrogate is a replacement character per byte                                    | ✗ (known) | ✓        | ✗       | ✗     | ✓       |
| window:4               | S | XTWINOPS reports the text area, the cell and the window state                               | ✗ (known) | ✓        | ✗       | –     | ✗       |
| window:19              | S | the title and the icon name are reported when the host allows it                            | ✗ (known) | ✓        | ✗       | –     | ✗       |
| window:29              | S | controls are taken out of a reported title                                                  | ✗ (known) | ✓        | ✗       | –     | ✗       |
| window:37              | S | the title stack keeps the title and the icon name together or apart                         | ✗ (known) | ✓        | ✗       | –     | ✗       |
| window:47              | S | a pop takes the slot that holds what it asks for                                            | ✗ (known) | ✓        | ✗       | –     | ✗       |
| window:55              | S | the title modes read and report in hexadecimal                                              | ✗ (known) | ✓        | ✗       | –     | ✗       |
| window:65              | S | a full reset clears the title modes and the stack                                           | ✗ (known) | ✓        | ✗       | –     | ✗       |

## Cases from published text that some engines fail and most pass (145)

`S` is a case from published text, `C` a convention; `–` is n/a.

| case                   |   | name                                                                                 | alacritty | wormhole | wezterm | vt100 | ghostty |
|------------------------|---|--------------------------------------------------------------------------------------|-----------|----------|---------|-------|---------|
| altscreen:25           | S | mode 1049 leaves the cursor where it is                                              | ✓         | ✓        | ✗       | ✗     | ✓       |
| altscreen:71           | S | mode 1047 clears the alternate screen when leaving it                                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| altscreen:86           | S | mode 1047 brings back the normal screen                                              | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| altscreen:95           | S | mode 1048 saves and restores the cursor                                              | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| altscreen:103          | S | mode 47 uses the alternate screen                                                    | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| altscreen:131          | S | mode 1049 again on the alternate screen changes nothing                              | ✓         | ✓        | ✓       | ✗     | ✗       |
| charsets:4             | S | DEC special graphics draws a box                                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| charsets:13            | S | DEC special graphics maps the line drawing characters                                | ✓         | ✓        | ✓       | ✗     | ✓       |
| charsets:20            | S | DEC special graphics maps the scan lines                                             | ✓         | ✓        | ✓       | ✗     | ✓       |
| charsets:27            | S | DEC special graphics maps the symbols                                                | ✓         | ✓        | ✓       | ✗     | ✓       |
| charsets:34            | S | DEC special graphics maps the diamond                                                | ✓         | ✓        | ✓       | ✗     | ✓       |
| charsets:41            | S | DEC special graphics maps the control symbols                                        | ✓         | ✓        | ✓       | ✗     | ✓       |
| charsets:55            | S | designating ASCII again stops the mapping                                            | ✓         | ✓        | ✓       | ✗     | ✓       |
| charsets:62            | S | the UK set shows the pound sign for number sign                                      | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| charsets:70            | S | shift out selects G1 and shift in selects G0                                         | ✓         | ✓        | ✓       | ✗     | ✓       |
| charsets:77            | S | G1 can be designated while G0 is in use                                              | ✓         | ✓        | ✓       | ✗     | ✓       |
| charsets:131           | S | a soft reset selects the default character sets                                      | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| charsets:139           | S | a soft reset also unlocks a shifted G1                                               | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| conformance-levels:122 | S | an 8-bit CSI breaks off a sequence                                                   | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| cursor:46              | S | HVP is CUP                                                                           | ✓         | ✓        | ✓       | ✗     | ✓       |
| cursor:171             | S | HPA moves to a column                                                                | ✓         | ✓        | ✓       | ✗     | ✓       |
| cursor:177             | S | HPR moves right                                                                      | ✓         | ✓        | ✓       | ✗     | ✓       |
| cursor:195             | S | VPR moves down                                                                       | ✓         | ✓        | ✓       | ✗     | ✓       |
| cursor:201             | S | CUU stops at the top margin when the cursor is inside the region                     | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| cursor:208             | S | CUD stops at the bottom margin when the cursor is inside the region                  | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| cursor:221             | S | CUD above the region still stops at the bottom margin                                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| cursor:272             | S | new line mode makes line feed also return the carriage                               | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| cursor:282             | S | NEL moves to the start of the next line                                              | ✓         | ✓        | ✓       | ✗     | ✓       |
| cursor:290             | S | IND moves down and keeps the column                                                  | ✓         | ✓        | ✓       | ✗     | ✓       |
| cursor:316             | S | NUL and DEL are ignored                                                              | ✓         | ✓        | ✓       | ✓     | ✗       |
| cursor:332             | S | IND at the bottom margin does not scroll a cursor outside the left and right margins | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| cursor:343             | S | RI at the top margin does not scroll a cursor outside the left and right margins     | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| cursor:351             | S | CBT goes past the left margin                                                        | ✓         | ✓        | ✓       | ✗     | ✓       |
| cursor:406             | S | CUB with reverse wraparound walks back like backspace                                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| edit:40                | S | DCH with a huge count deletes to the end of the row                                  | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| edit:133               | S | IL below the region does nothing                                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| edit:143               | S | DL above the region does nothing                                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| edit:153               | S | insert mode pushes characters right                                                  | ✓         | ✓        | ✓       | ✗     | ✓       |
| edit:161               | S | insert mode loses what is pushed off the row                                         | ✓         | ✓        | ✓       | ✗     | ✓       |
| edit:175               | S | REP repeats the last character                                                       | ✓         | ✓        | ✓       | ✗     | ✓       |
| edit:183               | S | REP defaults to one repeat                                                           | ✓         | ✓        | ✓       | ✗     | ✓       |
| erase:17               | S | ED 1 erases from the start of the screen to the cursor inclusive                     | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| erase:40               | S | ED 3 erases the saved lines and keeps the screen                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| graphics:73            | S | a kitty query is answered with the id                                                | ✗ (known) | ✓        | ✗       | –     | ✓       |
| graphics:86            | S | an answer about an image keeps its place among the other replies                     | ✗ (known) | ✓        | ✗       | –     | ✓       |
| margins:5              | S | DECSLRM sets the margins and homes the cursor                                        | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:18             | S | the right margin stops cursor forward                                                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:39             | S | carriage return goes to the left margin                                              | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:46             | S | a tab stops at the right margin                                                      | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:68             | S | ICH shifts only up to the right margin                                               | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:77             | S | DCH shifts only up to the right margin                                               | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:85             | S | ICH outside the margins does nothing                                                 | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:93             | S | DCH outside the margins does nothing                                                 | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:101            | S | insert mode shifts only up to the right margin                                       | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:228            | S | DECSLRM needs the left margin below the right                                        | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:235            | S | DECSLRM needs at least two columns                                                   | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:242            | S | origin mode with margins starts at the margins                                       | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| margins:249            | S | origin mode with margins clamps to them                                              | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| modes:57               | S | mode 5 reverse video can be asked for                                                | ✗ (known) | ✓        | ✗       | –     | ✓       |
| origin:9               | S | resetting origin mode moves the cursor to the home of the screen                     | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| origin:25              | S | HVP counts from the top of the region                                                | ✓         | ✓        | ✓       | ✗     | ✓       |
| origin:40              | S | VPA counts from the top of the region                                                | ✓         | ✓        | ✓       | ✗     | ✓       |
| origin:46              | S | CPR is relative to the region                                                        | ✗ (known) | ✓        | ✓       | –     | ✓       |
| origin:78              | S | a soft reset ends origin mode                                                        | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| osc:23                 | S | OSC 1 changes the icon name, not the window title                                    | ✓         | ✓        | ✗       | ✓     | ✓       |
| osc:167                | S | the colors a program sets are kept until it resets them, and a query answers them    | ✓         | ✓        | ✗       | –     | ✗       |
| osc:175                | S | a spec with # is the top bits of the byte and rgb: is scaled                         | ✓         | ✓        | ✗       | –     | ✗       |
| osc:203                | S | a full reset puts the colors back                                                    | ✓         | ✓        | ✗       | –     | ✓       |
| parser:106             | S | a huge parameter is clamped                                                          | ✓         | ✓        | ✗       | ✓     | ✓       |
| rect-attributes:111    | S | DECCARA 25 clears blink                                                              | ✓         | ✓        | ✗       | ✓     | ✗       |
| reports:26             | S | DSR 6 is relative to the margins in origin mode                                      | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:53             | S | DECXCPR reports the cursor position                                                  | ✗ (known) | ✓        | ✗       | –     | ✓       |
| reports:98             | S | DECID answers like primary device attributes                                         | ✓         | ✓        | ✗       | –     | ✓       |
| reports:104            | S | secondary device attributes                                                          | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:111            | S | secondary device attributes with an explicit 0                                       | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:118            | S | tertiary device attributes report a unit id                                          | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:125            | S | XTVERSION reports the terminal's name                                                | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:132            | S | XTVERSION with an explicit 0                                                         | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:151            | S | the cell size in pixels                                                              | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:201            | S | DECRQM of the alternate screen                                                       | ✓         | ✓        | ✗       | –     | ✓       |
| reports:207            | S | DECRQM of left and right margin mode                                                 | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:214            | S | DECRQM of synchronized output                                                        | ✗ (known) | ✓        | ✗       | –     | ✓       |
| reports:228            | S | DECRQSS reports the default SGR                                                      | ✗ (known) | ✓        | ✗       | –     | ✓       |
| reports:235            | S | DECRQSS reports the current SGR                                                      | ✗ (known) | ✓        | ✗       | –     | ✓       |
| reports:242            | S | DECRQSS reports the SGR colors                                                       | ✗ (known) | ✓        | ✗       | –     | ✓       |
| reports:249            | S | DECRQSS reports the scrolling region                                                 | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:258            | S | DECRQSS reports the cursor style                                                     | ✗ (known) | ✓        | ✗       | –     | ✓       |
| reports:272            | S | DECRQSS reports the left and right margins                                           | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:279            | S | DECRQSS answers an invalid request with 0                                            | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reports:286            | S | the foreground color can be asked for                                                | ✓         | ✓        | ✗       | –     | ✓       |
| reports:298            | S | the cursor color can be asked for                                                    | ✓         | ✓        | ✗       | –     | ✓       |
| reports:304            | S | a palette color can be asked for                                                     | ✓         | ✓        | ✗       | –     | ✓       |
| reports:310            | S | a palette color that was set is reported back                                        | ✓         | ✓        | ✗       | –     | ✓       |
| reports:316            | S | the foreground color that was set is reported back                                   | ✓         | ✓        | ✗       | –     | ✓       |
| reports:322            | S | several palette colors can be asked for at once                                      | ✓         | ✓        | ✗       | –     | ✓       |
| reset:74               | S | RIS forgets the saved cursor                                                         | ✓         | ✓        | ✗       | ✓     | ✓       |
| reset:95               | S | DECSTR returns the rendition to normal                                               | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| reset:102              | S | DECSTR ends origin mode and removes the margins                                      | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| reset:114              | S | DECSTR ends insert mode                                                              | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| reset:129              | S | DECSTR ends selective erase protection                                               | ✗ (known) | ✓        | ✗       | ✓     | ✓       |
| reset:137              | S | DECSTR resets modes seen by DECRQM                                                   | ✗ (known) | ✓        | ✓       | –     | ✓       |
| reset:144              | S | DECALN fills the screen with E                                                       | ✓         | ✓        | ✓       | ✗     | ✓       |
| reset:154              | S | DECALN homes the cursor                                                              | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| reset:161              | S | DECALN removes the margins                                                           | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| save-restore:27        | S | DECRC restores the character set                                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| save-restore:34        | S | DECRC restores the origin mode                                                       | ✗ (known) | ✓        | ✓       | ✓     | ✓       |
| save-restore:43        | S | DECRC restores the selective erase attribute                                         | ✓         | ✓        | ✓       | ✗     | ✓       |
| save-restore:76        | S | SCOSC and SCORC save and restore the position                                        | ✓         | ✓        | ✓       | ✗     | ✓       |
| save-restore:82        | S | SCORC restores the rendition like DECRC                                              | ✓         | ✓        | ✓       | ✗     | ✓       |
| save-restore:88        | S | a soft reset forgets the saved cursor                                                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| save-restore:96        | S | a full reset forgets the saved cursor                                                | ✓         | ✓        | ✗       | ✓     | ✓       |
| scroll:31              | S | the history keeps at most the lines it is allowed                                    | ✓         | ✓        | ✓       | ✓     | ✗       |
| scroll:40              | S | DECSTBM moves the cursor home                                                        | ✓         | ✓        | ✓       | ✗     | ✓       |
| scroll:146             | S | index keeps the column and scrolls at the bottom margin                              | ✓         | ✓        | ✓       | ✗     | ✓       |
| scroll:157             | S | next line scrolls at the bottom margin and returns the carriage                      | ✓         | ✓        | ✓       | ✗     | ✓       |
| scroll:236             | S | an invalid region is ignored                                                         | ✓         | ✓        | ✓       | ✗     | ✓       |
| scroll:287             | S | an invalid DECSTBM leaves the cursor where it is                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| sgr:46                 | S | SGR 5 is blink                                                                       | ✗ (known) | ✓        | ✓       | –     | ✓       |
| sgr:71                 | S | SGR 21 is double underline                                                           | ✗ (known) | ✓        | ✓       | –     | ✓       |
| sgr:78                 | S | SGR 53 is overline                                                                   | ✗ (known) | ✓        | ✓       | –     | ✓       |
| sgr:157                | S | underline style 4:0 is none                                                          | ✓         | ✓        | ✓       | ✗     | ✓       |
| sgr:163                | S | underline style 4:1 is single                                                        | ✓         | ✓        | ✓       | ✗     | ✓       |
| sgr:299                | S | SGR 38:2 with a color space id sets a truecolor foreground                           | ✓         | ✓        | ✓       | ✗     | ✓       |
| sgr:305                | S | SGR 48:2 with a color space id sets a truecolor background                           | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:39                | S | HTS sets a tab stop at the cursor                                                    | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:46                | S | HTS leaves the other stops                                                           | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:53                | S | TBC 0 clears the stop at the cursor                                                  | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:60                | S | TBC without a parameter is TBC 0                                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:67                | S | TBC 3 clears all stops                                                               | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:81                | S | DECST8C restores the stops every eight columns                                       | ✗ (known) | ✓        | ✗       | ✓     | ✓       |
| tabs:89                | S | CHT moves forward over tab stops                                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:95                | S | CHT defaults to one stop                                                             | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:107               | S | CBT moves back to the previous stop                                                  | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:113               | S | CBT by several stops stays on the first column                                       | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:119               | S | CBT from a stop goes to the one before                                               | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:131               | S | a stop set by HTS is used by CBT                                                     | ✓         | ✓        | ✓       | ✗     | ✓       |
| tabs:152               | S | CBT with every stop cleared stops at the first column                                | ✗ (known) | ✓        | ✓       | ✗     | ✓       |
| unicode:169            | S | grapheme clusters: a combining mark does not move the cursor                         | ✓         | ✓        | ✗       | ✓     | ✓       |
| unicode:196            | S | grapheme clusters: VS15 leaves an emoji that has no text presentation alone          | ✓         | ✓        | ✗       | ✓     | ✗       |
| unicode:297            | S | grapheme clusters: a virama that is a spacing mark takes no cell                     | ✓         | ✓        | ✓       | ✓     | ✗       |
| unicode:372            | S | an impossible byte is one replacement character                                      | ✓         | ✓        | ✗       | ✗     | ✓       |
| unicode:379            | S | a truncated sequence is one replacement character                                    | ✓         | ✓        | ✗       | ✗     | ✓       |
| unicode:386            | S | a truncated four byte sequence is one replacement character                          | ✓         | ✓        | ✗       | ✗     | ✓       |
| unicode:409            | S | a replacement character takes one cell                                               | ✓         | ✓        | ✗       | ✗     | ✓       |
| wrap:69                | S | autowrap off keeps the cursor in the last column and overwrites it                   | ✓         | ✓        | ✓       | ✗     | ✓       |

## What each engine fails

### alacritty (311 cases)

- altscreen:71 mode 1047 clears the alternate screen when leaving it (known, known)
- altscreen:86 mode 1047 brings back the normal screen (known, known)
- altscreen:95 mode 1048 saves and restores the cursor (known, known)
- altscreen:103 mode 47 uses the alternate screen (known, known)
- charsets:62 the UK set shows the pound sign for number sign (known, known)
- charsets:84 locking shift 2 invokes G2 (known, known)
- charsets:92 locking shift 3 invokes G3 (known, known)
- charsets:100 single shift 2 affects the next character only (known, known)
- charsets:108 single shift 3 affects the next character only (known, known)
- charsets:116 a single shift does not change the locking shift (known, known)
- charsets:131 a soft reset selects the default character sets (known, known)
- charsets:139 a soft reset also unlocks a shifted G1 (known, known)
- charsets:164 DECNRCM makes the national sets available (known, known)
- charsets:172 the Dutch set replaces twelve positions of ASCII (known, known)
- charsets:180 the Finnish set replaces twelve positions of ASCII (known, known)
- charsets:188 the French set replaces twelve positions of ASCII (known, known)
- charsets:196 the French Canadian set replaces twelve positions of ASCII (known, known)
- charsets:204 the German set replaces twelve positions of ASCII (known, known)
- charsets:212 the Italian set replaces twelve positions of ASCII (known, known)
- charsets:220 the Norwegian and Danish set replaces twelve positions of ASCII (known, known)
- charsets:228 the Portuguese set replaces twelve positions of ASCII (known, known)
- charsets:236 the Spanish set replaces twelve positions of ASCII (known, known)
- charsets:244 the Swedish set replaces twelve positions of ASCII (known, known)
- charsets:252 the Swiss set replaces twelve positions of ASCII (known, known)
- charsets:260 the alternate finals of the national sets (known, known)
- charsets:268 a national set in G1 comes with SO (known, known)
- charsets:276 a national set is not designated by a terminal that does not have it (known, known)
- charsets:287 DECSTR returns to the multinational set (known, known)
- charsets:296 DEC Technical maps the mathematical symbols (known, known)
- charsets:304 DEC Technical leaves its unused positions blank (known, known)
- charsets:312 DEC Supplemental is the right half of the Multinational set (known, known)
- charsets:320 the Latin-1 supplement is a 96-character set (known, known)
- charsets:328 a single shift takes one character from G2 (known, known)
- charsets:336 the user-preferred supplemental set starts as DEC Supplemental (known, known)
- charsets:345 DECAUPSS assigns ISO Latin-1 as the user-preferred set (known, known)
- checksum:15 DECRQCRA of one cell is its code point (known, known)
- checksum:23 DECRQCRA of a rectangle is the sum of its cells (known, known)
- checksum:31 DECRQCRA counts a blank cell as a space (known, known)
- checksum:39 DECRQCRA counts an erased cell as a space (known, known)
- checksum:47 DECRQCRA without an area sums the page (known, known)
- checksum:55 DECRQCRA without parameters sums the page and answers for request 0 (known, known)
- checksum:63 DECRQCRA echoes its request number (known, known)
- checksum:71 DECRQCRA is cut to the screen (known, known)
- checksum:79 DECRQCRA of an upside-down area sums nothing (known, known)
- checksum:87 DECRQCRA counts from the region in origin mode (known, known)
- checksum:95 DECRQCRA counts a wide character once (known, known)
- checksum:103 DECRQCRA ignores combining marks (known, known)
- checksum:111 DECRQCRA ignores the video attributes (known, known)
- checksum:119 DECRQCRA sums modulo 65536 (known, known)
- checksum:135 DECRQCRA of the first page is answered (known, known)
- conformance-levels:4 DA1 names the VT500 family and its features by default (known, known)
- conformance-levels:11 DECSCL 62 takes the terminal down to VT200 (known, known)
- conformance-levels:18 DECSCL 61 is VT100 mode (known, known)
- conformance-levels:25 DECRQM is a VT300 feature (known, known)
- conformance-levels:34 left and right margins are a VT400 feature (known, known)
- conformance-levels:47 DECSCL is a hard reset (known, known)
- conformance-levels:58 DECRQSS reports the level and the control mode (known, known)
- conformance-levels:67 DECSCL without its second parameter chooses 8-bit controls (known, known)
- conformance-levels:80 S8C1T makes the replies use one-byte C1 controls (known, known)
- conformance-levels:89 C1 bytes are controls only in 8-bit mode (known, known)
- conformance-levels:100 8-bit IND, NEL, RI, HTS, DECID, SPA and EPA (known, known)
- conformance-levels:115 an 8-bit ST ends a string (known, known)
- conformance-levels:122 an 8-bit CSI breaks off a sequence (known, known)
- conformance-levels:131 DA1 lists what each family has (known, known)
- cursor:201 CUU stops at the top margin when the cursor is inside the region (known, known)
- cursor:208 CUD stops at the bottom margin when the cursor is inside the region (known, known)
- cursor:221 CUD above the region still stops at the bottom margin (known, known)
- cursor:272 new line mode makes line feed also return the carriage (known, known)
- cursor:332 IND at the bottom margin does not scroll a cursor outside the left and right margins (known, known)
- cursor:343 RI at the top margin does not scroll a cursor outside the left and right margins (known, known)
- cursor:357 BS and CUB go back to the line above with reverse wraparound (45) only if it wrapped (known, known)
- cursor:368 reverse wraparound (1045) goes back over a line that did not wrap and round the top (known, known)
- cursor:379 reverse wraparound goes to the right margin (known, known)
- cursor:394 a backspace in the delayed wrap with reverse wraparound only ends the wait (known, known)
- cursor:406 CUB with reverse wraparound walks back like backspace (known, known)
- edit:40 DCH with a huge count deletes to the end of the row (known, known)
- edit:199 DECIC inserts columns (known, known)
- edit:211 DECDC deletes columns (known, known)
- edit:223 DL does not feed the history (known, known)
- edit:235 ECH ends a pending wrap (known, known)
- edit:244 ICH ends a pending wrap (known, known)
- edit:252 DCH ends a pending wrap (known, known)
- edit:260 SL shifts the columns left and blanks come in at the right (known, known)
- edit:270 SR shifts the columns right and blanks come in at the left (known, known)
- edit:279 SL and SR stay between the margins (known, known)
- edit:293 SL erases with the background color (known, known)
- erase:17 ED 1 erases from the start of the screen to the cursor inclusive (known, known)
- erase:156 DECSCA protection keeps a character from DECSEL (known, known)
- erase:164 DECSEL 0 erases unprotected characters to the right (known, known)
- erase:172 DECSEL 1 erases unprotected characters to the left (known, known)
- erase:180 DECSED 2 erases every unprotected character on the screen (known, known)
- erase:188 DECSED 0 erases unprotected characters below the cursor (known, known)
- erase:211 DECSCA 2 ends protection (known, known)
- erase:220 EL ends a pending wrap (known, known)
- erase:229 ED ends a pending wrap (known, known)
- erase:247 ED 2 on a blank screen adds nothing to the history (known, known)
- erase:254 ED 3 before ED 2 leaves no history (known, known)
- erase:263 ED, EL and ECH leave a guarded area alone (known, known)
- erase:277 DECSCA protection is not a guarded area (known, known)
- graphics:6 a kitty image leaves the cursor right of its last row (known, known)
- graphics:38 a kitty image at the bottom scrolls the screen to make room (known, known)
- graphics:53 a kitty image wider than the rest of the line is cut and the cursor waits at the edge (known, known)
- graphics:73 a kitty query is answered with the id (known, known)
- graphics:86 an answer about an image keeps its place among the other replies (known, known)
- graphics:93 an iTerm2 inline image leaves the cursor right of its last row (known, known)
- margins:5 DECSLRM sets the margins and homes the cursor (known, known)
- margins:18 the right margin stops cursor forward (known, known)
- margins:25 the left margin stops cursor backward (known, known)
- margins:32 backspace at the left margin does nothing (known, known)
- margins:39 carriage return goes to the left margin (known, known)
- margins:46 a tab stops at the right margin (known, known)
- margins:59 wrapping goes from the right margin to the left margin (known, known)
- margins:68 ICH shifts only up to the right margin (known, known)
- margins:77 DCH shifts only up to the right margin (known, known)
- margins:85 ICH outside the margins does nothing (known, known)
- margins:93 DCH outside the margins does nothing (known, known)
- margins:101 insert mode shifts only up to the right margin (known, known)
- margins:109 IL moves only the columns between the margins (known, known)
- margins:120 DL moves only the columns between the margins (known, known)
- margins:131 line feed at the bottom margin scrolls only the margin columns (known, known)
- margins:142 SU moves only the columns between the margins (known, known)
- margins:160 DECIC inserts columns inside the margins (known, known)
- margins:171 DECDC deletes columns inside the margins (known, known)
- margins:182 forward index moves the cursor right (known, known)
- margins:189 forward index at the right margin scrolls the margins left (known, known)
- margins:199 back index moves the cursor left (known, known)
- margins:206 back index at the left margin scrolls the margins right (known, known)
- margins:228 DECSLRM needs the left margin below the right (known, known)
- margins:235 DECSLRM needs at least two columns (known, known)
- margins:242 origin mode with margins starts at the margins (known, known)
- margins:249 origin mode with margins clamps to them (known, known)
- modes:57 mode 5 reverse video can be asked for (known, known)
- modes:70 the VT510 modes a terminal emulator has no hardware for are kept and answered (known, known)
- modes:81 DECHCCM and the ECMA-48 modes without a terminal to apply to are permanently reset (known, known)
- modes:88 mode 41 makes a tab at the end of a row wrap first (known, known)
- origin:9 resetting origin mode moves the cursor to the home of the screen (known, known)
- origin:46 CPR is relative to the region (known, known)
- origin:78 a soft reset ends origin mode (known, known)
- osc:160 an empty title is no title (known, known)
- osc:181 the special colors are OSC 4 from 256 and OSC 5 (known, known)
- osc:194 OSC 10 sets the colors after it too, and OSC 110 to 119 reset them one by one (known, known)
- parser:13 CAN cancels a string (known, known)
- parser:183 a sixel image leaves the cursor under it (known, known)
- rect-attributes:6 DECCARA in rectangle extent changes the rectangle (known, known)
- rect-attributes:24 DECCARA in stream extent changes the stream (known, known)
- rect-attributes:37 DECSACE 1 is the stream too (known, known)
- rect-attributes:46 DECSACE 2 and back to the stream (known, known)
- rect-attributes:53 DECSACE with another value changes nothing (known, known)
- rect-attributes:61 DECCARA in stream extent on one line (known, known)
- rect-attributes:84 DECCARA 0 clears bold, underline, blink and negative image (known, known)
- rect-attributes:92 DECCARA without attributes is the same as 0 (known, known)
- rect-attributes:99 DECCARA 22 clears bold (known, known)
- rect-attributes:105 DECCARA 24 clears underline (known, known)
- rect-attributes:116 DECCARA 27 clears inverse (known, known)
- rect-attributes:122 DECCARA 1 sets bold (known, known)
- rect-attributes:130 DECCARA 4 sets underline (known, known)
- rect-attributes:138 DECCARA 5 sets blink (known, known)
- rect-attributes:146 DECCARA 7 sets inverse (known, known)
- rect-attributes:154 DECCARA 8 sets hidden (known, known)
- rect-attributes:162 DECCARA 28 clears hidden (known, known)
- rect-attributes:169 DECCARA applies the attributes in order (known, known)
- rect-attributes:176 DECCARA ignores an attribute it does not know (known, known)
- rect-attributes:183 DECCARA replaces another underline style (known, known)
- rect-attributes:190 DECCARA changes blank cells in rectangle extent (known, known)
- rect-attributes:215 DECCARA counts from the region in origin mode (known, known)
- rect-attributes:225 DECCARA ignores the margins (known, known)
- rect-attributes:237 DECCARA is cut to the screen (known, known)
- rect-attributes:244 DECRARA reverses bold (known, known)
- rect-attributes:254 DECRARA 4 reverses underline (known, known)
- rect-attributes:261 DECRARA 5 reverses blink (known, known)
- rect-attributes:268 DECRARA 7 reverses inverse (known, known)
- rect-attributes:275 DECRARA 8 reverses hidden (known, known)
- rect-attributes:282 DECRARA 0 reverses bold, underline, blink and negative image (known, known)
- rect-attributes:290 DECRARA without attributes is the same as 0 (known, known)
- rect-attributes:301 DECRARA reverses an underline style to none (known, known)
- rect-attributes:312 DECRARA in stream extent reverses the stream (known, known)
- rect-attributes:323 DECRARA in rectangle extent reverses the rectangle (known, known)
- rect-attributes:333 RIS selects the stream extent again (known, known)
- rect-attributes:340 DECSTR leaves the extent as DECSACE set it (known, known)
- rect:8 DECERA acts on the rectangle (known, known)
- rect:22 DECERA without an area takes the whole page (known, known)
- rect:62 DECERA is cut to the screen (known, known)
- rect:76 DECERA counts from the region in origin mode (known, known)
- rect:90 DECERA ignores the margins (known, known)
- rect:104 DECERA leaves the cursor where it is (known, known)
- rect:119 DECERA also erases protected characters (known, known)
- rect:133 DECERA gives the cells the current background (known, known)
- rect:141 DECERA through half a wide character blanks both halves (known, known)
- rect:147 DECSERA acts on the rectangle (known, known)
- rect:161 DECSERA without an area takes the whole page (known, known)
- rect:201 DECSERA is cut to the screen (known, known)
- rect:215 DECSERA counts from the region in origin mode (known, known)
- rect:229 DECSERA ignores the margins (known, known)
- rect:243 DECSERA leaves the cursor where it is (known, known)
- rect:258 DECSERA leaves protected characters (known, known)
- rect:265 DECSERA without an area erases the unprotected characters of the page (known, known)
- rect:272 DECFRA acts on the rectangle (known, known)
- rect:286 DECFRA without an area takes the whole page (known, known)
- rect:326 DECFRA is cut to the screen (known, known)
- rect:340 DECFRA counts from the region in origin mode (known, known)
- rect:354 DECFRA ignores the margins (known, known)
- rect:368 DECFRA leaves the cursor where it is (known, known)
- rect:383 DECFRA fills with the current rendition (known, known)
- rect:391 DECFRA takes a character from the right-hand half (known, known)
- rect:449 DECFRA over half a wide character blanks the other half (known, known)
- rect:455 DECCRA copies a rectangle (known, known)
- rect:469 DECCRA copies as if the source were read whole first when the areas overlap downwards (known, known)
- rect:483 DECCRA copies as if the source were read whole first when the areas overlap upwards (known, known)
- rect:497 DECCRA copies within a row to the right (known, known)
- rect:511 DECCRA copies within a row to the left (known, known)
- rect:525 DECCRA is cut at the screen's edge (known, known)
- rect:539 DECCRA with the default source takes it from the corner (known, known)
- rect:553 DECCRA with the default target copies to the corner (known, known)
- rect:580 DECCRA with a source beyond the screen is cut to it (known, known)
- rect:594 DECCRA counts both corners from the region in origin mode (known, known)
- rect:608 DECCRA ignores the margins (known, known)
- rect:653 DECCRA copies the rendition and the link (known, known)
- rect:666 DECCRA copies a whole wide character (known, known)
- rect:674 DECCRA onto the second half of a wide character blanks the first (known, known)
- rect:680 DECCRA onto the first half of a wide character blanks the second (known, known)
- reports:26 DSR 6 is relative to the margins in origin mode (known, known)
- reports:53 DECXCPR reports the cursor position (known, known)
- reports:60 DSR ? 15 reports the printer status (known, known)
- reports:67 DSR ? 25 reports the user defined key status (known, known)
- reports:104 secondary device attributes (known, known)
- reports:111 secondary device attributes with an explicit 0 (known, known)
- reports:118 tertiary device attributes report a unit id (known, known)
- reports:125 XTVERSION reports the terminal's name (known, known)
- reports:132 XTVERSION with an explicit 0 (known, known)
- reports:151 the cell size in pixels (known, known)
- reports:207 DECRQM of left and right margin mode (known, known)
- reports:214 DECRQM of synchronized output (known, known)
- reports:221 DECRQM of grapheme clustering (known, known)
- reports:228 DECRQSS reports the default SGR (known, known)
- reports:235 DECRQSS reports the current SGR (known, known)
- reports:242 DECRQSS reports the SGR colors (known, known)
- reports:249 DECRQSS reports the scrolling region (known, known)
- reports:258 DECRQSS reports the cursor style (known, known)
- reports:265 DECRQSS reports the character protection (known, known)
- reports:272 DECRQSS reports the left and right margins (known, known)
- reports:279 DECRQSS answers an invalid request with 0 (known, known)
- reports:328 DECRQSS reports the settings that are only kept (known, known)
- reports:336 DECDSR 56, 62, 63 and 85 answer that the locator is a mouse, no macro space and no sessions (known, known)
- reports:343 DECDSR 53 and 55 answer that a locator is available (known, known)
- reports:350 DECXCPR and the keyboard report grow with the conformance level (known, known)
- reports:361 DECRQPSR 1 reports the cursor information (known, known)
- reports:368 DECCIR reports the position, the rendition and the protection (known, known)
- reports:375 DECCIR reports origin mode, a pending wrap and a single shift (known, known)
- reports:382 DECCIR reports the character sets, their sizes and what is invoked (known, known)
- reports:389 DECRSPS puts the cursor information back (known, known)
- reports:406 DECRQPSR 2 reports the tab stops (known, known)
- reports:415 DECRSPS puts the tab stops back (known, known)
- reports:426 DECRQTSR 1 reports the state (known, known)
- reports:433 DECRSTS puts the state back (known, known)
- reports:450 DECRQDE reports the displayed extent (known, known)
- reports:457 DECRQLP says there is no locator until DECELR (known, known)
- reports:464 DECRQLP reports the locator once DECELR enabled it (known, known)
- reports:471 DECELR 2 enables the locator for one report (known, known)
- reports:478 DECELR 0 disables the locator (known, known)
- reports:485 the locator replaces mouse tracking (known, known)
- reset:95 DECSTR returns the rendition to normal (known, known)
- reset:102 DECSTR ends origin mode and removes the margins (known, known)
- reset:114 DECSTR ends insert mode (known, known)
- reset:122 DECSTR shows the cursor (known, known)
- reset:129 DECSTR ends selective erase protection (known, known)
- reset:137 DECSTR resets modes seen by DECRQM (known, known)
- reset:154 DECALN homes the cursor (known, known)
- reset:161 DECALN removes the margins (known, known)
- resize:26 narrowing keeps the rows on screen when they fit (known, known)
- save-restore:34 DECRC restores the origin mode (known, known)
- save-restore:88 a soft reset forgets the saved cursor (known, known)
- sgr:46 SGR 5 is blink (known, known)
- sgr:71 SGR 21 is double underline (known, known)
- sgr:78 SGR 53 is overline (known, known)
- tabs:81 DECST8C restores the stops every eight columns (known, known)
- tabs:152 CBT with every stop cleared stops at the first column (known, known)
- unicode:125 erasing to the right from the second half removes the whole character (known, known)
- unicode:133 erasing characters over half of a wide character removes the whole character (known, known)
- unicode:141 grapheme clusters: a ZWJ emoji sequence is one cluster two cells wide (known, known)
- unicode:151 grapheme clusters: a skin tone modifier stays with its emoji (known, known)
- unicode:160 grapheme clusters: two regional indicators are one flag (known, known)
- unicode:177 grapheme clusters: VS16 makes a text symbol wide (known, known)
- unicode:186 grapheme clusters: VS16 at the right margin moves the cluster to the next row (known, known)
- unicode:203 grapheme clusters: VS15 makes a default-emoji character narrow (known, known)
- unicode:212 grapheme clusters: VS15 gives the second cell of a wide emoji back (known, known)
- unicode:222 grapheme clusters: VS15 at the right margin steps the cursor back (known, known)
- unicode:242 grapheme clusters: VS15 then VS16 is not a sequence (known, known)
- unicode:250 grapheme clusters: a spacing mark takes a cell (known, known)
- unicode:259 grapheme clusters: several spacing marks and a nonspacing mark stay two cells (known, known)
- unicode:274 grapheme clusters: a conjunct takes two cells (known, known)
- unicode:282 grapheme clusters: a longer conjunct with a vowel sign stays two cells (known, known)
- unicode:304 grapheme clusters: Khmer coeng makes a conjunct (known, known)
- unicode:312 grapheme clusters: an Arabic number sign has a cell (known, known)
- unicode:350 grapheme clusters: a soft reset returns the mode to its default (known, known)
- unicode:357 grapheme clusters: a full reset returns the mode to its default (known, known)
- unicode:364 a lone continuation byte is one replacement character (known, known)
- unicode:393 an overlong encoding is a replacement character per byte (known, known)
- unicode:401 an encoded surrogate is a replacement character per byte (known, known)
- unicode:416 a character completed by a later feed, one character, then half of another (known, known)
- window:4 XTWINOPS reports the text area, the cell and the window state (known, known)
- window:19 the title and the icon name are reported when the host allows it (known, known)
- window:29 controls are taken out of a reported title (known, known)
- window:37 the title stack keeps the title and the icon name together or apart (known, known)
- window:47 a pop takes the slot that holds what it asks for (known, known)
- window:55 the title modes read and report in hexadecimal (known, known)
- window:65 a full reset clears the title modes and the stack (known, known)
- window:79 the sizes in cells take effect at once when allowed (known, known)
- window:97 a size past what the screen holds is cut (known, known)
- wrap:26 a line feed clears the pending wrap (known, known)
- wrap:110 a line feed in the pending state on the last row scrolls once (known, known)
- wrap:193 a tab with 'more fix' on wraps first (known, known)

### wormhole (0 cases)


### wezterm (274 cases)

- altscreen:25 mode 1049 leaves the cursor where it is (screen)
- charsets:84 locking shift 2 invokes G2 (screen)
- charsets:92 locking shift 3 invokes G3 (screen)
- charsets:100 single shift 2 affects the next character only (screen)
- charsets:108 single shift 3 affects the next character only (screen)
- charsets:116 a single shift does not change the locking shift (screen)
- charsets:164 DECNRCM makes the national sets available (screen)
- charsets:172 the Dutch set replaces twelve positions of ASCII (screen)
- charsets:180 the Finnish set replaces twelve positions of ASCII (screen)
- charsets:188 the French set replaces twelve positions of ASCII (screen)
- charsets:196 the French Canadian set replaces twelve positions of ASCII (screen)
- charsets:204 the German set replaces twelve positions of ASCII (screen)
- charsets:212 the Italian set replaces twelve positions of ASCII (screen)
- charsets:220 the Norwegian and Danish set replaces twelve positions of ASCII (screen)
- charsets:228 the Portuguese set replaces twelve positions of ASCII (screen)
- charsets:236 the Spanish set replaces twelve positions of ASCII (screen)
- charsets:244 the Swedish set replaces twelve positions of ASCII (screen)
- charsets:252 the Swiss set replaces twelve positions of ASCII (screen)
- charsets:260 the alternate finals of the national sets (screen)
- charsets:268 a national set in G1 comes with SO (screen)
- charsets:276 a national set is not designated by a terminal that does not have it (row)
- charsets:287 DECSTR returns to the multinational set (reply)
- charsets:296 DEC Technical maps the mathematical symbols (screen)
- charsets:304 DEC Technical leaves its unused positions blank (screen)
- charsets:312 DEC Supplemental is the right half of the Multinational set (screen)
- charsets:320 the Latin-1 supplement is a 96-character set (screen)
- charsets:328 a single shift takes one character from G2 (screen)
- charsets:336 the user-preferred supplemental set starts as DEC Supplemental (screen)
- charsets:345 DECAUPSS assigns ISO Latin-1 as the user-preferred set (screen)
- checksum:15 DECRQCRA of one cell is its code point (reply)
- checksum:23 DECRQCRA of a rectangle is the sum of its cells (reply)
- checksum:31 DECRQCRA counts a blank cell as a space (reply)
- checksum:39 DECRQCRA counts an erased cell as a space (reply)
- checksum:47 DECRQCRA without an area sums the page (reply)
- checksum:55 DECRQCRA without parameters sums the page and answers for request 0 (reply)
- checksum:63 DECRQCRA echoes its request number (reply)
- checksum:71 DECRQCRA is cut to the screen (reply)
- checksum:79 DECRQCRA of an upside-down area sums nothing (reply)
- checksum:87 DECRQCRA counts from the region in origin mode (reply)
- checksum:95 DECRQCRA counts a wide character once (reply)
- checksum:103 DECRQCRA ignores combining marks (reply)
- checksum:111 DECRQCRA ignores the video attributes (reply)
- checksum:119 DECRQCRA sums modulo 65536 (reply)
- checksum:135 DECRQCRA of the first page is answered (reply)
- conformance-levels:4 DA1 names the VT500 family and its features by default (reply)
- conformance-levels:11 DECSCL 62 takes the terminal down to VT200 (reply)
- conformance-levels:18 DECSCL 61 is VT100 mode (reply)
- conformance-levels:25 DECRQM is a VT300 feature (reply)
- conformance-levels:34 left and right margins are a VT400 feature (screen)
- conformance-levels:47 DECSCL is a hard reset (screen)
- conformance-levels:58 DECRQSS reports the level and the control mode (reply)
- conformance-levels:67 DECSCL without its second parameter chooses 8-bit controls (reply)
- conformance-levels:80 S8C1T makes the replies use one-byte C1 controls (reply)
- conformance-levels:89 C1 bytes are controls only in 8-bit mode (screen)
- conformance-levels:100 8-bit IND, NEL, RI, HTS, DECID, SPA and EPA (reply)
- conformance-levels:131 DA1 lists what each family has (reply)
- cursor:357 BS and CUB go back to the line above with reverse wraparound (45) only if it wrapped (cursor)
- cursor:368 reverse wraparound (1045) goes back over a line that did not wrap and round the top (cursor)
- cursor:379 reverse wraparound goes to the right margin (cursor)
- edit:199 DECIC inserts columns (screen)
- edit:211 DECDC deletes columns (screen)
- edit:223 DL does not feed the history (scrollback-len)
- edit:235 ECH ends a pending wrap (screen)
- edit:244 ICH ends a pending wrap (screen)
- edit:252 DCH ends a pending wrap (screen)
- edit:260 SL shifts the columns left and blanks come in at the right (screen)
- edit:270 SR shifts the columns right and blanks come in at the left (screen)
- edit:279 SL and SR stay between the margins (screen)
- edit:293 SL erases with the background color (cell)
- erase:72 ED erased cells keep only the background of the current rendition (cell)
- erase:156 DECSCA protection keeps a character from DECSEL (screen)
- erase:164 DECSEL 0 erases unprotected characters to the right (screen)
- erase:172 DECSEL 1 erases unprotected characters to the left (screen)
- erase:180 DECSED 2 erases every unprotected character on the screen (screen)
- erase:188 DECSED 0 erases unprotected characters below the cursor (screen)
- erase:211 DECSCA 2 ends protection (screen)
- erase:220 EL ends a pending wrap (screen)
- erase:229 ED ends a pending wrap (screen)
- erase:237 ED 2 keeps the text it clears as history (scrollback)
- erase:263 ED, EL and ECH leave a guarded area alone (screen)
- erase:277 DECSCA protection is not a guarded area (screen)
- graphics:6 a kitty image leaves the cursor right of its last row (cursor)
- graphics:38 a kitty image at the bottom scrolls the screen to make room (cursor)
- graphics:53 a kitty image wider than the rest of the line is cut and the cursor waits at the edge (cursor)
- graphics:73 a kitty query is answered with the id (reply)
- graphics:86 an answer about an image keeps its place among the other replies (reply)
- graphics:93 an iTerm2 inline image leaves the cursor right of its last row (cursor)
- margins:160 DECIC inserts columns inside the margins (screen)
- margins:171 DECDC deletes columns inside the margins (screen)
- margins:182 forward index moves the cursor right (cursor)
- margins:189 forward index at the right margin scrolls the margins left (screen)
- margins:199 back index moves the cursor left (cursor)
- margins:206 back index at the left margin scrolls the margins right (screen)
- modes:57 mode 5 reverse video can be asked for (reply)
- modes:70 the VT510 modes a terminal emulator has no hardware for are kept and answered (reply)
- modes:81 DECHCCM and the ECMA-48 modes without a terminal to apply to are permanently reset (reply)
- modes:88 mode 41 makes a tab at the end of a row wrap first (reply)
- osc:23 OSC 1 changes the icon name, not the window title (title)
- osc:48 a title can be pushed and popped (title)
- osc:56 the title stack holds several titles (title)
- osc:64 a window title push and pop works with Ps = 2 (title)
- osc:167 the colors a program sets are kept until it resets them, and a query answers them (reply)
- osc:175 a spec with # is the top bits of the byte and rgb: is scaled (reply)
- osc:181 the special colors are OSC 4 from 256 and OSC 5 (reply)
- osc:194 OSC 10 sets the colors after it too, and OSC 110 to 119 reset them one by one (reply)
- osc:203 a full reset puts the colors back (reply)
- parser:13 CAN cancels a string (title)
- parser:106 a huge parameter is clamped (cursor)
- parser:183 a sixel image leaves the cursor under it (cursor)
- rect-attributes:6 DECCARA in rectangle extent changes the rectangle (cell)
- rect-attributes:24 DECCARA in stream extent changes the stream (cell)
- rect-attributes:37 DECSACE 1 is the stream too (cell)
- rect-attributes:46 DECSACE 2 and back to the stream (cell)
- rect-attributes:53 DECSACE with another value changes nothing (cell)
- rect-attributes:61 DECCARA in stream extent on one line (cell)
- rect-attributes:84 DECCARA 0 clears bold, underline, blink and negative image (cell)
- rect-attributes:92 DECCARA without attributes is the same as 0 (cell)
- rect-attributes:99 DECCARA 22 clears bold (cell)
- rect-attributes:105 DECCARA 24 clears underline (cell)
- rect-attributes:111 DECCARA 25 clears blink (cell)
- rect-attributes:116 DECCARA 27 clears inverse (cell)
- rect-attributes:122 DECCARA 1 sets bold (cell)
- rect-attributes:130 DECCARA 4 sets underline (cell)
- rect-attributes:138 DECCARA 5 sets blink (cell)
- rect-attributes:146 DECCARA 7 sets inverse (cell)
- rect-attributes:154 DECCARA 8 sets hidden (cell)
- rect-attributes:162 DECCARA 28 clears hidden (cell)
- rect-attributes:169 DECCARA applies the attributes in order (cell)
- rect-attributes:176 DECCARA ignores an attribute it does not know (cell)
- rect-attributes:183 DECCARA replaces another underline style (cell)
- rect-attributes:190 DECCARA changes blank cells in rectangle extent (cell)
- rect-attributes:215 DECCARA counts from the region in origin mode (cell)
- rect-attributes:225 DECCARA ignores the margins (cell)
- rect-attributes:237 DECCARA is cut to the screen (cell)
- rect-attributes:244 DECRARA reverses bold (cell)
- rect-attributes:254 DECRARA 4 reverses underline (cell)
- rect-attributes:261 DECRARA 5 reverses blink (cell)
- rect-attributes:268 DECRARA 7 reverses inverse (cell)
- rect-attributes:275 DECRARA 8 reverses hidden (cell)
- rect-attributes:282 DECRARA 0 reverses bold, underline, blink and negative image (cell)
- rect-attributes:290 DECRARA without attributes is the same as 0 (cell)
- rect-attributes:301 DECRARA reverses an underline style to none (cell)
- rect-attributes:312 DECRARA in stream extent reverses the stream (cell)
- rect-attributes:323 DECRARA in rectangle extent reverses the rectangle (cell)
- rect-attributes:333 RIS selects the stream extent again (cell)
- rect-attributes:340 DECSTR leaves the extent as DECSACE set it (cell)
- rect:8 DECERA acts on the rectangle (screen)
- rect:22 DECERA without an area takes the whole page (screen)
- rect:62 DECERA is cut to the screen (screen)
- rect:76 DECERA counts from the region in origin mode (screen)
- rect:90 DECERA ignores the margins (screen)
- rect:104 DECERA leaves the cursor where it is (screen)
- rect:119 DECERA also erases protected characters (screen)
- rect:133 DECERA gives the cells the current background (cell)
- rect:141 DECERA through half a wide character blanks both halves (row)
- rect:147 DECSERA acts on the rectangle (screen)
- rect:161 DECSERA without an area takes the whole page (screen)
- rect:201 DECSERA is cut to the screen (screen)
- rect:215 DECSERA counts from the region in origin mode (screen)
- rect:229 DECSERA ignores the margins (screen)
- rect:243 DECSERA leaves the cursor where it is (screen)
- rect:258 DECSERA leaves protected characters (screen)
- rect:265 DECSERA without an area erases the unprotected characters of the page (screen)
- rect:272 DECFRA acts on the rectangle (screen)
- rect:286 DECFRA without an area takes the whole page (screen)
- rect:326 DECFRA is cut to the screen (screen)
- rect:340 DECFRA counts from the region in origin mode (screen)
- rect:354 DECFRA ignores the margins (screen)
- rect:368 DECFRA leaves the cursor where it is (screen)
- rect:383 DECFRA fills with the current rendition (cell)
- rect:391 DECFRA takes a character from the right-hand half (row)
- rect:449 DECFRA over half a wide character blanks the other half (row)
- rect:455 DECCRA copies a rectangle (screen)
- rect:469 DECCRA copies as if the source were read whole first when the areas overlap downwards (screen)
- rect:483 DECCRA copies as if the source were read whole first when the areas overlap upwards (screen)
- rect:497 DECCRA copies within a row to the right (screen)
- rect:511 DECCRA copies within a row to the left (screen)
- rect:525 DECCRA is cut at the screen's edge (screen)
- rect:539 DECCRA with the default source takes it from the corner (screen)
- rect:553 DECCRA with the default target copies to the corner (screen)
- rect:580 DECCRA with a source beyond the screen is cut to it (screen)
- rect:594 DECCRA counts both corners from the region in origin mode (screen)
- rect:608 DECCRA ignores the margins (screen)
- rect:653 DECCRA copies the rendition and the link (cell)
- rect:666 DECCRA copies a whole wide character (row)
- rect:674 DECCRA onto the second half of a wide character blanks the first (row)
- rect:680 DECCRA onto the first half of a wide character blanks the second (row)
- reports:53 DECXCPR reports the cursor position (reply)
- reports:60 DSR ? 15 reports the printer status (reply)
- reports:67 DSR ? 25 reports the user defined key status (reply)
- reports:98 DECID answers like primary device attributes (reply)
- reports:201 DECRQM of the alternate screen (reply)
- reports:214 DECRQM of synchronized output (reply)
- reports:221 DECRQM of grapheme clustering (reply)
- reports:228 DECRQSS reports the default SGR (reply)
- reports:235 DECRQSS reports the current SGR (reply)
- reports:242 DECRQSS reports the SGR colors (reply)
- reports:258 DECRQSS reports the cursor style (reply)
- reports:265 DECRQSS reports the character protection (reply)
- reports:286 the foreground color can be asked for (reply)
- reports:298 the cursor color can be asked for (reply)
- reports:304 a palette color can be asked for (reply)
- reports:310 a palette color that was set is reported back (reply)
- reports:316 the foreground color that was set is reported back (reply)
- reports:322 several palette colors can be asked for at once (reply)
- reports:328 DECRQSS reports the settings that are only kept (reply)
- reports:336 DECDSR 56, 62, 63 and 85 answer that the locator is a mouse, no macro space and no sessions (reply)
- reports:343 DECDSR 53 and 55 answer that a locator is available (reply)
- reports:350 DECXCPR and the keyboard report grow with the conformance level (reply)
- reports:361 DECRQPSR 1 reports the cursor information (reply)
- reports:368 DECCIR reports the position, the rendition and the protection (reply)
- reports:375 DECCIR reports origin mode, a pending wrap and a single shift (reply)
- reports:382 DECCIR reports the character sets, their sizes and what is invoked (reply)
- reports:389 DECRSPS puts the cursor information back (cursor)
- reports:406 DECRQPSR 2 reports the tab stops (reply)
- reports:415 DECRSPS puts the tab stops back (cursor)
- reports:426 DECRQTSR 1 reports the state (reply)
- reports:433 DECRSTS puts the state back (cursor)
- reports:450 DECRQDE reports the displayed extent (reply)
- reports:457 DECRQLP says there is no locator until DECELR (reply)
- reports:464 DECRQLP reports the locator once DECELR enabled it (reply)
- reports:471 DECELR 2 enables the locator for one report (reply)
- reports:478 DECELR 0 disables the locator (reply)
- reports:485 the locator replaces mouse tracking (reply)
- reset:74 RIS forgets the saved cursor (screen)
- reset:122 DECSTR shows the cursor (cursor)
- reset:129 DECSTR ends selective erase protection (screen)
- resize:87 the cursor stays inside after narrowing (cursor)
- save-restore:96 a full reset forgets the saved cursor (screen)
- tabs:81 DECST8C restores the stops every eight columns (screen)
- unicode:61 a combining mark joins the cell before it (cell, only when fed a byte at a time)
- unicode:71 several combining marks join one cell (cell, only when fed a byte at a time)
- unicode:78 a combining mark joins a wide character (cell, only when fed a byte at a time)
- unicode:141 grapheme clusters: a ZWJ emoji sequence is one cluster two cells wide (cell, only when fed a byte at a time)
- unicode:151 grapheme clusters: a skin tone modifier stays with its emoji (cursor, only when fed a byte at a time)
- unicode:160 grapheme clusters: two regional indicators are one flag (cell, only when fed a byte at a time)
- unicode:169 grapheme clusters: a combining mark does not move the cursor (cell, only when fed a byte at a time)
- unicode:177 grapheme clusters: VS16 makes a text symbol wide (cell)
- unicode:186 grapheme clusters: VS16 at the right margin moves the cluster to the next row (screen)
- unicode:196 grapheme clusters: VS15 leaves an emoji that has no text presentation alone (cell, only when fed a byte at a time)
- unicode:203 grapheme clusters: VS15 makes a default-emoji character narrow (cell)
- unicode:212 grapheme clusters: VS15 gives the second cell of a wide emoji back (cell)
- unicode:222 grapheme clusters: VS15 at the right margin steps the cursor back (cursor)
- unicode:235 grapheme clusters: VS15 does not change the width of a text symbol (cell, only when fed a byte at a time)
- unicode:242 grapheme clusters: VS15 then VS16 is not a sequence (cell)
- unicode:250 grapheme clusters: a spacing mark takes a cell (cell)
- unicode:259 grapheme clusters: several spacing marks and a nonspacing mark stay two cells (cell)
- unicode:274 grapheme clusters: a conjunct takes two cells (cell, only when fed a byte at a time)
- unicode:282 grapheme clusters: a longer conjunct with a vowel sign stays two cells (cell, only when fed a byte at a time)
- unicode:304 grapheme clusters: Khmer coeng makes a conjunct (cell)
- unicode:312 grapheme clusters: an Arabic number sign has a cell (cell)
- unicode:327 without the mode each spacing mark takes a cell of its own (cell)
- unicode:350 grapheme clusters: a soft reset returns the mode to its default (reply)
- unicode:357 grapheme clusters: a full reset returns the mode to its default (reply)
- unicode:364 a lone continuation byte is one replacement character (screen)
- unicode:372 an impossible byte is one replacement character (screen)
- unicode:379 a truncated sequence is one replacement character (screen)
- unicode:386 a truncated four byte sequence is one replacement character (screen)
- unicode:393 an overlong encoding is a replacement character per byte (screen)
- unicode:401 an encoded surrogate is a replacement character per byte (screen)
- unicode:409 a replacement character takes one cell (cell)
- unicode:426 a combining mark with nothing before it is kept on a blank (cell)
- window:4 XTWINOPS reports the text area, the cell and the window state (reply)
- window:19 the title and the icon name are reported when the host allows it (reply)
- window:29 controls are taken out of a reported title (reply)
- window:37 the title stack keeps the title and the icon name together or apart (reply)
- window:47 a pop takes the slot that holds what it asks for (reply)
- window:55 the title modes read and report in hexadecimal (reply)
- window:65 a full reset clears the title modes and the stack (reply)
- window:79 the sizes in cells take effect at once when allowed (reply)
- window:97 a size past what the screen holds is cut (reply)
- wrap:142 a wide character that does not fit wraps as a whole (screen)
- wrap:155 a wide character that fits exactly leaves the cursor waiting (cursor)
- wrap:193 a tab with 'more fix' on wraps first (screen)

### vt100 (268 cases)

- altscreen:25 mode 1049 leaves the cursor where it is (screen)
- altscreen:71 mode 1047 clears the alternate screen when leaving it (alt-screen)
- altscreen:86 mode 1047 brings back the normal screen (screen)
- altscreen:95 mode 1048 saves and restores the cursor (cursor)
- altscreen:103 mode 47 uses the alternate screen (screen)
- altscreen:131 mode 1049 again on the alternate screen changes nothing (screen)
- charsets:4 DEC special graphics draws a box (screen)
- charsets:13 DEC special graphics maps the line drawing characters (screen)
- charsets:20 DEC special graphics maps the scan lines (screen)
- charsets:27 DEC special graphics maps the symbols (screen)
- charsets:34 DEC special graphics maps the diamond (screen)
- charsets:41 DEC special graphics maps the control symbols (screen)
- charsets:55 designating ASCII again stops the mapping (screen)
- charsets:62 the UK set shows the pound sign for number sign (screen)
- charsets:70 shift out selects G1 and shift in selects G0 (screen)
- charsets:77 G1 can be designated while G0 is in use (screen)
- charsets:84 locking shift 2 invokes G2 (screen)
- charsets:92 locking shift 3 invokes G3 (screen)
- charsets:100 single shift 2 affects the next character only (screen)
- charsets:108 single shift 3 affects the next character only (screen)
- charsets:116 a single shift does not change the locking shift (screen)
- charsets:147 the character set applies to repeated characters (screen)
- charsets:164 DECNRCM makes the national sets available (screen)
- charsets:172 the Dutch set replaces twelve positions of ASCII (screen)
- charsets:180 the Finnish set replaces twelve positions of ASCII (screen)
- charsets:188 the French set replaces twelve positions of ASCII (screen)
- charsets:196 the French Canadian set replaces twelve positions of ASCII (screen)
- charsets:204 the German set replaces twelve positions of ASCII (screen)
- charsets:212 the Italian set replaces twelve positions of ASCII (screen)
- charsets:220 the Norwegian and Danish set replaces twelve positions of ASCII (screen)
- charsets:228 the Portuguese set replaces twelve positions of ASCII (screen)
- charsets:236 the Spanish set replaces twelve positions of ASCII (screen)
- charsets:244 the Swedish set replaces twelve positions of ASCII (screen)
- charsets:252 the Swiss set replaces twelve positions of ASCII (screen)
- charsets:260 the alternate finals of the national sets (screen)
- charsets:268 a national set in G1 comes with SO (screen)
- charsets:276 a national set is not designated by a terminal that does not have it (row)
- charsets:296 DEC Technical maps the mathematical symbols (screen)
- charsets:304 DEC Technical leaves its unused positions blank (screen)
- charsets:312 DEC Supplemental is the right half of the Multinational set (screen)
- charsets:320 the Latin-1 supplement is a 96-character set (screen)
- charsets:328 a single shift takes one character from G2 (screen)
- conformance-levels:34 left and right margins are a VT400 feature (screen)
- conformance-levels:89 C1 bytes are controls only in 8-bit mode (screen)
- conformance-levels:115 an 8-bit ST ends a string (title)
- conformance-levels:122 an 8-bit CSI breaks off a sequence (screen)
- cursor:46 HVP is CUP (cursor)
- cursor:171 HPA moves to a column (cursor)
- cursor:177 HPR moves right (cursor)
- cursor:195 VPR moves down (cursor)
- cursor:221 CUD above the region still stops at the bottom margin (cursor)
- cursor:272 new line mode makes line feed also return the carriage (screen)
- cursor:282 NEL moves to the start of the next line (screen)
- cursor:290 IND moves down and keeps the column (screen)
- cursor:343 RI at the top margin does not scroll a cursor outside the left and right margins (cell)
- cursor:351 CBT goes past the left margin (cursor)
- cursor:357 BS and CUB go back to the line above with reverse wraparound (45) only if it wrapped (cursor)
- cursor:368 reverse wraparound (1045) goes back over a line that did not wrap and round the top (cursor)
- cursor:379 reverse wraparound goes to the right margin (cursor)
- cursor:394 a backspace in the delayed wrap with reverse wraparound only ends the wait (screen)
- cursor:406 CUB with reverse wraparound walks back like backspace (cursor)
- edit:133 IL below the region does nothing (screen)
- edit:143 DL above the region does nothing (screen)
- edit:153 insert mode pushes characters right (screen)
- edit:161 insert mode loses what is pushed off the row (screen)
- edit:175 REP repeats the last character (screen)
- edit:183 REP defaults to one repeat (screen)
- edit:190 REP wraps like typed text (screen)
- edit:199 DECIC inserts columns (screen)
- edit:211 DECDC deletes columns (screen)
- edit:235 ECH ends a pending wrap (screen)
- edit:244 ICH ends a pending wrap (screen)
- edit:252 DCH ends a pending wrap (screen)
- edit:260 SL shifts the columns left and blanks come in at the right (screen)
- edit:270 SR shifts the columns right and blanks come in at the left (screen)
- edit:279 SL and SR stay between the margins (screen)
- edit:293 SL erases with the background color (cell)
- erase:40 ED 3 erases the saved lines and keeps the screen (scrollback-len)
- erase:72 ED erased cells keep only the background of the current rendition (cell)
- erase:156 DECSCA protection keeps a character from DECSEL (screen)
- erase:164 DECSEL 0 erases unprotected characters to the right (screen)
- erase:172 DECSEL 1 erases unprotected characters to the left (screen)
- erase:180 DECSED 2 erases every unprotected character on the screen (screen)
- erase:188 DECSED 0 erases unprotected characters below the cursor (screen)
- erase:211 DECSCA 2 ends protection (screen)
- erase:220 EL ends a pending wrap (screen)
- erase:229 ED ends a pending wrap (screen)
- erase:237 ED 2 keeps the text it clears as history (scrollback)
- erase:254 ED 3 before ED 2 leaves no history (scrollback-len)
- erase:263 ED, EL and ECH leave a guarded area alone (screen)
- erase:277 DECSCA protection is not a guarded area (screen)
- graphics:6 a kitty image leaves the cursor right of its last row (cursor)
- graphics:38 a kitty image at the bottom scrolls the screen to make room (cursor)
- graphics:53 a kitty image wider than the rest of the line is cut and the cursor waits at the edge (cursor)
- graphics:93 an iTerm2 inline image leaves the cursor right of its last row (cursor)
- margins:5 DECSLRM sets the margins and homes the cursor (cursor)
- margins:18 the right margin stops cursor forward (cursor)
- margins:25 the left margin stops cursor backward (cursor)
- margins:32 backspace at the left margin does nothing (cursor)
- margins:39 carriage return goes to the left margin (cursor)
- margins:46 a tab stops at the right margin (cursor)
- margins:59 wrapping goes from the right margin to the left margin (screen)
- margins:68 ICH shifts only up to the right margin (screen)
- margins:77 DCH shifts only up to the right margin (screen)
- margins:85 ICH outside the margins does nothing (screen)
- margins:93 DCH outside the margins does nothing (screen)
- margins:101 insert mode shifts only up to the right margin (screen)
- margins:109 IL moves only the columns between the margins (screen)
- margins:120 DL moves only the columns between the margins (screen)
- margins:131 line feed at the bottom margin scrolls only the margin columns (screen)
- margins:142 SU moves only the columns between the margins (screen)
- margins:160 DECIC inserts columns inside the margins (screen)
- margins:171 DECDC deletes columns inside the margins (screen)
- margins:182 forward index moves the cursor right (cursor)
- margins:189 forward index at the right margin scrolls the margins left (screen)
- margins:199 back index moves the cursor left (cursor)
- margins:206 back index at the left margin scrolls the margins right (screen)
- margins:228 DECSLRM needs the left margin below the right (cursor)
- margins:235 DECSLRM needs at least two columns (cursor)
- margins:242 origin mode with margins starts at the margins (cursor)
- margins:249 origin mode with margins clamps to them (cursor)
- origin:25 HVP counts from the top of the region (screen)
- origin:40 VPA counts from the top of the region (cursor)
- origin:78 a soft reset ends origin mode (screen)
- osc:48 a title can be pushed and popped (title)
- osc:56 the title stack holds several titles (title)
- osc:64 a window title push and pop works with Ps = 2 (title)
- parser:13 CAN cancels a string (title)
- parser:183 a sixel image leaves the cursor under it (cursor)
- rect-attributes:6 DECCARA in rectangle extent changes the rectangle (cell)
- rect-attributes:24 DECCARA in stream extent changes the stream (cell)
- rect-attributes:37 DECSACE 1 is the stream too (cell)
- rect-attributes:46 DECSACE 2 and back to the stream (cell)
- rect-attributes:53 DECSACE with another value changes nothing (cell)
- rect-attributes:61 DECCARA in stream extent on one line (cell)
- rect-attributes:84 DECCARA 0 clears bold, underline, blink and negative image (cell)
- rect-attributes:92 DECCARA without attributes is the same as 0 (cell)
- rect-attributes:122 DECCARA 1 sets bold (cell)
- rect-attributes:130 DECCARA 4 sets underline (cell)
- rect-attributes:146 DECCARA 7 sets inverse (cell)
- rect-attributes:169 DECCARA applies the attributes in order (cell)
- rect-attributes:176 DECCARA ignores an attribute it does not know (cell)
- rect-attributes:190 DECCARA changes blank cells in rectangle extent (cell)
- rect-attributes:215 DECCARA counts from the region in origin mode (cell)
- rect-attributes:225 DECCARA ignores the margins (cell)
- rect-attributes:237 DECCARA is cut to the screen (cell)
- rect-attributes:244 DECRARA reverses bold (cell)
- rect-attributes:254 DECRARA 4 reverses underline (cell)
- rect-attributes:268 DECRARA 7 reverses inverse (cell)
- rect-attributes:312 DECRARA in stream extent reverses the stream (cell)
- rect-attributes:323 DECRARA in rectangle extent reverses the rectangle (cell)
- rect-attributes:333 RIS selects the stream extent again (cell)
- rect-attributes:340 DECSTR leaves the extent as DECSACE set it (cell)
- rect:8 DECERA acts on the rectangle (screen)
- rect:22 DECERA without an area takes the whole page (screen)
- rect:62 DECERA is cut to the screen (screen)
- rect:76 DECERA counts from the region in origin mode (screen)
- rect:90 DECERA ignores the margins (screen)
- rect:104 DECERA leaves the cursor where it is (screen)
- rect:119 DECERA also erases protected characters (screen)
- rect:133 DECERA gives the cells the current background (cell)
- rect:141 DECERA through half a wide character blanks both halves (row)
- rect:147 DECSERA acts on the rectangle (screen)
- rect:161 DECSERA without an area takes the whole page (screen)
- rect:201 DECSERA is cut to the screen (screen)
- rect:215 DECSERA counts from the region in origin mode (screen)
- rect:229 DECSERA ignores the margins (screen)
- rect:243 DECSERA leaves the cursor where it is (screen)
- rect:258 DECSERA leaves protected characters (screen)
- rect:265 DECSERA without an area erases the unprotected characters of the page (screen)
- rect:272 DECFRA acts on the rectangle (screen)
- rect:286 DECFRA without an area takes the whole page (screen)
- rect:326 DECFRA is cut to the screen (screen)
- rect:340 DECFRA counts from the region in origin mode (screen)
- rect:354 DECFRA ignores the margins (screen)
- rect:368 DECFRA leaves the cursor where it is (screen)
- rect:383 DECFRA fills with the current rendition (cell)
- rect:391 DECFRA takes a character from the right-hand half (row)
- rect:449 DECFRA over half a wide character blanks the other half (row)
- rect:455 DECCRA copies a rectangle (screen)
- rect:469 DECCRA copies as if the source were read whole first when the areas overlap downwards (screen)
- rect:483 DECCRA copies as if the source were read whole first when the areas overlap upwards (screen)
- rect:497 DECCRA copies within a row to the right (screen)
- rect:511 DECCRA copies within a row to the left (screen)
- rect:525 DECCRA is cut at the screen's edge (screen)
- rect:539 DECCRA with the default source takes it from the corner (screen)
- rect:553 DECCRA with the default target copies to the corner (screen)
- rect:580 DECCRA with a source beyond the screen is cut to it (screen)
- rect:594 DECCRA counts both corners from the region in origin mode (screen)
- rect:608 DECCRA ignores the margins (screen)
- rect:666 DECCRA copies a whole wide character (row)
- rect:674 DECCRA onto the second half of a wide character blanks the first (row)
- rect:680 DECCRA onto the first half of a wide character blanks the second (row)
- reset:95 DECSTR returns the rendition to normal (cell)
- reset:102 DECSTR ends origin mode and removes the margins (screen)
- reset:122 DECSTR shows the cursor (cursor)
- reset:144 DECALN fills the screen with E (screen)
- reset:154 DECALN homes the cursor (cursor)
- reset:161 DECALN removes the margins (screen)
- resize:7 widening joins a wrapped row (screen)
- resize:17 narrowing keeps the text of a long row (lines)
- resize:26 narrowing keeps the rows on screen when they fit (screen)
- resize:54 narrowing and widening again restores the line (lines)
- resize:74 shrinking the height keeps the cursor row on screen (screen)
- resize:94 resizing keeps the rendition of the text (cell)
- resize:111 resizing resets the scrolling region (scrollback-len)
- resize:119 wide characters are not split by narrowing (lines)
- save-restore:27 DECRC restores the character set (screen)
- save-restore:43 DECRC restores the selective erase attribute (screen)
- save-restore:76 SCOSC and SCORC save and restore the position (cursor)
- save-restore:82 SCORC restores the rendition like DECRC (cell)
- save-restore:88 a soft reset forgets the saved cursor (screen)
- scroll:40 DECSTBM moves the cursor home (cursor)
- scroll:68 line feed in a region that starts at the top feeds the history (scrollback)
- scroll:124 reverse index above the region moves nowhere (screen)
- scroll:146 index keeps the column and scrolls at the bottom margin (screen)
- scroll:157 next line scrolls at the bottom margin and returns the carriage (screen)
- scroll:236 an invalid region is ignored (screen)
- scroll:287 an invalid DECSTBM leaves the cursor where it is (cursor)
- sgr:157 underline style 4:0 is none (cell)
- sgr:163 underline style 4:1 is single (cell)
- sgr:299 SGR 38:2 with a color space id sets a truecolor foreground (cell)
- sgr:305 SGR 48:2 with a color space id sets a truecolor background (cell)
- tabs:39 HTS sets a tab stop at the cursor (screen)
- tabs:46 HTS leaves the other stops (screen)
- tabs:53 TBC 0 clears the stop at the cursor (screen)
- tabs:60 TBC without a parameter is TBC 0 (screen)
- tabs:67 TBC 3 clears all stops (screen)
- tabs:89 CHT moves forward over tab stops (cursor)
- tabs:95 CHT defaults to one stop (cursor)
- tabs:101 CHT stops at the last column (cursor)
- tabs:107 CBT moves back to the previous stop (cursor)
- tabs:113 CBT by several stops stays on the first column (cursor)
- tabs:119 CBT from a stop goes to the one before (cursor)
- tabs:131 a stop set by HTS is used by CBT (cursor)
- tabs:152 CBT with every stop cleared stops at the first column (cursor)
- unicode:141 grapheme clusters: a ZWJ emoji sequence is one cluster two cells wide (cell)
- unicode:151 grapheme clusters: a skin tone modifier stays with its emoji (cell)
- unicode:160 grapheme clusters: two regional indicators are one flag (cell)
- unicode:177 grapheme clusters: VS16 makes a text symbol wide (cell)
- unicode:186 grapheme clusters: VS16 at the right margin moves the cluster to the next row (screen)
- unicode:203 grapheme clusters: VS15 makes a default-emoji character narrow (cell)
- unicode:212 grapheme clusters: VS15 gives the second cell of a wide emoji back (cell)
- unicode:222 grapheme clusters: VS15 at the right margin steps the cursor back (cell)
- unicode:242 grapheme clusters: VS15 then VS16 is not a sequence (cell)
- unicode:250 grapheme clusters: a spacing mark takes a cell (cell)
- unicode:259 grapheme clusters: several spacing marks and a nonspacing mark stay two cells (cell)
- unicode:274 grapheme clusters: a conjunct takes two cells (cell)
- unicode:282 grapheme clusters: a longer conjunct with a vowel sign stays two cells (cell)
- unicode:304 grapheme clusters: Khmer coeng makes a conjunct (cell)
- unicode:312 grapheme clusters: an Arabic number sign has a cell (cell)
- unicode:364 a lone continuation byte is one replacement character (screen)
- unicode:372 an impossible byte is one replacement character (screen)
- unicode:379 a truncated sequence is one replacement character (screen)
- unicode:386 a truncated four byte sequence is one replacement character (screen)
- unicode:393 an overlong encoding is a replacement character per byte (screen)
- unicode:401 an encoded surrogate is a replacement character per byte (screen)
- unicode:409 a replacement character takes one cell (cell)
- unicode:416 a character completed by a later feed, one character, then half of another (screen)
- unicode:426 a combining mark with nothing before it is kept on a blank (cell)
- wrap:26 a line feed clears the pending wrap (screen)
- wrap:52 cursor back clears the pending wrap (screen)
- wrap:60 cursor up clears the pending wrap (screen)
- wrap:69 autowrap off keeps the cursor in the last column and overwrites it (screen)
- wrap:110 a line feed in the pending state on the last row scrolls once (screen)
- wrap:142 a wide character that does not fit wraps as a whole (wrapped)
- wrap:183 a tab at the last column leaves a pending wrap (screen)
- wrap:193 a tab with 'more fix' on wraps first (screen)

### ghostty (200 cases)

- altscreen:131 mode 1049 again on the alternate screen changes nothing (screen)
- charsets:164 DECNRCM makes the national sets available (screen)
- charsets:172 the Dutch set replaces twelve positions of ASCII (screen)
- charsets:180 the Finnish set replaces twelve positions of ASCII (screen)
- charsets:188 the French set replaces twelve positions of ASCII (screen)
- charsets:196 the French Canadian set replaces twelve positions of ASCII (screen)
- charsets:204 the German set replaces twelve positions of ASCII (screen)
- charsets:212 the Italian set replaces twelve positions of ASCII (screen)
- charsets:220 the Norwegian and Danish set replaces twelve positions of ASCII (screen)
- charsets:228 the Portuguese set replaces twelve positions of ASCII (screen)
- charsets:236 the Spanish set replaces twelve positions of ASCII (screen)
- charsets:244 the Swedish set replaces twelve positions of ASCII (screen)
- charsets:252 the Swiss set replaces twelve positions of ASCII (screen)
- charsets:260 the alternate finals of the national sets (screen)
- charsets:268 a national set in G1 comes with SO (screen)
- charsets:276 a national set is not designated by a terminal that does not have it (row)
- charsets:287 DECSTR returns to the multinational set (reply)
- charsets:296 DEC Technical maps the mathematical symbols (screen)
- charsets:304 DEC Technical leaves its unused positions blank (screen)
- charsets:312 DEC Supplemental is the right half of the Multinational set (screen)
- charsets:320 the Latin-1 supplement is a 96-character set (screen)
- charsets:336 the user-preferred supplemental set starts as DEC Supplemental (screen)
- charsets:345 DECAUPSS assigns ISO Latin-1 as the user-preferred set (screen)
- checksum:15 DECRQCRA of one cell is its code point (reply)
- checksum:23 DECRQCRA of a rectangle is the sum of its cells (reply)
- checksum:31 DECRQCRA counts a blank cell as a space (reply)
- checksum:39 DECRQCRA counts an erased cell as a space (reply)
- checksum:47 DECRQCRA without an area sums the page (reply)
- checksum:55 DECRQCRA without parameters sums the page and answers for request 0 (reply)
- checksum:63 DECRQCRA echoes its request number (reply)
- checksum:71 DECRQCRA is cut to the screen (reply)
- checksum:79 DECRQCRA of an upside-down area sums nothing (reply)
- checksum:87 DECRQCRA counts from the region in origin mode (reply)
- checksum:95 DECRQCRA counts a wide character once (reply)
- checksum:103 DECRQCRA ignores combining marks (reply)
- checksum:111 DECRQCRA ignores the video attributes (reply)
- checksum:119 DECRQCRA sums modulo 65536 (reply)
- checksum:135 DECRQCRA of the first page is answered (reply)
- conformance-levels:4 DA1 names the VT500 family and its features by default (reply)
- conformance-levels:11 DECSCL 62 takes the terminal down to VT200 (reply)
- conformance-levels:18 DECSCL 61 is VT100 mode (reply)
- conformance-levels:25 DECRQM is a VT300 feature (reply)
- conformance-levels:34 left and right margins are a VT400 feature (screen)
- conformance-levels:47 DECSCL is a hard reset (screen)
- conformance-levels:58 DECRQSS reports the level and the control mode (reply)
- conformance-levels:67 DECSCL without its second parameter chooses 8-bit controls (reply)
- conformance-levels:80 S8C1T makes the replies use one-byte C1 controls (reply)
- conformance-levels:89 C1 bytes are controls only in 8-bit mode (screen)
- conformance-levels:100 8-bit IND, NEL, RI, HTS, DECID, SPA and EPA (screen)
- conformance-levels:115 an 8-bit ST ends a string (title)
- conformance-levels:131 DA1 lists what each family has (reply)
- cursor:316 NUL and DEL are ignored (screen)
- edit:199 DECIC inserts columns (screen)
- edit:211 DECDC deletes columns (screen)
- edit:260 SL shifts the columns left and blanks come in at the right (screen)
- edit:270 SR shifts the columns right and blanks come in at the left (screen)
- edit:279 SL and SR stay between the margins (screen)
- edit:293 SL erases with the background color (cell)
- erase:237 ED 2 keeps the text it clears as history (scrollback)
- graphics:93 an iTerm2 inline image leaves the cursor right of its last row (cursor)
- margins:25 the left margin stops cursor backward (cursor)
- margins:32 backspace at the left margin does nothing (cursor)
- margins:160 DECIC inserts columns inside the margins (screen)
- margins:171 DECDC deletes columns inside the margins (screen)
- margins:182 forward index moves the cursor right (cursor)
- margins:189 forward index at the right margin scrolls the margins left (screen)
- margins:199 back index moves the cursor left (cursor)
- margins:206 back index at the left margin scrolls the margins right (screen)
- modes:70 the VT510 modes a terminal emulator has no hardware for are kept and answered (reply)
- modes:81 DECHCCM and the ECMA-48 modes without a terminal to apply to are permanently reset (reply)
- modes:88 mode 41 makes a tab at the end of a row wrap first (reply)
- osc:48 a title can be pushed and popped (title)
- osc:56 the title stack holds several titles (title)
- osc:64 a window title push and pop works with Ps = 2 (title)
- osc:167 the colors a program sets are kept until it resets them, and a query answers them (reply)
- osc:175 a spec with # is the top bits of the byte and rgb: is scaled (reply)
- osc:181 the special colors are OSC 4 from 256 and OSC 5 (reply)
- osc:194 OSC 10 sets the colors after it too, and OSC 110 to 119 reset them one by one (reply)
- parser:183 a sixel image leaves the cursor under it (cursor)
- rect-attributes:6 DECCARA in rectangle extent changes the rectangle (cell)
- rect-attributes:24 DECCARA in stream extent changes the stream (cell)
- rect-attributes:37 DECSACE 1 is the stream too (cell)
- rect-attributes:46 DECSACE 2 and back to the stream (cell)
- rect-attributes:53 DECSACE with another value changes nothing (cell)
- rect-attributes:61 DECCARA in stream extent on one line (cell)
- rect-attributes:84 DECCARA 0 clears bold, underline, blink and negative image (cell)
- rect-attributes:92 DECCARA without attributes is the same as 0 (cell)
- rect-attributes:99 DECCARA 22 clears bold (cell)
- rect-attributes:105 DECCARA 24 clears underline (cell)
- rect-attributes:111 DECCARA 25 clears blink (cell)
- rect-attributes:116 DECCARA 27 clears inverse (cell)
- rect-attributes:122 DECCARA 1 sets bold (cell)
- rect-attributes:130 DECCARA 4 sets underline (cell)
- rect-attributes:138 DECCARA 5 sets blink (cell)
- rect-attributes:146 DECCARA 7 sets inverse (cell)
- rect-attributes:154 DECCARA 8 sets hidden (cell)
- rect-attributes:162 DECCARA 28 clears hidden (cell)
- rect-attributes:169 DECCARA applies the attributes in order (cell)
- rect-attributes:176 DECCARA ignores an attribute it does not know (cell)
- rect-attributes:183 DECCARA replaces another underline style (cell)
- rect-attributes:190 DECCARA changes blank cells in rectangle extent (cell)
- rect-attributes:215 DECCARA counts from the region in origin mode (cell)
- rect-attributes:225 DECCARA ignores the margins (cell)
- rect-attributes:237 DECCARA is cut to the screen (cell)
- rect-attributes:244 DECRARA reverses bold (cell)
- rect-attributes:254 DECRARA 4 reverses underline (cell)
- rect-attributes:261 DECRARA 5 reverses blink (cell)
- rect-attributes:268 DECRARA 7 reverses inverse (cell)
- rect-attributes:275 DECRARA 8 reverses hidden (cell)
- rect-attributes:282 DECRARA 0 reverses bold, underline, blink and negative image (cell)
- rect-attributes:290 DECRARA without attributes is the same as 0 (cell)
- rect-attributes:301 DECRARA reverses an underline style to none (cell)
- rect-attributes:312 DECRARA in stream extent reverses the stream (cell)
- rect-attributes:323 DECRARA in rectangle extent reverses the rectangle (cell)
- rect-attributes:333 RIS selects the stream extent again (cell)
- rect-attributes:340 DECSTR leaves the extent as DECSACE set it (cell)
- rect:8 DECERA acts on the rectangle (screen)
- rect:22 DECERA without an area takes the whole page (screen)
- rect:62 DECERA is cut to the screen (screen)
- rect:76 DECERA counts from the region in origin mode (screen)
- rect:90 DECERA ignores the margins (screen)
- rect:104 DECERA leaves the cursor where it is (screen)
- rect:119 DECERA also erases protected characters (screen)
- rect:133 DECERA gives the cells the current background (cell)
- rect:141 DECERA through half a wide character blanks both halves (row)
- rect:147 DECSERA acts on the rectangle (screen)
- rect:161 DECSERA without an area takes the whole page (screen)
- rect:201 DECSERA is cut to the screen (screen)
- rect:215 DECSERA counts from the region in origin mode (screen)
- rect:229 DECSERA ignores the margins (screen)
- rect:243 DECSERA leaves the cursor where it is (screen)
- rect:258 DECSERA leaves protected characters (screen)
- rect:265 DECSERA without an area erases the unprotected characters of the page (screen)
- rect:272 DECFRA acts on the rectangle (screen)
- rect:286 DECFRA without an area takes the whole page (screen)
- rect:326 DECFRA is cut to the screen (screen)
- rect:340 DECFRA counts from the region in origin mode (screen)
- rect:354 DECFRA ignores the margins (screen)
- rect:368 DECFRA leaves the cursor where it is (screen)
- rect:383 DECFRA fills with the current rendition (cell)
- rect:391 DECFRA takes a character from the right-hand half (row)
- rect:449 DECFRA over half a wide character blanks the other half (row)
- rect:455 DECCRA copies a rectangle (screen)
- rect:469 DECCRA copies as if the source were read whole first when the areas overlap downwards (screen)
- rect:483 DECCRA copies as if the source were read whole first when the areas overlap upwards (screen)
- rect:497 DECCRA copies within a row to the right (screen)
- rect:511 DECCRA copies within a row to the left (screen)
- rect:525 DECCRA is cut at the screen's edge (screen)
- rect:539 DECCRA with the default source takes it from the corner (screen)
- rect:553 DECCRA with the default target copies to the corner (screen)
- rect:580 DECCRA with a source beyond the screen is cut to it (screen)
- rect:594 DECCRA counts both corners from the region in origin mode (screen)
- rect:608 DECCRA ignores the margins (screen)
- rect:653 DECCRA copies the rendition and the link (cell)
- rect:666 DECCRA copies a whole wide character (row)
- rect:674 DECCRA onto the second half of a wide character blanks the first (row)
- rect:680 DECCRA onto the first half of a wide character blanks the second (row)
- reports:60 DSR ? 15 reports the printer status (reply)
- reports:67 DSR ? 25 reports the user defined key status (reply)
- reports:221 DECRQM of grapheme clustering (reply)
- reports:265 DECRQSS reports the character protection (reply)
- reports:328 DECRQSS reports the settings that are only kept (reply)
- reports:336 DECDSR 56, 62, 63 and 85 answer that the locator is a mouse, no macro space and no sessions (reply)
- reports:343 DECDSR 53 and 55 answer that a locator is available (reply)
- reports:350 DECXCPR and the keyboard report grow with the conformance level (reply)
- reports:361 DECRQPSR 1 reports the cursor information (reply)
- reports:368 DECCIR reports the position, the rendition and the protection (reply)
- reports:375 DECCIR reports origin mode, a pending wrap and a single shift (reply)
- reports:382 DECCIR reports the character sets, their sizes and what is invoked (reply)
- reports:389 DECRSPS puts the cursor information back (cursor)
- reports:406 DECRQPSR 2 reports the tab stops (reply)
- reports:415 DECRSPS puts the tab stops back (cursor)
- reports:426 DECRQTSR 1 reports the state (reply)
- reports:433 DECRSTS puts the state back (cursor)
- reports:450 DECRQDE reports the displayed extent (reply)
- reports:457 DECRQLP says there is no locator until DECELR (reply)
- reports:464 DECRQLP reports the locator once DECELR enabled it (reply)
- reports:471 DECELR 2 enables the locator for one report (reply)
- reports:478 DECELR 0 disables the locator (reply)
- reports:485 the locator replaces mouse tracking (reply)
- resize:87 the cursor stays inside after narrowing (cursor)
- save-restore:103 DECRC restores the hyperlink (cell)
- save-restore:109 DECRC brings back the state of no hyperlink (cell)
- scroll:31 the history keeps at most the lines it is allowed (scrollback)
- unicode:196 grapheme clusters: VS15 leaves an emoji that has no text presentation alone (cell)
- unicode:242 grapheme clusters: VS15 then VS16 is not a sequence (cell)
- unicode:297 grapheme clusters: a virama that is a spacing mark takes no cell (cell)
- unicode:350 grapheme clusters: a soft reset returns the mode to its default (reply)
- unicode:357 grapheme clusters: a full reset returns the mode to its default (reply)
- unicode:426 a combining mark with nothing before it is kept on a blank (cell)
- window:4 XTWINOPS reports the text area, the cell and the window state (reply)
- window:19 the title and the icon name are reported when the host allows it (reply)
- window:29 controls are taken out of a reported title (reply)
- window:37 the title stack keeps the title and the icon name together or apart (reply)
- window:47 a pop takes the slot that holds what it asks for (reply)
- window:55 the title modes read and report in hexadecimal (reply)
- window:65 a full reset clears the title modes and the stack (reply)
- window:79 the sizes in cells take effect at once when allowed (reply)
- window:97 a size past what the screen holds is cut (reply)
- wrap:193 a tab with 'more fix' on wraps first (screen)

