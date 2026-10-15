#!/bin/zsh
_gwt_env_remove() {
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

  if [[ -n "$1" ]]; then
    local env_list=()
    local -A seen
    local r
    while IFS= read -r r || [[ -n "$r" ]]; do
      [[ "$1" = "$r" ]] && continue
      [[ -n "${seen[$r]}" ]] && continue
      seen[$r]=1
      env_list+=("$r")
    done < "$env_file"
    rm $env_file
    for (( i = 1; i <= ${#env_list[@]}; i++ )); do
      echo "${env_list[i]}" >> "$env_file"
    done
  elif [[ -f "$env_file" ]]; then
    echo "gwt: Deleting $env_file"
    rm $env_file
  else
    echo "gwt: nothing to remove"
  fi

  _gwt_release_lock "$lockfile"
  return 0
}
