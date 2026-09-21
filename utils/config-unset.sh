#!/bin/zsh
_gwt_unset_config() {
  local target_key="$1"
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local config_file="$config_dir/config"
  if [[ -f "$config_file" ]]; then
    local temp_file="${config_file}.tmp.$$"
    touch "$temp_file"
    local line key
    while IFS= read -r line || [[ -n "$line" ]]; do
      if [[ "$line" == *"="* && "$line" != \#* ]]; then
        key="${line%%=*}"
        if [[ "$key" == "$target_key" ]]; then
          continue
        fi
      fi
      echo "$line" >> "$temp_file"
    done < "$config_file"
    mv "$temp_file" "$config_file"
  fi
}
