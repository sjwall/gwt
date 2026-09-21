#!/bin/zsh
_gwt_agent() {
  source "$gwt_dir/commands/cd.sh"
  source "$gwt_dir/utils/agent-launch.sh"
  local override_agent=""
  local args=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --agent=*)
        override_agent="${1#--agent=}"
        shift
        ;;
      --agent)
        if [[ $# -lt 2 ]]; then
          echo "gwt: --agent requires an argument" >&2
          return 44
        fi
        override_agent="$2"
        shift 2
        ;;
      --ide=*)
        shift
        ;;
      --ide)
        if [[ $# -lt 2 ]]; then
          echo "gwt: --ide requires an argument" >&2
          return 11
        fi
        shift 2
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  if [[ ${#args[@]} -ne 1 ]]; then
    echo "gwt: unknown command: agent '${args[*]}'" >&2
    return 45
  fi

  local query="${args[1]}"
  _gwt_cd "$query" || return $?
  _gwt_launch_agent "$override_agent"
}
