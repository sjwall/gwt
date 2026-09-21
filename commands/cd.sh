#!/bin/zsh
_gwt_cd() {
  if [[ $# -ne 1 ]]; then
    echo "gwt: unknown command: 'cd $*'" >&2
    return 1
  fi

  local query="$1"
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local repos_file="$config_dir/repos"

  local matches=()
  local exact_matches=()

  _gwt_find_worktrees() {
    local repo_list=("$@")
    matches=()
    exact_matches=()
    local repo wt_list wt_path wt_name
    for repo in "${repo_list[@]}"; do
      [[ -z "$repo" || ! -d "$repo" ]] && continue
      wt_list=$(git -C "$repo" worktree list --porcelain 2>/dev/null | grep "^worktree " | sed "s/^worktree //")
      while IFS= read -r wt_path || [[ -n "$wt_path" ]]; do
        [[ -z "$wt_path" ]] && continue
        wt_name=$(basename "$wt_path")
        if [[ "${wt_name:l}" == "${query:l}" ]]; then
          exact_matches+=("$wt_path")
        fi
        if [[ "${wt_name:l}" == *"${query:l}"* ]]; then
          matches+=("$wt_path")
        fi
      done <<< "$wt_list"
    done
  }

  if [[ -n "$main_repo" ]]; then
    _gwt_find_worktrees "$main_repo"
    if [[ ${#exact_matches[@]} -eq 1 ]]; then
      cd "${exact_matches[1]}"
      echo "${exact_matches[1]}"
      return 0
    elif [[ ${#matches[@]} -eq 1 ]]; then
      cd "${matches[1]}"
      echo "${matches[1]}"
      return 0
    elif [[ ${#matches[@]} -gt 1 ]]; then
      echo "gwt: multiple worktrees match '$query':" >&2
      for m in "${matches[@]}"; do
        echo "  $m" >&2
      done
      return 2
    fi
  fi

  if [[ -f "$repos_file" ]]; then
    local other_repos=()
    local r
    while IFS= read -r r || [[ -n "$r" ]]; do
      [[ -z "$r" || "$r" == "$main_repo" ]] && continue
      other_repos+=("$r")
    done < "$repos_file"

    if [[ ${#other_repos[@]} -gt 0 ]]; then
      _gwt_find_worktrees "${other_repos[@]}"
      if [[ ${#exact_matches[@]} -eq 1 ]]; then
        cd "${exact_matches[1]}"
        echo "${exact_matches[1]}"
        return 0
      elif [[ ${#matches[@]} -eq 1 ]]; then
        cd "${matches[1]}"
        echo "${matches[1]}"
        return 0
      elif [[ ${#matches[@]} -gt 1 ]]; then
        echo "gwt: multiple worktrees match '$query':" >&2
        for m in "${matches[@]}"; do
          echo "  $m" >&2
        done
        return 3
      fi
    fi
  fi

  echo "gwt: no matching worktree found for '$query'" >&2
  return 4
}
