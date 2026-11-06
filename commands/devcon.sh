#!/bin/zsh
_gwt_devcon() {
  if [[ -z "$main_repo" ]]; then
    echo "gwt: not inside a git repository" >&2
    return 56
  fi

  source "$gwt_dir/commands/track.sh"
  _gwt_track

  local configFile=""
  if [ -f ".devcontainer.json" ]; then
    configFile="./.devcontainer.json"
  elif [ -f "./.devcontainer/devcontainer.json" ]; then
    configFile="./.devcontainer/devcontainer.json"
  fi

  if [[ -z "$configFile" ]] then
    echo "gwt: no devcontainer configuration. Please setup first."
    return 57
  fi

  if ! command -v jq >/dev/null 2>&1; then
    echo "gwt: jq is not installed, unable to update the devcontainer configuration." >&2
    return 62
  fi

  # The devcontainer's "name" field is untrusted input (it comes from the
  # repository being worked on), so it must be passed to jq as data via
  # --arg, never interpolated into the jq program string.
  local gwtName="gwt-$(jq -r '.name' "$configFile")"

  local mounts="$(jq '.mounts' "$configFile")"
  if [ "$mounts" = "null" ] || [ "$mounts" = "[]" ]; then
    echo "gwt: setting $gwtName mount"
    source "$gwt_dir/utils/jq-inplace.sh"
    _gwt_jq_inplace "$configFile" --arg name "$gwtName" \
      '.mounts = [{"type": "volume", "source": $name, "target": ("/workspaces/" + $name)}]' || return $?
  else
    local exists="$(jq --arg name "$gwtName" '.mounts[] | select(.source == $name)' "$configFile")"
    if [[ -z "$exists" ]]; then
      echo "gwt: appending $gwtName mount"
      source "$gwt_dir/utils/jq-inplace.sh"
      _gwt_jq_inplace "$configFile" --arg name "$gwtName" \
        '.mounts += [{"type": "volume", "source": $name, "target": ("/workspaces/" + $name)}]' || return $?
    else
      echo "gwt: $gwtName mount already set."
    fi
  fi

  if command -v docker >/dev/null 2>&1; then
    local volume="$(docker volume ls --filter "name=$gwtName" --format '{{.Name}}')"
    if [[ -z "$volume" ]]; then
        docker volume create "$gwtName" >/dev/null || return 63
    else
      echo "gwt: docker volume $gwtName already exists."
    fi
  else
    echo "gwt: docker is not installed, unable to create volume."
  fi
}
