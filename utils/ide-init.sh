#!/bin/zsh
_gwt_init_ide() {
  source "$gwt_dir/utils/ide-launch.sh"
  local override_ide="$1"
  local skip_install="${2:-0}"
  if [[ "$skip_install" -eq 0 ]]; then
    if [ -f yarn.lock ]; then
      yarn
    fi
  fi
  _gwt_launch_ide "$override_ide"
}
