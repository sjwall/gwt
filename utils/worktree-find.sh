#!/bin/zsh
_gwt_find_worktrees() {
  local query="$1"
  shift
  local repo_list=("$@")
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
