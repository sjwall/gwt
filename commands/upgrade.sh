#!/bin/zsh
_gwt_upgrade() {
  local gwt_dir="${GWT_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/gwt}"
  if [[ ! -d "$gwt_dir/.git" ]]; then
    local script_dir="${${(%):-%x}:A:h}"
    if [[ -d "$script_dir/.git" ]]; then
      gwt_dir="$script_dir"
    fi
  fi

  if [[ ! -d "$gwt_dir/.git" ]]; then
    echo "gwt: repository not found at $gwt_dir" >&2
    return 34
  fi

  echo "Upgrading gwt at $gwt_dir..."
  if git -C "$gwt_dir" pull "$@"; then
    if [[ -f "$gwt_dir/gwt.sh" ]]; then
      source "$gwt_dir/gwt.sh"
    fi
    if [[ -f "$gwt_dir/skills.sh" ]]; then
      sh "$gwt_dir/skills.sh" --dir="$gwt_dir" --sync
    fi
    return 0
  else
    return 35
  fi
}
