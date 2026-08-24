#!/bin/zsh
# git worktree helper
#  gwt NAME            create worktree ../gwt-<dir-name>/NAME, cd, yarn, nvim
#  gwt pull NAME       fetch origin/NAME, create tracking worktree, cd, yarn, nvim
#  gwt p NAME          as above
#  gwt cd NAME         cd to worktree matching NAME
#  gwt remove NAME     remove worktree ../gwt-<dir-name>/NAME
#  gwt rm NAME         as above
#  gwt rm -force NAME  as above but with force
#  gwt rm -f NAME      as above
unalias gwt 2>/dev/null  #omz git plugin defines `gwt` alias; remove so func wins
gwt() {
  local dir_name=$(basename "$PWD")
  local dir_gwt="../gwt-${dir_name}"

  local main_repo=$(git worktree list --porcelain 2>/dev/null | head -n 1 | sed 's/^worktree //')
  if [[ -n "$main_repo" ]]; then
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
    local repos_file="$config_dir/repos"
    mkdir -p "$config_dir"
    if [[ ! -f "$repos_file" ]] || ! grep -Fxq "$main_repo" "$repos_file" 2>/dev/null; then
      echo "$main_repo" >> "$repos_file"
    fi
  fi

  _gwt_cd() {
    if [[ $# -ne 1 ]]; then
      echo "gwt: unknown command: cd $*" >&2
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
        return 0
      elif [[ ${#matches[@]} -eq 1 ]]; then
        cd "${matches[1]}"
        return 0
      elif [[ ${#matches[@]} -gt 1 ]]; then
        echo "gwt: multiple worktrees match '$query':" >&2
        for m in "${matches[@]}"; do
          echo "  $m" >&2
        done
        return 1
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
          return 0
        elif [[ ${#matches[@]} -eq 1 ]]; then
          cd "${matches[1]}"
          return 0
        elif [[ ${#matches[@]} -gt 1 ]]; then
          echo "gwt: multiple worktrees match '$query':" >&2
          for m in "${matches[@]}"; do
            echo "  $m" >&2
          done
          return 1
        fi
      fi
    fi

    echo "gwt: no matching worktree found for '$query'" >&2
    return 1
  }

  _gwt_remove() {
    git worktree remove "$@"
  }

  _gwt_pull() {
    if [[ $# -ne 1 ]]; then
      echo "gwt: unknown command: pull $*" >&2
      return 1
    fi
    local branch="$1"
    mkdir -p "$dir_gwt"
    git fetch origin "$branch"
    local dest="$dir_gwt/$branch"
    git worktree add -b "$branch" "$dest" "origin/$branch"
    cd "$dest"
    _gwt_init_ide
  }

  _gwt_create() {
    if [[ $# -ne 1 ]]; then
      echo "gwt: unknown command: $*" >&2
      return 1
    fi
    local branch="$1"
    mkdir -p "$dir_gwt"
    local dest="$dir_gwt/$branch"
    git worktree add "$dest"
    cd "$dest"
    _gwt_init_ide
  }

  _gwt_init_ide() {
    if [ -f yarn.lock ]; then
      yarn
    fi
    nvim
  }

  {
    case "$1" in
      cd)
        shift
        _gwt_cd "$@"
        ;;
      remove|rm)
        shift
        _gwt_remove "$@"
        ;;
      pull|p)
        shift
        _gwt_pull "$@"
        ;;
      *)
        _gwt_create "$@"
        ;;
    esac
  } always {
    unfunction _gwt_remove _gwt_pull _gwt_create _gwt_init_ide _gwt_cd _gwt_find_worktrees 2>/dev/null
  }
}

