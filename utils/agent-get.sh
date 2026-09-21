#!/bin/zsh
_gwt_get_agent() {
  source "$gwt_dir/utils/config-get.sh"
  local agent
  agent=$(_gwt_get_config "agent")
  if [[ -n "$agent" ]]; then
    echo "$agent"
  elif [[ -n "$GWT_AGENT" ]]; then
    echo "$GWT_AGENT"
  fi
}
