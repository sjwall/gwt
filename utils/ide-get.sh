#!/bin/zsh
_gwt_get_ide() {
  source "$gwt_dir/utils/config-get.sh"
  local ide
  ide=$(_gwt_get_config "ide")
  if [[ -n "$ide" ]]; then
    echo "$ide"
  elif [[ -n "$GWT_IDE" ]]; then
    echo "$GWT_IDE"
  else
    echo "nvim"
  fi
}
