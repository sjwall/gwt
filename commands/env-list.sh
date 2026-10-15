#!/bin/zsh
_gwt_env_list() {
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt/envs"
  local local_config_file="$PWD/.gwt/envs"
  local main_config_file="$main_repo/.gwt/envs"
  local repo_name=$(basename $main_repo)
  local env_file="$main_config_file"

  # Current worktree always overrides main
  if [[ -f "$local_config_file" ]]; then
    env_file="$local_config_file"
  fi
  # User config always overrides local
  if [[ -f "$config_dir/$repo_name" ]]; then
    if [[ -f "$env_file" ]]; then
      echo "gwt: WARN configured env is overriden by user env!"
    fi
    env_file="$config_dir/$repo_name"
  fi

  if [[ -f "$env_file" ]]; then
    echo $env_file
    local env_list=()
    local -A seen
    local r
    while IFS= read -r r || [[ -n "$r" ]]; do
      [[ -z "$r" || ! -f "$main_repo/$r" ]] && continue
      [[ -n "${seen[$r]}" ]] && continue
      seen[$r]=1
      env_list+=("$r")
    done < "$env_file"
    echo "environment files:"
    for (( i = 1; i <= ${#env_list[@]}; i++ )); do
      echo "${env_list[i]}" >&2
    done
  else
    echo "gwt: no env configured for $repo_name"
  fi
}
