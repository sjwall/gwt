#!/bin/zsh
# Completion function for branches (local and remote)

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
