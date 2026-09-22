#!/bin/zsh
_gwt_get_configured_parent() {
  local repo="$1"
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local locations_file="$config_dir/locations"
  local config_file="$config_dir/config"
  local file
  for file in "$locations_file" "$config_file"; do
    if [[ -f "$file" ]]; then
      local line key val
      while IFS= read -r line || [[ -n "$line" ]]; do
        [[ -z "$line" || "$line" == \#* ]] && continue
        if [[ "$line" == *"="* ]]; then
          key="${line%%=*}"
          val="${line#*=}"
          if [[ "$key" == "$repo" ]]; then
            if _gwt_is_unsuitable_path "$val"; then
              echo "gwt: configured parent path is unsuitable ($val)" >&2
              return 1
            fi
            echo "$val"
            return 0
          fi
        fi
      done < "$file"
    fi
  done
  return 1
}
