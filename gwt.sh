#!/bin/zsh
# git worktree helper
#  gwt [add] [--ide IDE] [--no-install] NAME  create worktree ../gwt-<dir-name>/NAME, cd, yarn, launch IDE
#  gwt pull [--ide IDE] [--no-install] NAME   fetch origin/NAME, create tracking worktree, cd, yarn, launch IDE
#  gwt p NAME                  as above
#  gwt cd NAME                 cd to worktree matching NAME
#  gwt main [NAME]             cd to main repository matching NAME (defaults to main repo of current worktree)
#  gwt m [NAME]                as above
#  gwt switch [--ide IDE] NAME cd to worktree matching NAME, launch IDE
#  gwt s NAME                  as above
#  gwt ls                      list all tracked worktrees
#  gwt remove [NAME]           remove worktree (defaults to current worktree, cd to main repo)
#  gwt rm [NAME]               as above
#  gwt rm -force [NAME]        as above but with force
#  gwt rm -f [NAME]            as above
#  gwt config [KEY] [VAL]      get or set configuration (e.g. gwt config ide code)
#  gwt ide [NAME]              get or set configured IDE (defaults to nvim)
#  gwt upgrade                 upgrade gwt repository (git pull)
unalias gwt 2>/dev/null  #omz git plugin defines `gwt` alias; remove so func wins
gwt() {
  local main_repo=$(git worktree list --porcelain 2>/dev/null | head -n 1 | sed 's/^worktree //')
  local dir_name=$(basename "${main_repo:-$PWD}")

  if [[ -n "$main_repo" ]]; then
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
    local repos_file="$config_dir/repos"
    mkdir -p "$config_dir"
    if [[ ! -f "$repos_file" ]] || ! grep -Fxq "$main_repo" "$repos_file" 2>/dev/null; then
      echo "$main_repo" >> "$repos_file"
    fi
  fi

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
              echo "$val"
              return 0
            fi
          fi
        done < "$file"
      fi
    done
    return 1
  }

  _gwt_save_configured_parent() {
    local repo="$1"
    local safe_parent="$2"
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
    local locations_file="$config_dir/locations"
    mkdir -p "$config_dir"
    if [[ -f "$locations_file" ]]; then
      local temp_file="${locations_file}.tmp.$$"
      local found=0
      local line key
      while IFS= read -r line || [[ -n "$line" ]]; do
        if [[ "$line" == *"="* ]]; then
          key="${line%%=*}"
          if [[ "$key" == "$repo" ]]; then
            echo "$repo=$safe_parent" >> "$temp_file"
            found=1
            continue
          fi
        fi
        echo "$line" >> "$temp_file"
      done < "$locations_file"
      if [[ $found -eq 0 ]]; then
        echo "$repo=$safe_parent" >> "$temp_file"
      fi
      mv "$temp_file" "$locations_file"
    else
      echo "$repo=$safe_parent" >> "$locations_file"
    fi
  }

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

  _gwt_get_ide() {
    local ide
    ide=$(_gwt_get_config "ide")
    if [[ -n "$ide" ]]; then
      echo "$ide"
    elif [[ -n "$GWT_IDE" ]]; then
      echo "$GWT_IDE"
    else
      echo "nvim"
    fi
  }

  _gwt_get_dir_gwt() {
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

  _gwt_cd() {
    if [[ $# -ne 1 ]]; then
      echo "gwt: unknown command: 'cd $*'" >&2
      return 1
    fi

    local query="$1"
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
    local repos_file="$config_dir/repos"

    local matches=()
    local exact_matches=()

    _gwt_find_worktrees() {
      local repo_list=("$@")
      matches=()
      exact_matches=()
      local repo wt_list wt_path wt_name
      for repo in "${repo_list[@]}"; do
        [[ -z "$repo" || ! -d "$repo" ]] && continue
        wt_list=$(git -C "$repo" worktree list --porcelain 2>/dev/null | grep "^worktree " | sed "s/^worktree //")
        while IFS= read -r wt_path || [[ -n "$wt_path" ]]; do
          [[ -z "$wt_path" ]] && continue
          wt_name=$(basename "$wt_path")
          if [[ "${wt_name:l}" == "${query:l}" ]]; then
            exact_matches+=("$wt_path")
          fi
          if [[ "${wt_name:l}" == *"${query:l}"* ]]; then
            matches+=("$wt_path")
          fi
        done <<< "$wt_list"
      done
    }

    if [[ -n "$main_repo" ]]; then
      _gwt_find_worktrees "$main_repo"
      if [[ ${#exact_matches[@]} -eq 1 ]]; then
        cd "${exact_matches[1]}"
        echo "${exact_matches[1]}"
        return 0
      elif [[ ${#matches[@]} -eq 1 ]]; then
        cd "${matches[1]}"
        echo "${matches[1]}"
        return 0
      elif [[ ${#matches[@]} -gt 1 ]]; then
        echo "gwt: multiple worktrees match '$query':" >&2
        for m in "${matches[@]}"; do
          echo "  $m" >&2
        done
        return 1
      fi
    fi

    if [[ -f "$repos_file" ]]; then
      local other_repos=()
      local r
      while IFS= read -r r || [[ -n "$r" ]]; do
        [[ -z "$r" || "$r" == "$main_repo" ]] && continue
        other_repos+=("$r")
      done < "$repos_file"

      if [[ ${#other_repos[@]} -gt 0 ]]; then
        _gwt_find_worktrees "${other_repos[@]}"
        if [[ ${#exact_matches[@]} -eq 1 ]]; then
          cd "${exact_matches[1]}"
          echo "${exact_matches[1]}"
          return 0
        elif [[ ${#matches[@]} -eq 1 ]]; then
          cd "${matches[1]}"
          echo "${matches[1]}"
          return 0
        elif [[ ${#matches[@]} -gt 1 ]]; then
          echo "gwt: multiple worktrees match '$query':" >&2
          for m in "${matches[@]}"; do
            echo "  $m" >&2
          done
          return 1
        fi
      fi
    fi

    echo "gwt: no matching worktree found for '$query'" >&2
    return 1
  }

  _gwt_main() {
    local query="$1"
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
    local repos_file="$config_dir/repos"

    if [[ $# -eq 0 ]]; then
      if [[ -n "$main_repo" ]]; then
        cd "$main_repo"
        echo "$main_repo"
        return 0
      else
        echo "gwt: no matching repository found" >&2
        return 1
      fi
    elif [[ $# -gt 1 ]]; then
      echo "gwt: unknown command: 'main $*'" >&2
      return 1
    fi

    local repo_list=()
    if [[ -n "$main_repo" ]]; then
      repo_list+=("$main_repo")
    fi

    if [[ -f "$repos_file" ]]; then
      local r
      while IFS= read -r r || [[ -n "$r" ]]; do
        [[ -z "$r" ]] && continue
        repo_list+=("$r")
      done < "$repos_file"
    fi

    local -A seen
    local repo repo_name
    local exact_matches=()
    local matches=()
    local path_matches=()

    for repo in "${repo_list[@]}"; do
      [[ -z "$repo" || ! -d "$repo" ]] && continue
      [[ -n "${seen[$repo]}" ]] && continue
      seen[$repo]=1

      repo_name=$(basename "$repo")
      if [[ "${repo_name:l}" == "${query:l}" ]]; then
        exact_matches+=("$repo")
      fi
      if [[ "${repo_name:l}" == *"${query:l}"* ]]; then
        matches+=("$repo")
      elif [[ "${repo:l}" == *"${query:l}"* ]]; then
        path_matches+=("$repo")
      fi
    done

    if [[ ${#exact_matches[@]} -eq 1 ]]; then
      cd "${exact_matches[1]}"
      echo "${exact_matches[1]}"
      return 0
    elif [[ ${#exact_matches[@]} -gt 1 ]]; then
      echo "gwt: multiple repositories match '$query':" >&2
      for m in "${exact_matches[@]}"; do
        echo "  $m" >&2
      done
      return 1
    elif [[ ${#matches[@]} -eq 1 ]]; then
      cd "${matches[1]}"
      echo "${matches[1]}"
      return 0
    elif [[ ${#matches[@]} -gt 1 ]]; then
      echo "gwt: multiple repositories match '$query':" >&2
      for m in "${matches[@]}"; do
        echo "  $m" >&2
      done
      return 1
    elif [[ ${#path_matches[@]} -eq 1 ]]; then
      cd "${path_matches[1]}"
      echo "${path_matches[1]}"
      return 0
    elif [[ ${#path_matches[@]} -gt 1 ]]; then
      echo "gwt: multiple repositories match '$query':" >&2
      for m in "${path_matches[@]}"; do
        echo "  $m" >&2
      done
      return 1
    fi

    echo "gwt: no matching repository found for '$query'" >&2
    return 1
  }

  _gwt_switch() {
    local override_ide=""
    local args=()

    while [[ $# -gt 0 ]]; do
      case "$1" in
        --ide=*)
          override_ide="${1#--ide=}"
          shift
          ;;
        --ide)
          if [[ $# -lt 2 ]]; then
            echo "gwt: --ide requires an argument" >&2
            return 1
          fi
          override_ide="$2"
          shift 2
          ;;
        *)
          args+=("$1")
          shift
          ;;
      esac
    done

    if [[ ${#args[@]} -ne 1 ]]; then
      echo "gwt: unknown command: switch '${args[*]}'" >&2
      return 1
    fi

    local query="${args[1]}"
    _gwt_cd "$query" || return 1
    _gwt_launch_ide "$override_ide"
  }

  _gwt_ls() {
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
    local repos_file="$config_dir/repos"
    local repo_list=()

    if [[ -n "$main_repo" ]]; then
      repo_list+=("$main_repo")
    fi

    if [[ -f "$repos_file" ]]; then
      local r
      while IFS= read -r r || [[ -n "$r" ]]; do
        [[ -z "$r" ]] && continue
        repo_list+=("$r")
      done < "$repos_file"
    fi

    local -A seen
    local repo
    for repo in "${repo_list[@]}"; do
      [[ -z "$repo" || ! -d "$repo" ]] && continue
      [[ -n "${seen[$repo]}" ]] && continue
      seen[$repo]=1
      git -C "$repo" rev-parse --git-dir >/dev/null 2>&1 || continue
      git -C "$repo" worktree list "$@"
    done
  }

  _gwt_remove() {
    local flags=()
    local targets=()

    while [[ $# -gt 0 ]]; do
      case "$1" in
        -f|--force|-force)
          flags+=(--force)
          shift
          ;;
        *)
          targets+=("$1")
          shift
          ;;
      esac
    done

    if [[ -z "$main_repo" ]]; then
      git worktree remove "${flags[@]}" "${targets[@]}"
      return $?
    fi

    local current_wt=$(git rev-parse --show-toplevel 2>/dev/null)
    local is_linked_worktree=0
    if [[ -n "$current_wt" && "$current_wt" != "$main_repo" ]] || [[ "$(git rev-parse --git-dir 2>/dev/null)" != "$(git rev-parse --git-common-dir 2>/dev/null)" ]]; then
      is_linked_worktree=1
    fi

    if [[ ${#targets[@]} -eq 0 ]]; then
      if [[ $is_linked_worktree -eq 1 ]]; then
        targets=("$current_wt")
      else
        echo "gwt: cannot remove main repository; please specify a worktree" >&2
        return 1
      fi
    else
      local resolved_targets=()
      local t
      for t in "${targets[@]}"; do
        if [[ -d "$t" ]]; then
          resolved_targets+=("$(cd "$t" 2>/dev/null && pwd -P)")
        else
          resolved_targets+=("$t")
        fi
      done
      targets=("${resolved_targets[@]}")
    fi

    local orig_pwd="$PWD"
    if [[ $is_linked_worktree -eq 1 ]] || [[ "$PWD" != "$main_repo" ]]; then
      cd "$main_repo" || return 1
    fi

    if ! git worktree remove "${flags[@]}" "${targets[@]}"; then
      if [[ "$PWD" != "$orig_pwd" ]]; then
        cd "$orig_pwd" 2>/dev/null
      fi
      return 1
    fi
  }

  _gwt_pull() {
    local override_ide=""
    local skip_install=0
    local args=()

    while [[ $# -gt 0 ]]; do
      case "$1" in
        --ide=*)
          override_ide="${1#--ide=}"
          shift
          ;;
        --ide)
          if [[ $# -lt 2 ]]; then
            echo "gwt: --ide requires an argument" >&2
            return 1
          fi
          override_ide="$2"
          shift 2
          ;;
        --no-install)
          skip_install=1
          shift
          ;;
        *)
          args+=("$1")
          shift
          ;;
      esac
    done

    if [[ ${#args[@]} -ne 1 ]]; then
      echo "gwt: unknown command: 'pull ${args[*]}'" >&2
      return 1
    fi

    local branch="${args[1]}"
    local dir_gwt
    dir_gwt=$(_gwt_get_dir_gwt) || return 1
    mkdir -p "$dir_gwt" || return 1
    git fetch origin "$branch" || return 1
    local dest="$dir_gwt/$branch"
    git worktree add -b "$branch" "$dest" "origin/$branch" || return 1
    cd "$dest" || return 1
    _gwt_init_ide "$override_ide" "$skip_install"
  }

  _gwt_create() {
    local override_ide=""
    local skip_install=0
    local args=()

    while [[ $# -gt 0 ]]; do
      case "$1" in
        --ide=*)
          override_ide="${1#--ide=}"
          shift
          ;;
        --ide)
          if [[ $# -lt 2 ]]; then
            echo "gwt: --ide requires an argument" >&2
            return 1
          fi
          override_ide="$2"
          shift 2
          ;;
        --no-install)
          skip_install=1
          shift
          ;;
        *)
          args+=("$1")
          shift
          ;;
      esac
    done

    if [[ ${#args[@]} -ne 1 ]]; then
      echo "gwt: unknown command: '${args[*]}'" >&2
      return 1
    fi

    local branch="${args[1]}"
    local dir_gwt
    dir_gwt=$(_gwt_get_dir_gwt) || return 1
    mkdir -p "$dir_gwt" || return 1
    local dest="$dir_gwt/$branch"
    git worktree add "$dest" || return 1
    cd "$dest" || return 1
    _gwt_init_ide "$override_ide" "$skip_install"
  }

  _gwt_config() {
    if [[ $# -eq 0 ]]; then
      local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
      local config_file="$config_dir/config"
      local has_ide=0
      if [[ -f "$config_file" ]]; then
        local line key
        while IFS= read -r line || [[ -n "$line" ]]; do
          [[ -z "$line" || "$line" == \#* ]] && continue
          if [[ "$line" == *"="* ]]; then
            key="${line%%=*}"
            if [[ "$key" == "ide" ]]; then
              has_ide=1
            fi
          fi
          echo "$line"
        done < "$config_file"
      fi
      if [[ $has_ide -eq 0 ]]; then
        echo "ide=nvim (default)"
      fi
      return 0
    fi

    case "$1" in
      get)
        shift
        if [[ $# -ne 1 ]]; then
          echo "gwt: usage: gwt config get <key>" >&2
          return 1
        fi
        local key="$1"
        if [[ "$key" == "ide" ]]; then
          _gwt_get_ide
          return 0
        fi
        local val
        val=$(_gwt_get_config "$key")
        if [[ -n "$val" ]]; then
          echo "$val"
          return 0
        else
          echo "gwt: config key '$key' not found" >&2
          return 1
        fi
        ;;
      set)
        shift
        if [[ $# -lt 2 ]]; then
          echo "gwt: usage: gwt config set <key> <value>" >&2
          return 1
        fi
        local key="$1"
        shift
        local val="$*"
        _gwt_save_config "$key" "$val"
        echo "gwt: set $key to $val"
        return 0
        ;;
      unset|--unset|remove|rm)
        shift
        if [[ $# -ne 1 ]]; then
          echo "gwt: usage: gwt config unset <key>" >&2
          return 1
        fi
        local key="$1"
        _gwt_unset_config "$key"
        echo "gwt: unset $key"
        return 0
        ;;
      *)
        if [[ $# -eq 1 ]]; then
          local key="$1"
          if [[ "$key" == "ide" ]]; then
            _gwt_get_ide
            return 0
          fi
          local val
          val=$(_gwt_get_config "$key")
          if [[ -n "$val" ]]; then
            echo "$val"
            return 0
          else
            echo "gwt: config key '$key' not found" >&2
            return 1
          fi
        else
          local key="$1"
          shift
          local val="$*"
          _gwt_save_config "$key" "$val"
          echo "gwt: set $key to $val"
          return 0
        fi
        ;;
    esac
  }

  _gwt_launch_ide() {
    local override_ide="$1"
    local ide_cmd="${override_ide:-$(_gwt_get_ide)}"
    if [[ "${ide_cmd:l}" == "none" ]]; then
      return 0
    fi
    eval "$ide_cmd"
  }

  _gwt_init_ide() {
    local override_ide="$1"
    local skip_install="${2:-0}"
    if [[ "$skip_install" -eq 0 ]]; then
      if [ -f yarn.lock ]; then
        yarn
      fi
    fi
    _gwt_launch_ide "$override_ide"
  }

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
      return 1
    fi

    echo "Upgrading gwt at $gwt_dir..."
    if git -C "$gwt_dir" pull "$@"; then
      if [[ -f "$gwt_dir/gwt.sh" ]]; then
        source "$gwt_dir/gwt.sh"
      fi
      return 0
    else
      return 1
    fi
  }

  {
    case "$1" in
      config)
        shift
        _gwt_config "$@"
        ;;
      ide)
        shift
        if [[ $# -eq 0 ]]; then
          _gwt_config get ide
        else
          _gwt_config set ide "$@"
        fi
        ;;
      switch|s)
        shift
        _gwt_switch "$@"
        ;;
      cd)
        shift
        _gwt_cd "$@"
        ;;
      main|m)
        shift
        _gwt_main "$@"
        ;;
      list|ls)
        shift
        _gwt_ls "$@"
        ;;
      remove|rm)
        shift
        _gwt_remove "$@"
        ;;
      pull|p)
        shift
        _gwt_pull "$@"
        ;;
      upgrade)
        shift
        _gwt_upgrade "$@"
        ;;
      add)
        shift
        _gwt_create "$@"
        ;;
      *)
        _gwt_create "$@"
        ;;
    esac
  } always {
    unfunction _gwt_remove _gwt_pull _gwt_create _gwt_init_ide _gwt_launch_ide _gwt_cd _gwt_main _gwt_switch _gwt_ls _gwt_find_worktrees _gwt_is_unsuitable_path _gwt_get_configured_parent _gwt_save_configured_parent _gwt_get_dir_gwt _gwt_get_config _gwt_save_config _gwt_unset_config _gwt_get_ide _gwt_config _gwt_upgrade 2>/dev/null
  }
}

