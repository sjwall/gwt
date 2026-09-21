#!/bin/zsh
_gwt_switch() {
  source "$gwt_dir/commands/cd.sh"
  source "$gwt_dir/utils/ide-launch.sh"
  local override_ide=""
  local args=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --ide=*)
        override_ide="${1#--ide=}"
        shift
        ;;
      --ide)
        if [[ $# -lt 2 ]]; then
          echo "gwt: --ide requires an argument" >&2
          return 11
        fi
        override_ide="$2"
        shift 2
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  if [[ ${#args[@]} -ne 1 ]]; then
    echo "gwt: unknown command: switch '${args[*]}'" >&2
    return 12
  fi

  local query="${args[1]}"
  _gwt_cd "$query" || return $?
  _gwt_launch_ide "$override_ide"
}
