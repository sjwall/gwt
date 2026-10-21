#!/bin/zsh
_gwt_track_prune() {
  local target_repo=""
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local repos_file="$config_dir/repos"
  local lockfile="$config_dir/repos.lock"
  local dry_run="0"

  if [[ -n "$1" ]] && [[ "$1" == "--dry-run" ]]; then
    dry_run="1"
  fi

  if [ -f "$repos_file" ]; then
    source "$gwt_dir/utils/file-lock.sh"
    _gwt_acquire_lock "$lockfile" || return 1

    {
      if [[ "$dry_run" == "1" ]]; then
        echo "Repos to remove:"
      fi

      local other_repos=()
      local r
      while IFS= read -r r || [[ -n "$r" ]]; do
        [[ -z "$r" || "$r" == "$main_repo" ]] && continue
        if [[ ! -d "$r" ]]; then
          if [[ "$dry_run" == "1" ]]; then
            echo "$r"
          fi
          continue
        fi
        other_repos+=("$r")
      done < "$repos_file"

      if [[ "$dry_run" == "0" ]]; then
        rm $repos_file

        for (( i = 1; i <= ${#other_repos[@]}; i++ )); do
          echo "${other_repos[i]}" >> "$repos_file"
        done
      fi
    } always {
      _gwt_release_lock "$lockfile"
    }
  fi
  return 0
}
