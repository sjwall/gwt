#!/bin/zsh
# Completion for gwt clone command

_gwt_comp_clone() {
  (( $+functions[_arguments] )) || return 0
  
  _arguments \
    '--ide=[Override IDE to launch]:IDE:_gwt_comp_ides' \
    '(-a --agent)'{-a,--agent}'[Launch agent instead of IDE]::agent command: ' \
    '--no-install[Skip dependency installation]' \
    '*::git clone arguments: ' && return 0
}
