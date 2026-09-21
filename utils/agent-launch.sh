#!/bin/zsh
_gwt_launch_agent() {
  source "$gwt_dir/utils/agent-get.sh"
  source "$gwt_dir/utils/config-save.sh"
  local override_agent="$1"
  local agent_cmd="$override_agent"

  if [[ -z "$agent_cmd" ]]; then
    agent_cmd=$(_gwt_get_agent)
  fi

  if [[ -z "$agent_cmd" ]]; then
    local user_agent
    read -r "user_agent?Enter command to launch agent: "
    if [[ -n "$user_agent" ]]; then
      _gwt_save_config "agent" "$user_agent"
      agent_cmd="$user_agent"
    else
      echo "gwt: no agent configured" >&2
      return 46
    fi
  fi

  if [[ "${agent_cmd:l}" == "none" ]]; then
    return 0
  fi

  eval "$agent_cmd"
}
