#!/bin/zsh
_gwt_remove() {
  local flags=()
  local targets=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -f|--force|-force)
        flags+=(--force)
        shift
        ;;
      *)
        targets+=("$1")
        shift
        ;;
    esac
  done

  if [[ -z "$main_repo" ]]; then
    git worktree remove "${flags[@]}" "${targets[@]}"
    return $?
  fi

  local current_wt=$(git rev-parse --show-toplevel 2>/dev/null)
  local is_linked_worktree=0
  if [[ -n "$current_wt" && "$current_wt" != "$main_repo" ]] || [[ "$(git rev-parse --git-dir 2>/dev/null)" != "$(git rev-parse --git-common-dir 2>/dev/null)" ]]; then
    is_linked_worktree=1
  fi

  if [[ ${#targets[@]} -eq 0 ]]; then
    if [[ $is_linked_worktree -eq 1 ]]; then
      targets=("$current_wt")
    else
      echo "gwt: cannot remove main repository; please specify a worktree" >&2
      return 13
    fi
  else
    local resolved_targets=()
    local t
    for t in "${targets[@]}"; do
      if [[ -d "$t" ]]; then
        resolved_targets+=("$(cd "$t" 2>/dev/null && pwd -P)")
      else
        resolved_targets+=("$t")
      fi
    done
    targets=("${resolved_targets[@]}")
  fi

  local orig_pwd="$PWD"
  if [[ $is_linked_worktree -eq 1 ]] || [[ "$PWD" != "$main_repo" ]]; then
    cd "$main_repo" || return 14
  fi

  if ! git worktree remove "${flags[@]}" "${targets[@]}"; then
    if [[ "$PWD" != "$orig_pwd" ]]; then
      cd "$orig_pwd" 2>/dev/null
    fi
    return 15
  fi
}
