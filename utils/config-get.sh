#!/bin/zsh
_gwt_get_config() {
  local target_key="$1"
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local config_file="$config_dir/config"
  if [[ -f "$config_file" ]]; then
    local line key val
    while IFS= read -r line || [[ -n "$line" ]]; do
      [[ -z "$line" || "$line" == \#* ]] && continue
      if [[ "$line" == *"="* ]]; then
        key="${line%%=*}"
        val="${line#*=}"
        if [[ "$key" == "$target_key" ]]; then
          echo "$val"
          return 0
        fi
      fi
    done < "$config_file"
  fi
  return 1
}
