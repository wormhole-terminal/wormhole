# the OSC 133 marks zsh integration scripts send (kitty/iTerm2/Ghostty): A prompt start, B prompt end / command start, C output start, D finished
_wh_precmd() { print -Pn "\e]133;D;$?\a"; }
_wh_preexec() { print -Pn "\e]133;C\a"; }
precmd_functions+=(_wh_precmd); preexec_functions+=(_wh_preexec)
PS1=$'%{\e]133;A\a%}%~ %# %{\e]133;B\a%}'
