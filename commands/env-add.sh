#!/bin/zsh
_gwt_env_add() {
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt/envs"
  local local_config_file="$PWD/.gwt/envs"
  local repo_name=$(basename $main_repo)
  local env_file="$local_config_file"

  #
  # User config overrides local only when -u passed
  if [[ "$1" = "-u" ]]; then
    shift
    env_file="$config_dir/$repo_name"
  fi

  source "$gwt_dir/utils/file-lock.sh"
  _gwt_acquire_lock "$lockfile" || return 1

  {
    echo $env_file
    if [[ ! -f "$env_file" ]]; then
      mkdir -p -- "${env_file:h}"
    fi

    echo "$1" >> "$env_file"
  } always {
    _gwt_release_lock "$lockfile"
  }
  return 0
}
