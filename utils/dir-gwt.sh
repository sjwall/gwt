#!/bin/zsh
_gwt_get_dir_gwt() {
  source "$gwt_dir/utils/parent-get.sh"
  source "$gwt_dir/utils/path-check.sh"
  source "$gwt_dir/utils/parent-save.sh"
  local target_repo="${main_repo:-$PWD}"
  local safe_parent
  safe_parent=$(_gwt_get_configured_parent "$target_repo")
  if [[ -n "$safe_parent" ]]; then
    safe_parent="${safe_parent/#\~/$HOME}"
    safe_parent="${safe_parent%/}"
    echo "$safe_parent/gwt-${dir_name}"
    return 0
  fi

  if _gwt_is_unsuitable_path "$target_repo"; then
    echo "gwt: repository is in an unsuitable location ($target_repo)" >&2
    local user_input
    read -r "user_input?Enter safe parent directory for worktrees (e.g. ~/projects): "
    user_input="${user_input/#\~/$HOME}"
    user_input="${user_input%/}"
    if [[ -n "$user_input" ]]; then
      _gwt_save_configured_parent "$target_repo" "$user_input"
      echo "$user_input/gwt-${dir_name}"
      return 0
    fi
  fi

  echo "../gwt-${dir_name}"
}
