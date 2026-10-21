#!/bin/zsh
_gwt_track() {
  local flags=()
  local args=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -*)
        flags+=("$1")
        shift
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  if [[ ${#flags[@]} -eq 0 ]]; then
    source "$gwt_dir/commands/track-add.sh"
    _gwt_track_add $args
  else
    echo "gwt: unknown command: track ${flags[*]} ${args[*]}" >&2
    return 59
  fi
}
