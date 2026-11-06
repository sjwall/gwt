#!/bin/zsh
# Returns 0 if $1 is a safe .gwt/envs entry: a non-empty relative path with no
# ".." component, so it cannot name a file outside the source directory nor a
# location outside the current worktree. The config file is repo-controlled
# (it can be committed), so entries must be validated before use.
_gwt_env_entry_is_safe() {
  local entry="$1"
  [[ -n "$entry" ]] || return 1
  [[ "$entry" != /* ]] || return 1
  local part
  for part in ${(s./.)entry}; do
    [[ "$part" == ".." ]] && return 1
  done
  return 0
}

_gwt_env_sync() {
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt/envs"
  local local_config_file="$PWD/.gwt/envs"
  local main_config_file="$main_repo/.gwt/envs"
  local repo_name=$(basename $main_repo)
  local env_file="$main_config_file"
  local lockfile="$config_dir/envs.lock"

  source "$gwt_dir/utils/file-lock.sh"
  _gwt_acquire_lock "$lockfile" || return 75

  {
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

    local env_src="$main_repo"
    if [[ -n "$1" ]]; then
      env_src="$1"
      echo "gwt: using source dir $env_src"
    fi

    if [[ -f "$env_file" ]]; then
      local env_list=()
      local -A seen
      local r
      while IFS= read -r r || [[ -n "$r" ]]; do
        if ! _gwt_env_entry_is_safe "$r"; then
          [[ -n "$r" ]] && echo "gwt: WARN skipping unsafe env path: $r" >&2
          continue
        fi
        [[ ! -f "$env_src/$r" ]] && continue
        [[ -n "${seen[$r]}" ]] && continue
        seen[$r]=1
        env_list+=("$r")
      done < "$env_file"
      # Symlinked entries (or entries under symlinked directories) can escape
      # even when the entry itself is lexically safe, so containment is checked
      # again on the resolved paths at copy time.
      local src_root="${env_src:A}"
      local dest_root="${PWD:A}"
      echo "Copying files:"
      for (( i = 1; i <= ${#env_list[@]}; i++ )); do
        echo "${env_list[i]}" >&2
        local item="$env_src/${env_list[i]}"
        local dest="$PWD/${env_list[i]}"
        if [[ "${item:A}" != "$src_root"/* ]]; then
          echo "gwt: WARN skipping env path outside source dir: ${env_list[i]}" >&2
          continue
        fi
        if [[ "${dest:A}" != "$dest_root"/* ]]; then
          echo "gwt: WARN skipping env path outside worktree: ${env_list[i]}" >&2
          continue
        fi
        if [[ -f "$item" ]]; then
          cp "$item" "$dest"
        else
          echo "gwt: WARN unable to copy env ${env_list[i]}"
        fi
      done
    else
      echo "gwt: no env configured for $repo_name"
    fi
  } always {
    _gwt_release_lock "$lockfile"
  }
}
