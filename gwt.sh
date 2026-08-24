#!/bin/zsh
# git worktree helper
#  gwt NAME            create worktree ../gwt-<dir-name>/NAME, cd, yarn, nvim
#  gwt pull NAME       fetch origin/NAME, create tracking worktree, cd, yarn, nvim
#  gwt p NAME          as above
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

  _gwt_remove() {
    git worktree remove "$@"
  }

  _gwt_pull() {
    if [[ $# -ne 1 ]]; then
      echo "gwt: unknown command: pull $*" >&2
      return 1
    fi
    local branch="$1"
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
    mkdir -p "$dir_gwt"
    case "$1" in
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
    unfunction _gwt_remove _gwt_pull _gwt_create _gwt_init_ide 2>/dev/null
  }
}

