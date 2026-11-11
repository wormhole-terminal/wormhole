#!/bin/sh
# a program that tracks the mouse and shows the bytes it is sent (the stand-in for a coding agent's own selection)
stty raw -echo
printf '\033[?1000h\033[?1006h'
printf 'selectable text of the program\r\n'
exec cat -v
