# Command lines to shoot the same states again (scratch tree: scratchpad/s2-terminal/drive; scenes made by the python block in the transcript's header: menu, scroll, fold2, rules, tick).
# start (Swift): WORMHOLE_FONT_DIR=crates/wormhole-core/assets/fonts/JetBrainsMono ZDOTDIR=<empty dir> WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-s2-terminal-x WORMHOLE_SHELL=<feed.sh> WORMHOLE_AUTH=cancel|allow WORMHOLE_DEVCTL=<ctl> <bin> --background &
# start (GPUI): scripts/devctl-instance.sh --shell <feed.sh> <fresh scratch dir> <repo>   (a reused dir restores the old panes) ; then: onboarding close, resize 1320 820, reduce-transparency on, appearance dark
# Swift setup: size 1320x820 / appearance dark / reduce-transparency on / panel      (panel closes the right column)
# T15:  echo menu > swift.fifo ; right-click 468 51 (url) | 409 105 (block) | 409 69 (plain) ; menu-pick <title>   (GPUI: right-click, wait 1 s, menu, menu-pick; positions 466 51 / 405 105 / 405 69)
# T13:  hover 900 400 ; hover 700 300 ; hover 420 100 ; hover 410 105 ; wait-idle 800 ; window-shot x.png      then python3 measure_hover.py a.png b.png
# T11:  echo scroll > swift.fifo ; scroll 30 ; hover 1305 300 ; bounds 1307 300 2 ; window-shot x.png         (GPUI: scroller overlay|legacy ; scroll 700 300 -1800 ; frames 3 ; shot n)
# T25:  echo fold2 > p.fifo ; scroll 30 ; frames 3 ; probes ; window-shot x.png ; hover 450 87 ; click 450 87 ; scroll 12 ; probes     (find: find needle ; echo tick > p.fifo ; scroll 30 ; frames 5)
# T24:  echo rules > p.fifo ; link 1 8 ; hover-cmd 430 69 ; hover-cmd 434 69 ; window-shot x.png
# T23:  hover 900 400 ; hover 520 51 ; hover 528 51 ; window-shot x.png ; hover 440 87 ; hover 451 87 ; window-shot y.png
