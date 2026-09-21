#!/bin/zsh
_gwt_pull() {
  source "$gwt_dir/utils/dir-gwt.sh"
  source "$gwt_dir/utils/ide-init.sh"
  local override_ide=""
  local skip_install=0
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
          return 16
        fi
        override_ide="$2"
        shift 2
        ;;
      --no-install)
        skip_install=1
        shift
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  if [[ ${#args[@]} -ne 1 ]]; then
    echo "gwt: unknown command: 'pull ${args[*]}'" >&2
    return 17
  fi

  local branch="${args[1]}"
  local dir_gwt
  dir_gwt=$(_gwt_get_dir_gwt) || return 18
  mkdir -p "$dir_gwt" || return 19
  git fetch origin "$branch" || return 20
  local dest="$dir_gwt/$branch"
  git worktree prune 2>/dev/null
  if git show-ref --verify --quiet "refs/heads/$branch"; then
    git worktree add "$dest" "$branch" || return 21
  else
    git worktree add -b "$branch" "$dest" "origin/$branch" || return 21
  fi
  cd "$dest" || return 22
  _gwt_init_ide "$override_ide" "$skip_install"
}
