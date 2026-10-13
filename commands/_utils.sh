#!/bin/zsh
# Shared completion functions for gwt

_gwt_comp_ides() {
  (( $+functions[_describe] )) || return 0
  local -a ides
  ides=(
    'nvim:Neovim'
    'vim:Vim'
    'code:Visual Studio Code'
    'cursor:Cursor'
    'idea:IntelliJ IDEA'
    'zed:Zed'
    'emacs:Emacs'
    'nano:Nano'
    'subl:Sublime Text'
    'webstorm:WebStorm'
    'none:Do not launch an IDE'
  )
  _describe -t ides 'IDE' ides
}

_gwt_comp_worktrees() {
  (( $+functions[_describe] )) || return 0
  local -a wt_entries
  local -A seen_paths
  local main_repo repo wt_list wt_path wt_name
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
    wt_list=$(git -C "$repo" worktree list --porcelain 2>/dev/null | grep "^worktree " | sed "s/^worktree //")
    while IFS= read -r wt_path || [[ -n "$wt_path" ]]; do
      [[ -z "$wt_path" || -n "${seen_paths[$wt_path]}" ]] && continue
      seen_paths[$wt_path]=1
      wt_name=$(basename "$wt_path")
      wt_entries+=("${wt_name//:/\\:}:${wt_path//:/\\:}")
    done <<< "$wt_list"
  done

  if [[ ${#wt_entries[@]} -gt 0 ]]; then
    _describe -t worktrees 'worktree' wt_entries
  fi
}

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

_gwt_comp_remote_branches() {
  (( $+functions[_describe] )) || return 0
  local -a branches
  local b
  for b in ${(f)"$(git for-each-ref --format='%(refname:short)' refs/remotes/origin/ 2>/dev/null)"}; do
    [[ -z "$b" || "$b" == "origin/HEAD"* ]] && continue
    branches+=("${b#origin/}")
  done
  if [[ ${#branches[@]} -gt 0 ]]; then
    _describe -t remote-branches 'remote branch' branches
  fi
}

_gwt_comp_branches() {
  (( $+functions[_describe] )) || return 0
  local -a branches
  local b
  for b in ${(f)"$(git for-each-ref --format='%(refname:short)' refs/heads/ refs/remotes/origin/ 2>/dev/null)"}; do
    [[ -z "$b" || "$b" == "origin/HEAD"* ]] && continue
    branches+=("${b#origin/}")
  done
  typeset -U branches
  if [[ ${#branches[@]} -gt 0 ]]; then
    _describe -t branches 'branch' branches
  fi
}
