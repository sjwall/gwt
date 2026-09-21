#!/bin/zsh
_gwt_create() {
  source "$gwt_dir/utils/dir-gwt.sh"
  source "$gwt_dir/utils/ide-launch.sh"
  source "$gwt_dir/utils/agent-launch.sh"
  local override_ide=""
  local override_agent=""
  local use_agent=0
  local skip_install=0
  local agent_opt_idx=0
  local raw=("$@")
  local -a non_opts=()
  local -a non_opt_indices=()
  local i=1

  while (( i <= $#raw )); do
    local arg="${raw[i]}"
    case "$arg" in
      --ide=*)
        override_ide="${arg#--ide=}"
        (( i++ ))
        ;;
      --ide)
        if (( i + 1 > $#raw )); then
          echo "gwt: --ide requires an argument" >&2
          return 23
        fi
        override_ide="${raw[i+1]}"
        (( i += 2 ))
        ;;
      --no-install)
        skip_install=1
        (( i++ ))
        ;;
      --agent=*)
        use_agent=1
        override_agent="${arg#--agent=}"
        (( i++ ))
        ;;
      -a=*)
        use_agent=1
        override_agent="${arg#-a=}"
        (( i++ ))
        ;;
      --agent|-a)
        use_agent=1
        agent_opt_idx=$i
        (( i++ ))
        ;;
      *)
        non_opts+=("$arg")
        non_opt_indices+=($i)
        (( i++ ))
        ;;
    esac
  done

  local branch=""
  if (( agent_opt_idx > 0 )); then
    if (( ${#non_opts[@]} == 1 )); then
      branch="${non_opts[1]}"
    elif (( ${#non_opts[@]} == 2 )); then
      if (( non_opt_indices[1] == agent_opt_idx + 1 )); then
        override_agent="${non_opts[1]}"
        branch="${non_opts[2]}"
      elif (( non_opt_indices[2] == agent_opt_idx + 1 )); then
        override_agent="${non_opts[2]}"
        branch="${non_opts[1]}"
      else
        echo "gwt: unknown command: '${non_opts[*]}'" >&2
        return 24
      fi
    else
      echo "gwt: unknown command: '${non_opts[*]}'" >&2
      return 24
    fi
  else
    if (( ${#non_opts[@]} == 1 )); then
      branch="${non_opts[1]}"
    else
      echo "gwt: unknown command: '${non_opts[*]}'" >&2
      return 24
    fi
  fi

  local dir_gwt
  dir_gwt=$(_gwt_get_dir_gwt) || return 25
  mkdir -p "$dir_gwt" || return 26
  local dest="$dir_gwt/$branch"
  git worktree add "$dest" || return 27
  cd "$dest" || return 28
  if [[ "$skip_install" -eq 0 ]]; then
    if [ -f yarn.lock ]; then
      yarn
    fi
  fi
  if [[ $use_agent -eq 1 ]]; then
    _gwt_launch_agent "$override_agent"
  else
    _gwt_launch_ide "$override_ide"
  fi
}
