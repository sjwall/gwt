#!/bin/zsh
_gwt_main() {
  local query="$1"
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local repos_file="$config_dir/repos"

  if [[ $# -eq 0 ]]; then
    if [[ -n "$main_repo" ]]; then
      cd "$main_repo" || return $?
      echo "$main_repo"
      return 0
    fi

    local options=()
    local -A seen
    if [[ -f "$repos_file" ]]; then
      local r
      while IFS= read -r r || [[ -n "$r" ]]; do
        [[ -z "$r" || ! -d "$r" ]] && continue
        [[ -n "${seen[$r]}" ]] && continue
        git -C "$r" rev-parse --git-dir >/dev/null 2>&1 || continue
        seen[$r]=1
        options+=("$r")
      done < "$repos_file"
    fi

    if [[ ${#options[@]} -eq 0 ]]; then
      echo "gwt: no matching repository found" >&2
      return 5
    elif [[ ${#options[@]} -eq 1 ]]; then
      cd "${options[1]}" || return $?
      echo "${options[1]}"
      return 0
    fi

    echo "Select a repository to switch to:" >&2
    local i
    for (( i = 1; i <= ${#options[@]}; i++ )); do
      echo "  $i) ${options[i]}" >&2
    done

    local choice
    read -r "choice?Enter selection [1-${#options[@]}]: "
    if [[ -z "$choice" ]]; then
      echo "gwt: no repository selected" >&2
      return 5
    fi

    local selected=""
    if [[ "$choice" =~ ^[0-9]+$ ]] && (( choice >= 1 && choice <= ${#options[@]} )); then
      selected="${options[choice]}"
    else
      local matches=()
      local opt opt_name
      for opt in "${options[@]}"; do
        opt_name=$(basename "$opt")
        if [[ "${opt_name:l}" == "${choice:l}" || "${opt:l}" == "${choice:l}" ]]; then
          matches+=("$opt")
        fi
      done
      if [[ ${#matches[@]} -eq 1 ]]; then
        selected="${matches[1]}"
      elif [[ ${#matches[@]} -gt 1 ]]; then
        echo "gwt: multiple repositories match '$choice':" >&2
        local m
        for m in "${matches[@]}"; do
          echo "  $m" >&2
        done
        return 7
      fi
    fi

    if [[ -z "$selected" ]]; then
      echo "gwt: invalid selection" >&2
      return 5
    fi

    cd "$selected" || return $?
    echo "$selected"
    return 0
  elif [[ $# -gt 1 ]]; then
    echo "gwt: unknown command: 'main $*'" >&2
    return 6
  fi

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

  local -A seen
  local repo repo_name
  local exact_matches=()
  local matches=()
  local path_matches=()

  for repo in "${repo_list[@]}"; do
    [[ -z "$repo" || ! -d "$repo" ]] && continue
    [[ -n "${seen[$repo]}" ]] && continue
    seen[$repo]=1

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

  if [[ ${#exact_matches[@]} -eq 1 ]]; then
    cd "${exact_matches[1]}"
    echo "${exact_matches[1]}"
    return 0
  elif [[ ${#exact_matches[@]} -gt 1 ]]; then
    echo "gwt: multiple repositories match '$query':" >&2
    for m in "${exact_matches[@]}"; do
      echo "  $m" >&2
    done
    return 7
  elif [[ ${#matches[@]} -eq 1 ]]; then
    cd "${matches[1]}"
    echo "${matches[1]}"
    return 0
  elif [[ ${#matches[@]} -gt 1 ]]; then
    echo "gwt: multiple repositories match '$query':" >&2
    for m in "${matches[@]}"; do
      echo "  $m" >&2
    done
    return 8
  elif [[ ${#path_matches[@]} -eq 1 ]]; then
    cd "${path_matches[1]}"
    echo "${path_matches[1]}"
    return 0
  elif [[ ${#path_matches[@]} -gt 1 ]]; then
    echo "gwt: multiple repositories match '$query':" >&2
    for m in "${path_matches[@]}"; do
      echo "  $m" >&2
    done
    return 9
  fi

  echo "gwt: no matching repository found for '$query'" >&2
  return 10
}

_gwt_main_ide() {
  source "$gwt_dir/utils/ide-launch.sh"
  local override_ide=""
  local args=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --ide=*)
        override_ide="${1#--ide=}"
        shift
        ;;
      --ide)
        if [[ $# -lt 2 ]]; then
          echo "gwt: --ide requires an argument" >&2
          return 11
        fi
        override_ide="$2"
        shift 2
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  if [[ ${#args[@]} -gt 1 ]]; then
    echo "gwt: unknown command: 'M ${args[*]}'" >&2
    return 6
  fi

  _gwt_main "${args[@]}" || return $?
  _gwt_launch_ide "$override_ide"
}
