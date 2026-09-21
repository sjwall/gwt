#!/bin/zsh
_gwt_config() {
  source "$gwt_dir/utils/config-get.sh"
  source "$gwt_dir/utils/config-save.sh"
  source "$gwt_dir/utils/config-unset.sh"
  source "$gwt_dir/utils/ide-get.sh"
  source "$gwt_dir/utils/agent-get.sh"

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
        return 29
      fi
      local key="$1"
      if [[ "$key" == "ide" ]]; then
        _gwt_get_ide
        return 0
      fi
      if [[ "$key" == "agent" ]]; then
        local agent
        agent=$(_gwt_get_agent)
        if [[ -n "$agent" ]]; then
          echo "$agent"
          return 0
        else
          echo "gwt: config key 'agent' not found" >&2
          return 30
        fi
      fi
      local val
      val=$(_gwt_get_config "$key")
      if [[ -n "$val" ]]; then
        echo "$val"
        return 0
      else
        echo "gwt: config key '$key' not found" >&2
        return 30
      fi
      ;;
    set)
      shift
      if [[ $# -lt 2 ]]; then
        echo "gwt: usage: gwt config set <key> <value>" >&2
        return 31
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
        return 32
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
        if [[ "$key" == "agent" ]]; then
          local agent
          agent=$(_gwt_get_agent)
          if [[ -n "$agent" ]]; then
            echo "$agent"
            return 0
          else
            echo "gwt: config key 'agent' not found" >&2
            return 33
          fi
        fi
        local val
        val=$(_gwt_get_config "$key")
        if [[ -n "$val" ]]; then
          echo "$val"
          return 0
        else
          echo "gwt: config key '$key' not found" >&2
          return 33
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
