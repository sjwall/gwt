#!/bin/zsh
_gwt_migrate() {
  source "$gwt_dir/utils/gwt-dir-get.sh"
  local dry_run=0
  local force_flag=""
  local args=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -n|--dry-run)
        dry_run=1
        shift
        ;;
      -f|--force|-force)
        force_flag="--force"
        shift
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  if [[ ${#args[@]} -gt 0 ]]; then
    echo "gwt: unknown command: 'migrate ${args[*]}'" >&2
    return 49
  fi

  if [[ -z "$main_repo" ]]; then
    echo "gwt: not inside a git repository" >&2
    return 48
  fi

  local dir_gwt
  dir_gwt=$(_gwt_get_dir_gwt) || return 50
  if [[ -z "$dir_gwt" ]]; then
    return 50
  fi

  local target_base
  if [[ "$dir_gwt" == /* ]]; then
    target_base="$dir_gwt"
  else
    local repo_parent
    repo_parent="$(cd "$main_repo/.." 2>/dev/null && pwd -P)"
    target_base="${repo_parent}/${dir_gwt#../}"
  fi

  if [[ -d "$target_base" ]]; then
    target_base="$(cd "$target_base" 2>/dev/null && pwd -P || echo "$target_base")"
  else
    local base_parent
    base_parent="$(cd "$(dirname "$target_base")" 2>/dev/null && pwd -P || echo "$(dirname "$target_base")")"
    target_base="${base_parent}/$(basename "$target_base")"
  fi
  target_base="${target_base%/}"

  git -C "$main_repo" worktree prune 2>/dev/null
  local wt_output
  wt_output=$(git -C "$main_repo" worktree list --porcelain 2>/dev/null)

  local -a wt_paths=()
  local -a wt_branches=()
  local cur_wt=""
  local cur_branch=""
  local line

  while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    if [[ "$line" == "worktree "* ]]; then
      if [[ -n "$cur_wt" ]]; then
        wt_paths+=("$cur_wt")
        wt_branches+=("$cur_branch")
        cur_wt=""
        cur_branch=""
      fi
      cur_wt="${line#worktree }"
    elif [[ "$line" == "branch "* ]]; then
      cur_branch="${line#branch }"
      cur_branch="${cur_branch#refs/heads/}"
    elif [[ "$line" == "detached" ]]; then
      cur_branch=""
    elif [[ -z "$line" ]]; then
      if [[ -n "$cur_wt" ]]; then
        wt_paths+=("$cur_wt")
        wt_branches+=("$cur_branch")
        cur_wt=""
        cur_branch=""
      fi
    fi
  done <<< "$wt_output"

  if [[ -n "$cur_wt" ]]; then
    wt_paths+=("$cur_wt")
    wt_branches+=("$cur_branch")
  fi

  local can_main
  can_main="$(cd "$main_repo" 2>/dev/null && pwd -P || echo "$main_repo")"

  local -a to_move_src=()
  local -a to_move_dest=()
  local i

  for (( i = 1; i <= ${#wt_paths[@]}; i++ )); do
    local p="${wt_paths[i]}"
    local can_wt
    can_wt="$(cd "$p" 2>/dev/null && pwd -P || echo "$p")"
    [[ "$can_wt" == "$can_main" ]] && continue
    [[ ! -d "$can_wt" ]] && continue

    local branch_name="${wt_branches[i]}"
    local expected_dest
    if [[ -n "$branch_name" ]]; then
      expected_dest="$target_base/$branch_name"
    else
      expected_dest="$target_base/$(basename "$can_wt")"
    fi

    local can_expected
    if [[ -d "$expected_dest" ]]; then
      can_expected="$(cd "$expected_dest" 2>/dev/null && pwd -P || echo "$expected_dest")"
    else
      local exp_parent
      exp_parent="$(cd "$(dirname "$expected_dest")" 2>/dev/null && pwd -P || echo "$(dirname "$expected_dest")")"
      can_expected="${exp_parent}/$(basename "$expected_dest")"
    fi

    if [[ "$can_wt" != "$can_expected" ]]; then
      to_move_src+=("$can_wt")
      to_move_dest+=("$expected_dest")
    fi
  done

  if [[ ${#to_move_src[@]} -eq 0 ]]; then
    echo "gwt: all worktrees match the expected path pattern"
    return 0
  fi

  if [[ $dry_run -eq 1 ]]; then
    for (( i = 1; i <= ${#to_move_src[@]}; i++ )); do
      echo "Would move '${to_move_src[i]}' to '${to_move_dest[i]}'"
    done
    return 0
  fi

  local orig_pwd="$PWD"
  local new_pwd=""

  for (( i = 1; i <= ${#to_move_src[@]}; i++ )); do
    local src="${to_move_src[i]}"
    local dest="${to_move_dest[i]}"
    echo "Moving '$src' to '$dest'..."
    mkdir -p "$(dirname "$dest")"
    local move_cmd=(git -C "$main_repo" worktree move)
    [[ -n "$force_flag" ]] && move_cmd+=("$force_flag")
    move_cmd+=("$src" "$dest")
    if ! "${move_cmd[@]}"; then
      echo "gwt: failed to move worktree '$src' to '$dest'" >&2
      return 51
    fi

    if [[ "$orig_pwd" == "$src" ]]; then
      new_pwd="$dest"
    elif [[ "$orig_pwd" == "$src"/* ]]; then
      local rel_sub="${orig_pwd#$src/}"
      new_pwd="$dest/$rel_sub"
    fi
  done

  if [[ -n "$new_pwd" && -d "$new_pwd" ]]; then
    cd "$new_pwd"
  elif [[ -n "$new_pwd" ]]; then
    cd "$dest"
  fi

  return 0
}
