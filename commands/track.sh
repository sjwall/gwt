#!/bin/zsh
_gwt_track() {
  local target_repo=""
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local repos_file="$config_dir/repos"
  local lockfile="$config_dir/repos.lock"

  if [[ $# -eq 0 ]]; then
    if [[ -z "$main_repo" ]]; then
      echo "gwt: not inside a git repository" >&2
      return 36
    fi
    target_repo="$main_repo"
  elif [[ $# -eq 1 ]]; then
    local path_arg="$1"
    if [[ ! -d "$path_arg" ]]; then
      echo "gwt: not a git repository: '$path_arg'" >&2
      return 38
    fi
    target_repo=$(git -C "$path_arg" worktree list --porcelain 2>/dev/null | head -n 1 | sed 's/^worktree //')
    if [[ -z "$target_repo" ]]; then
      echo "gwt: not a git repository: '$path_arg'" >&2
      return 38
    fi
  else
    echo "gwt: unknown command: 'track $*'" >&2
    return 37
  fi

  source "${0:A:h:h}/utils/file-lock.sh"
  _gwt_acquire_lock "$lockfile" || return 1
  
  mkdir -p "$config_dir"
  if [[ ! -f "$repos_file" ]] || ! grep -Fxq "$target_repo" "$repos_file" 2>/dev/null; then
    echo "$target_repo" >> "$repos_file"
  fi
  echo "$target_repo"
  
  _gwt_release_lock "$lockfile"
  return 0
}
