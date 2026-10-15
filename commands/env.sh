#!/bin/zsh
_gwt_env() {
  if [[ -z "$main_repo" ]]; then
    echo "gwt: not inside a git repository" >&2
    return 58
  fi

  {
    case "$1" in
       add)
         shift
         source "$gwt_dir/commands/env-add.sh"
         _gwt_env_add "$@"
         ;;
       list|ls)
         shift
         source "$gwt_dir/commands/env-list.sh"
         _gwt_env_list "$@"
         ;;
       sync)
         shift
         source "$gwt_dir/commands/env-sync.sh"
         _gwt_env_sync "$@"
         ;;
       remove|rm)
         shift
         source "$gwt_dir/commands/env-remove.sh"
         _gwt_env_remove "$@"
         ;;
       *)
         source "$gwt_dir/commands/env-sync.sh"
         _gwt_env_sync "$@"
         ;;
    esac
  } always {
    unfunction _gwt_env_sync _gwt_env_add _gwt_env_list _gwt_env_remove 2>/dev/null
  }
}
