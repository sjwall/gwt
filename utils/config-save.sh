#!/bin/zsh
_gwt_save_config() {
  local target_key="$1"
  local target_val="$2"
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local config_file="$config_dir/config"
  mkdir -p "$config_dir"
  if [[ -f "$config_file" ]]; then
    local temp_file="${config_file}.tmp.$$"
    touch "$temp_file"
    local found=0
    local line key
    while IFS= read -r line || [[ -n "$line" ]]; do
      if [[ "$line" == *"="* && "$line" != \#* ]]; then
        key="${line%%=*}"
        if [[ "$key" == "$target_key" ]]; then
          echo "$target_key=$target_val" >> "$temp_file"
          found=1
          continue
        fi
      fi
      echo "$line" >> "$temp_file"
    done < "$config_file"
    if [[ $found -eq 0 ]]; then
      echo "$target_key=$target_val" >> "$temp_file"
    fi
    mv "$temp_file" "$config_file"
  else
    echo "$target_key=$target_val" >> "$config_file"
  fi
}
