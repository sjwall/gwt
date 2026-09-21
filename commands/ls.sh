#!/bin/zsh
_gwt_ls() {
  local flags=()
  local args=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -*)
        flags+=("$1")
        shift
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local repos_file="$config_dir/repos"
  local repo_list=()

  if [[ -n "$main_repo" ]]; then
    repo_list+=("$main_repo")
  fi

  if [[ -f "$repos_file" ]]; then
    local r
    while IFS= read -r r || [[ -n "$r" ]]; do
      [[ -z "$r" ]] && continue
      repo_list+=("$r")
    done < "$repos_file"
  fi

  if [[ ${#args[@]} -eq 0 ]]; then
    local -A seen
    local repo
    for repo in "${repo_list[@]}"; do
      [[ -z "$repo" || ! -d "$repo" ]] && continue
      [[ -n "${seen[$repo]}" ]] && continue
      seen[$repo]=1
      git -C "$repo" rev-parse --git-dir >/dev/null 2>&1 || continue
      git -C "$repo" worktree list "${flags[@]}"
    done
    return 0
  elif [[ ${#args[@]} -gt 1 ]]; then
    echo "gwt: unknown command: 'list ${args[*]}'" >&2
    return 39
  fi

  local query="${args[1]}"
  local -A seen
  local repo repo_name
  local exact_matches=()
  local matches=()
  local path_matches=()

  for repo in "${repo_list[@]}"; do
    [[ -z "$repo" || ! -d "$repo" ]] && continue
    [[ -n "${seen[$repo]}" ]] && continue
    seen[$repo]=1
    git -C "$repo" rev-parse --git-dir >/dev/null 2>&1 || continue

    repo_name=$(basename "$repo")
    if [[ "${repo_name:l}" == "${query:l}" ]]; then
      exact_matches+=("$repo")
    fi
    if [[ "${repo_name:l}" == *"${query:l}"* ]]; then
      matches+=("$repo")
    elif [[ "${repo:l}" == *"${query:l}"* ]]; then
      path_matches+=("$repo")
    fi
  done

  local target_repos=()
  if [[ ${#exact_matches[@]} -eq 1 ]]; then
    target_repos=("${exact_matches[1]}")
  elif [[ ${#exact_matches[@]} -gt 1 ]]; then
    echo "gwt: multiple repositories match '$query':" >&2
    for m in "${exact_matches[@]}"; do
      echo "  $m" >&2
    done
    return 40
  elif [[ ${#matches[@]} -eq 1 ]]; then
    target_repos=("${matches[1]}")
  elif [[ ${#matches[@]} -gt 1 ]]; then
    echo "gwt: multiple repositories match '$query':" >&2
    for m in "${matches[@]}"; do
      echo "  $m" >&2
    done
    return 41
  elif [[ ${#path_matches[@]} -eq 1 ]]; then
    target_repos=("${path_matches[1]}")
  elif [[ ${#path_matches[@]} -gt 1 ]]; then
    echo "gwt: multiple repositories match '$query':" >&2
    for m in "${path_matches[@]}"; do
      echo "  $m" >&2
    done
    return 42
  else
    echo "gwt: no matching repository found for '$query'" >&2
    return 43
  fi

  for repo in "${target_repos[@]}"; do
    git -C "$repo" worktree list "${flags[@]}"
  done
}
