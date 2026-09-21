#!/bin/zsh
_gwt_launch_ide() {
  source "$gwt_dir/utils/ide-get.sh"
  local override_ide="$1"
  local ide_cmd="${override_ide:-$(_gwt_get_ide)}"
  if [[ "${ide_cmd:l}" == "none" ]]; then
    return 0
  fi
  eval "$ide_cmd"
}
