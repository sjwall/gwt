#!/bin/zsh
_gwt_track_explore() {
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local repos_file="$config_dir/repos"
  local lockfile="$config_dir/repos.lock"
  local root="${PWD:A}"
  local -a pending=()
  local -a repos=()
  local -A visited=()
  local -A tracked=()
  local dir child target_repo current_repo

  if [[ $# -gt 0 ]]; then
    echo "gwt: track --explore does not accept arguments" >&2
    return 37
  fi

  current_repo=$(git -C "$PWD" worktree list --porcelain 2>/dev/null | head -n 1 | sed 's/^worktree //')
  if [[ -n "$current_repo" ]]; then
    echo "gwt: current directory is already a git repository: '$current_repo'" >&2
    return 60
  fi

  setopt local_options glob_dots

  if [[ ! -r "$root" || ! -x "$root" ]]; then
    echo "gwt: cannot read directory: '$PWD'" >&2
    return 0
  fi

  for child in "$root"/*(N/); do
    pending+=("$child")
  done

  while (( ${#pending[@]} > 0 )); do
    dir="${pending[1]}"
    pending[1]=()

    local resolved="${dir:A}"
    [[ -n "${visited[$resolved]}" ]] && continue
    visited[$resolved]=1

    [[ "${dir:t}" == ".git" ]] && continue

    if [[ -d "$dir/.git" ]]; then
      repos+=("$dir")
      continue
    fi

    if [[ ! -r "$dir" || ! -x "$dir" ]]; then
      echo "gwt: cannot read directory: '$dir'" >&2
      continue
    fi

    for child in "$dir"/*(N/); do
      pending+=("$child")
    done
  done

  if (( ${#repos[@]} == 0 )); then
    echo "gwt: no git repositories found in '$PWD'" >&2
    return 0
  fi

  repos=("${(@o)repos}")

  source "$gwt_dir/utils/file-lock.sh"
  _gwt_acquire_lock "$lockfile" || return 1

  {
    mkdir -p "$config_dir"
    for dir in "${repos[@]}"; do
      target_repo=$(git -C "$dir" worktree list --porcelain 2>/dev/null | head -n 1 | sed 's/^worktree //')
      if [[ -z "$target_repo" ]]; then
        echo "gwt: not a git repository: '$dir'" >&2
        continue
      fi
      [[ -n "${tracked[$target_repo]}" ]] && continue
      tracked[$target_repo]=1
      if [[ ! -f "$repos_file" ]] || ! grep -Fxq "$target_repo" "$repos_file" 2>/dev/null; then
        echo "$target_repo" >> "$repos_file"
      fi
      echo "$target_repo"
    done
  } always {
    _gwt_release_lock "$lockfile"
  }
  return 0
}
