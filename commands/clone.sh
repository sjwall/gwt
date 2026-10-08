#!/bin/zsh
_gwt_clone() {
  local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/gwt"
  local repos_file="$config_dir/repos"
  local lockfile="$config_dir/repos.lock"

  # If no arguments provided, show usage
  if [[ $# -eq 0 ]]; then
    echo "Usage: gwt clone [--ide IDE] [--agent[=AGENT]|-a[=AGENT]] [--no-install] [git clone args...]" >&2
    echo "       gwt clone passes arguments through to git clone and then tracks the repository" >&2
    return 39
  fi

  # Parse options similar to create.sh
  local override_ide=""
  local override_agent=""
  local use_agent=0
  local skip_install=0
  local raw=("$@")
  local -a git_args=()
  local i=1

  while (( i <= $#raw )); do
    local arg="${raw[i]}"
    case "$arg" in
      --ide=*)
        override_ide="${arg#--ide=}"
        (( i++ ))
        ;;
      --ide)
        if (( i + 1 > $#raw )); then
          echo "gwt: --ide requires an argument" >&2
          return 23
        fi
        override_ide="${raw[i+1]}"
        (( i += 2 ))
        ;;
      --no-install)
        skip_install=1
        (( i++ ))
        ;;
      --agent=*)
        use_agent=1
        override_agent="${arg#--agent=}"
        (( i++ ))
        ;;
      -a=*)
        use_agent=1
        override_agent="${arg#-a=}"
        (( i++ ))
        ;;
      --agent|-a)
        use_agent=1
        if (( i + 1 > $#raw )); then
          echo "gwt: --agent requires an argument" >&2
          return 44
        fi
        override_agent="${raw[i+1]}"
        (( i += 2 ))
        ;;
      *)
        git_args+=("$arg")
        (( i++ ))
        ;;
    esac
  done

  # If no git clone arguments provided, show usage
  if [[ ${#git_args[@]} -eq 0 ]]; then
    echo "Usage: gwt clone [--ide IDE] [--agent[=AGENT]|-a[=AGENT]] [--no-install] [git clone args...]" >&2
    echo "       gwt clone passes arguments through to git clone and then tracks the repository" >&2
    return 39
  fi

  # Store current directory to look for new repositories
  local current_dir="$PWD"
  local current_time=$(date +%s)

  # Execute git clone with the provided arguments
  echo "Cloning repository with: git clone ${git_args[*]}"
  git clone "${git_args[@]}" || return 40

  # Find the newly cloned repository by looking for the newest .git directory
  local repo_path=""
  local newest_time=0

  # Check if the last argument was a directory that now exists and is a git repo
  local last_arg="${git_args[-1]}"
  if [[ -d "$last_arg" && -d "$last_arg/.git" ]]; then
    repo_path="$last_arg"
  else
    # Look for the newest .git directory in the current directory
    for dir in *(N); do
      if [[ -d "$dir/.git" ]]; then
        local dir_time
        if [[ "$OSTYPE" == "darwin"* ]]; then
            # macOS: use -f format with %m (time of last modification)
            dir_time=$(stat -f %m "$dir")
        else
            # Linux: use -c format with %Y (time of last modification, seconds since epoch)
            dir_time=$(stat -c %Y "$dir")
        fi
        if (( dir_time > newest_time )); then
          newest_time=$dir_time
          repo_path="$dir"
        fi
      fi
    done
  fi

  # If we didn't find a repository, try looking in subdirectories
  if [[ -z "$repo_path" || ! -d "$repo_path/.git" ]]; then
    # Look for any .git directory that was created recently
    find . -maxdepth 2 -name ".git" -type d -newer "$current_dir" 2>/dev/null | while read git_dir; do
      local candidate="${git_dir%/.git}"
      if [[ -d "$candidate/.git" ]]; then
        local dir_time=$(stat -c %Y "$candidate")
        if (( dir_time > newest_time )); then
          newest_time=$dir_time
          repo_path="$candidate"
        fi
      fi
    done
  fi

  # If still no repository found, try the last argument as a path
  if [[ -z "$repo_path" || ! -d "$repo_path/.git" ]]; then
    if [[ -d "$last_arg" ]]; then
      repo_path="$last_arg"
    fi
  fi

  # Verify it's a git repository
  if [[ -z "$repo_path" || ! -d "$repo_path/.git" ]]; then
    echo "gwt: could not determine cloned repository path" >&2
    return 41
  fi

  # Get the absolute path
  repo_path=$(realpath "$repo_path")

  # Verify it's a git repository
  if [[ ! -d "$repo_path/.git" ]]; then
    echo "gwt: cloned path is not a git repository: $repo_path" >&2
    return 42
  fi

  # Track the repository
  source "$gwt_dir/utils/file-lock.sh"
  _gwt_acquire_lock "$lockfile" || return 1

  mkdir -p "$config_dir"
  if [[ ! -f "$repos_file" ]] || ! grep -Fxq "$repo_path" "$repos_file" 2>/dev/null; then
    echo "$repo_path" >> "$repos_file"
  fi

  _gwt_release_lock "$lockfile"

  echo "Tracked repository: $repo_path"

  # Change to the cloned repository
  cd "$repo_path" || return 1

  if [[ "$skip_install" -eq 0 ]]; then
    source "$gwt_dir/utils/pull-dependencies.sh"
    _gwt_pull_dependencies
  fi

  # Launch IDE or agent if specified
  if [[ $use_agent -eq 1 ]]; then
    source "$gwt_dir/utils/agent-launch.sh"
    _gwt_launch_agent "$override_agent"
  elif [[ -n "$override_ide" ]]; then
    source "$gwt_dir/utils/ide-launch.sh"
    _gwt_launch_ide "$override_ide"
  fi

  return 0
}
