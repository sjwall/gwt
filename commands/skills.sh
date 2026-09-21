#!/bin/zsh
_gwt_skills() {
  local gwt_dir="${GWT_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/gwt}"
  if [[ ! -d "$gwt_dir" || ! -f "$gwt_dir/skills.sh" ]]; then
    local script_dir="${${(%):-%x}:A:h}"
    if [[ -f "$script_dir/skills.sh" ]]; then
      gwt_dir="$script_dir"
    fi
  fi

  if [[ ! -f "$gwt_dir/skills.sh" ]]; then
    echo "gwt: skills helper not found at $gwt_dir/skills.sh" >&2
    return 47
  fi

  sh "$gwt_dir/skills.sh" --dir="$gwt_dir" "$@"
}
