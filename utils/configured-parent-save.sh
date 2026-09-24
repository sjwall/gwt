#!/bin/zsh
source "$gwt_dir/utils/file-lock.sh"

_gwt_save_configured_parent() {
  local repo="$1"
  local safe_parent="$2"
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local locations_file="$config_dir/locations"
  local lockfile="$config_dir/locations.lock"

  _gwt_acquire_lock "$lockfile" || return 1

  mkdir -p "$config_dir"
  if [[ -f "$locations_file" ]]; then
    local temp_file="${locations_file}.tmp.$$"
    local found=0
    local line key
    while IFS= read -r line || [[ -n "$line" ]]; do
      if [[ "$line" == *"="* ]]; then
        key="${line%%=*}"
        if [[ "$key" == "$repo" ]]; then
          echo "$repo=$safe_parent" >> "$temp_file"
          found=1
          continue
        fi
      fi
      echo "$line" >> "$temp_file"
    done < "$locations_file"
    if [[ $found -eq 0 ]]; then
      echo "$repo=$safe_parent" >> "$temp_file"
    fi
    mv "$temp_file" "$locations_file"
  else
    echo "$repo=$safe_parent" >> "$locations_file"
  fi

  _gwt_release_lock "$lockfile"
}
