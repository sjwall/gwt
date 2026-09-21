#!/bin/zsh
_gwt_is_unsuitable_path() {
  local target_path="$1"
  if [[ "$target_path" == *"/."* || "$target_path" == "."* ]]; then
    return 0
  fi
  local parent_dir=$(dirname "$target_path")
  if [[ ! -w "$parent_dir" || "$parent_dir" == "/" ]]; then
    return 0
  fi
  return 1
}
