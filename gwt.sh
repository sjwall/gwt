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
  local init_ide=false
  local dir_gwt="../gwt-${dir_name}"
  mkdir -p "$dir_gwt"
  case "$1" in
    remove|rm)
      shift
      git worktree remove $@
      ;;
    pull|p)
      git fetch origin "$2"
      local dest="$dir_gwt/$2"
      git worktree add -b "$2" "$dest" "origin/$2"
      cd "$dest"
      init_ide=true
      ;;
    *)
      if [[ $# -ne 1 ]]; then
        echo "gwt: unknown command: $*" >&2
        return 1
      fi
      local dest="$dir_gwt/$1"
      git worktree add "$dest"
      cd "$dest"
      init_ide=true
      ;;
  esac
  if $init_ide; then
    if [ -f yarn.lock ]; then
      yarn
    fi
    nvim
  fi
}

