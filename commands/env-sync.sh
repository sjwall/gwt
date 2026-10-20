#!/bin/zsh
_gwt_env_sync() {
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

  local env_src=$main_repo
  if [[ -n "$1" ]]; then
    echo "gwt: using source dir $env_src"
    $env_src=$1
  fi

  if [[ -f "$env_file" ]]; then
    local env_list=()
    local -A seen
    local r
    while IFS= read -r r || [[ -n "$r" ]]; do
      [[ -z "$r" || ! -f "$env_src/$r" ]] && continue
      [[ -n "${seen[$r]}" ]] && continue
      seen[$r]=1
      env_list+=("$r")
    done < "$env_file"
    echo "Copying files:"
    for (( i = 1; i <= ${#env_list[@]}; i++ )); do
      echo "${env_list[i]}" >&2
      local item="$env_src/${env_list[i]}"
      if [[ -f "$item" ]]; then
        cp "$item" "$PWD/${env_list[i]}"
      else
        echo "gwt: WARN unable to copy env ${env_list[i]}"
      fi
    done
  else
    echo "gwt: no env configured for $repo_name"
  fi
}
