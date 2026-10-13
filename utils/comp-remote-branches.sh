#!/bin/zsh
# Completion function for remote branches

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
