#!/bin/zsh
# git worktree helper
#  gwt [add] [--ide IDE] [--agent[=AGENT]|-a[=AGENT]] [--no-install] NAME  create worktree ../gwt-<dir-name>/NAME, cd, yarn, launch IDE or agent
#  gwt pull [--ide IDE] [--no-install] NAME   fetch origin/NAME, create or recreate tracking worktree, cd, yarn, launch IDE
#  gwt p NAME                  as above
#  gwt cd NAME                 cd to worktree matching NAME
#  gwt main [NAME]             cd to main repository matching NAME (defaults to main repo of current worktree, or prompts/switches from tracked repos if not in repo)
#  gwt m [NAME]                as above
#  gwt M [--ide IDE] [NAME]    as above, launch IDE
#  gwt switch [--ide IDE] NAME cd to worktree matching NAME, launch IDE
#  gwt s NAME                  as above
#  gwt agent [--agent AGENT] NAME cd to worktree matching NAME, launch agent
#  gwt a NAME                  as above
#  gwt ls [NAME]               list tracked worktrees (matching repository NAME)
#  gwt list [NAME]             as above
#  gwt remove [NAME]           remove worktree (defaults to current worktree, cd to main repo)
#  gwt rm [NAME]               as above
#  gwt rm -force [NAME]        as above but with force
#  gwt rm -f [NAME]            as above
#  gwt track [PATH]            track git repository (defaults to current repository)
#  gwt t [PATH]                as above
#  gwt config [KEY] [VAL]      get or set configuration (e.g. gwt config ide code)
#  gwt ide [NAME]              get or set configured IDE (defaults to nvim)
#  gwt skills [TARGETS]        manage agent skill symlinks (e.g. gwt skills claude,gemini)
#  gwt migrate [--dry-run]     migrate worktrees to expected path pattern
#  gwt upgrade                 upgrade gwt repository (git pull)
#
# Exit Codes:
#   0  - Success
#   1  - cd: invalid argument count (expected exactly 1 argument)
#   2  - cd: multiple worktrees match in current repository
#   3  - cd: multiple worktrees match across tracked repositories
#   4  - cd: no matching worktree found for query
#   5  - main: not inside a git repository and no repository specified / found
#   6  - main: invalid argument count (more than 1 argument provided)
#   7  - main: multiple exact repository matches found
#   8  - main: multiple repository name matches found
#   9  - main: multiple repository path matches found
#   10 - main: no matching repository found for query
#   11 - switch: --ide option requires an argument
#   12 - switch: invalid argument count (expected exactly 1 worktree name)
#   13 - remove: cannot remove main repository
#   14 - remove: failed to change directory to main repository
#   15 - remove: git worktree remove command failed
#   16 - pull: --ide option requires an argument
#   17 - pull: invalid argument count (expected exactly 1 branch name)
#   18 - pull: failed to determine target worktree directory location
#   19 - pull: failed to create worktree parent directory
#   20 - pull: git fetch origin failed
#   21 - pull: git worktree add command failed
#   22 - pull: failed to change directory to newly created worktree
#   23 - add: --ide option requires an argument
#   24 - add: invalid argument count (expected exactly 1 branch name)
#   25 - add: failed to determine target worktree directory location
#   26 - add: failed to create worktree parent directory
#   27 - add: git worktree add command failed
#   28 - add: failed to change directory to newly created worktree
#   29 - config: invalid argument count for get (expected exactly 1 key)
#   30 - config: specified key not found in get
#   31 - config: invalid argument count for set (expected key and value)
#   32 - config: invalid argument count for unset (expected exactly 1 key)
#   33 - config: specified key not found
#   34 - upgrade: gwt repository not found at target directory
#   35 - upgrade: git pull failed during upgrade
#   36 - track: not inside a git repository and no repository specified
#   37 - track: invalid argument count (more than 1 argument provided)
#   38 - track: specified path is not a git repository
#   39 - list: invalid argument count (more than 1 repository specified)
#   40 - list: multiple exact repository matches found
#   41 - list: multiple repository name matches found
#   42 - list: multiple repository path matches found
#   43 - list: no matching repository found for query
#   44 - agent: --agent option requires an argument
#   45 - agent: invalid argument count (expected exactly 1 worktree name)
#   46 - agent: no agent configured
#   47 - skills: skills helper not found
#   48 - migrate: not inside a git repository
#   49 - migrate: unknown argument
#   50 - migrate: failed to determine target worktree directory location
#   51 - migrate: git worktree move failed
unalias gwt 2>/dev/null || true  #omz git plugin defines `gwt` alias; remove so func wins
gwt() {
  # TODO: Detect this
  local gwt_dir="$HOME/.local/share/gwt"
  local main_repo=$(git worktree list --porcelain 2>/dev/null | head -n 1 | sed 's/^worktree //')
  local dir_name=$(basename "${main_repo:-$PWD}")

  if [[ -n "$main_repo" ]]; then
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
    local repos_file="$config_dir/repos"
    local lockfile="$config_dir/repos.lock"
    
    source "$gwt_dir/utils/file-lock.sh"
    _gwt_acquire_lock "$lockfile" || return 1
    
    mkdir -p "$config_dir"
    if [[ ! -f "$repos_file" ]] || ! grep -Fxq "$main_repo" "$repos_file" 2>/dev/null; then
      echo "$main_repo" >> "$repos_file"
    fi
    
    _gwt_release_lock "$lockfile"
  fi

  {
    case "$1" in
      config)
        shift
        source "$gwt_dir/commands/config.sh"
        _gwt_config "$@"
        ;;
      ide)
        shift
        source "$gwt_dir/commands/config.sh"
        if [[ $# -eq 0 ]]; then
          _gwt_config get ide
        else
          _gwt_config set ide "$@"
        fi
        ;;
      switch|s)
        shift
        source "$gwt_dir/commands/switch.sh"
        _gwt_switch "$@"
        ;;
      agent|a)
        shift
        source "$gwt_dir/commands/agent.sh"
        _gwt_agent "$@"
        ;;
      cd)
        shift
        source "$gwt_dir/commands/cd.sh"
        _gwt_cd "$@"
        ;;
      main|m)
        shift
        source "$gwt_dir/commands/main.sh"
        _gwt_main "$@"
        ;;
      Main|M)
        shift
        source "$gwt_dir/commands/main.sh"
        _gwt_main_ide "$@"
        ;;
      list|ls)
        shift
        source "$gwt_dir/commands/ls.sh"
        _gwt_ls "$@"
        ;;
      remove|rm)
        shift
        source "$gwt_dir/commands/remove.sh"
        _gwt_remove "$@"
        ;;
      pull|p)
        shift
        source "$gwt_dir/commands/pull.sh"
        _gwt_pull "$@"
        ;;
      skills)
        shift
        source "$gwt_dir/commands/skills.sh"
        _gwt_skills "$@"
        ;;
      migrate)
        shift
        source "$gwt_dir/commands/migrate.sh"
        _gwt_migrate "$@"
        ;;
      upgrade)
        shift
        source "$gwt_dir/commands/upgrade.sh"
        _gwt_upgrade "$@"
        ;;
      track|t)
        shift
        source "$gwt_dir/commands/track.sh"
        _gwt_track "$@"
        ;;
      add)
        source "$gwt_dir/commands/create.sh"
        _gwt_create "$@"
        ;;
      *)
        source "$gwt_dir/commands/create.sh"
        _gwt_create "$@"
        ;;
    esac
  } always {
    unfunction _gwt_remove _gwt_pull _gwt_create _gwt_init_ide _gwt_launch_ide _gwt_cd _gwt_main _gwt_main_ide _gwt_switch _gwt_agent _gwt_launch_agent _gwt_get_agent _gwt_ls _gwt_find_worktrees _gwt_is_unsuitable_path _gwt_get_configured_parent _gwt_save_configured_parent _gwt_get_dir_gwt _gwt_get_config _gwt_save_config _gwt_unset_config _gwt_get_ide _gwt_config _gwt_upgrade _gwt_track _gwt_skills _gwt_migrate 2>/dev/null
  }
}

# ------------------------------------------------------------------------------
# Zsh Completion Definition for gwt
# ------------------------------------------------------------------------------
if [[ -n "$ZSH_VERSION" ]]; then
  _gwt_comp_dir="${${(%):-%x}:A:h}"
  if [[ -d "$_gwt_comp_dir" && "${fpath[(I)$_gwt_comp_dir]}" -eq 0 ]]; then
    fpath=("$_gwt_comp_dir" $fpath)
  fi
  if [[ -f "$_gwt_comp_dir/_gwt" ]]; then
    source "$_gwt_comp_dir/_gwt"
  fi
  if (( $+functions[compdef] )); then
    compdef _gwt gwt
  fi
  unset _gwt_comp_dir
fi
