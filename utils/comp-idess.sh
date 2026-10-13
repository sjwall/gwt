#!/bin/zsh
# Completion function for IDE selection

_gwt_comp_ides() {
  (( $+functions[_describe] )) || return 0
  local -a ides
  ides=(
    'nvim:Neovim'
    'vim:Vim'
    'code:Visual Studio Code'
    'cursor:Cursor'
    'idea:IntelliJ IDEA'
    'zed:Zed'
    'emacs:Emacs'
    'nano:Nano'
    'subl:Sublime Text'
    'webstorm:WebStorm'
    'none:Do not launch an IDE'
  )
  _describe -t ides 'IDE' ides
}
