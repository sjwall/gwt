#!/bin/zsh
# Completion function for repository selection

_gwt_comp_repositories() {
  (( $+functions[_describe] )) || return 0
  local -a repo_entries
  local -A seen_repos
  local main_repo repo repo_name
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local repos_file="$config_dir/repos"
  local -a repo_list

  main_repo=$(git worktree list --porcelain 2>/dev/null | head -n 1 | sed 's/^worktree //')
  if [[ -n "$main_repo" ]]; then
    repo_list+=("$main_repo")
  fi

  if [[ -f "$repos_file" ]]; then
    local r
    while IFS= read -r r || [[ -n "$r" ]]; do
      [[ -z "$r" || ! -d "$r" ]] && continue
      repo_list+=("$r")
    done < "$repos_file"
  fi

  for repo in "${repo_list[@]}"; do
    [[ -n "${seen_repos[$repo]}" ]] && continue
    seen_repos[$repo]=1
    repo_name=$(basename "$repo")
    repo_entries+=("${repo_name//:/\\:}:${repo//:/\\:}")
  done

  if [[ ${#repo_entries[@]} -gt 0 ]]; then
    _describe -t repositories 'repository' repo_entries
  fi
}
