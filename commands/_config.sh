#!/bin/zsh
# Completion for gwt config command

_gwt_comp_config() {
  (( $+functions[_describe] )) || return 0
  local -a config_actions config_keys
  config_actions=(
    'get:Get configuration key value'
    'set:Set configuration key value'
    'unset:Remove configuration key'
    'remove:Alias for unset'
    'rm:Alias for unset'
    'ide:Get or set configured IDE'
  )
  config_keys=('ide:Configured default IDE' 'agent:Configured agent command')

  local config_file="${XDG_CONFIG_HOME:-$HOME/.config}/gwt/config"
  if [[ -f "$config_file" ]]; then
    local line k
    while IFS= read -r line || [[ -n "$line" ]]; do
      [[ -z "$line" || "$line" == \#* || "$line" != *"="* ]] && continue
      k="${line%%=*}"
      [[ "$k" != "ide" && "$k" != "agent" ]] && config_keys+=("$k")
    done < "$config_file"
  fi

  if (( CURRENT == 3 )); then
    _describe -t actions 'config action' config_actions
    _describe -t keys 'config key' config_keys
  elif (( CURRENT == 4 )); then
    case "${words[3]}" in
      get|unset|remove|rm)
        _describe -t keys 'config key' config_keys
        ;;
      set)
        _describe -t keys 'config key' config_keys
        ;;
      ide)
        _gwt_comp_ides
        ;;
      *)
        if [[ "${words[3]}" == "ide" ]]; then
          _gwt_comp_ides
        fi
        ;;
    esac
  elif (( CURRENT == 5 )); then
    if [[ "${words[3]}" == "set" && "${words[4]}" == "ide" ]]; then
      _gwt_comp_ides
    fi
  fi
}
