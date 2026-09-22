#!/bin/zsh
# File locking utility for gwt
# Usage: _gwt_acquire_lock <lockfile> [timeout]
#        _gwt_release_lock <lockfile>

_gwt_acquire_lock() {
  local lockfile="$1"
  local timeout=${2:-30}
  local start_time=$(date +%s)
  local lock_dir="$(dirname "$lockfile")"
  
  mkdir -p "$lock_dir"
  
  while [[ -f "$lockfile" ]]; do
    local current_time=$(date +%s)
    local elapsed=$((current_time - start_time))
    
    if [[ $elapsed -ge $timeout ]]; then
      echo "gwt: timeout waiting for lock on $lockfile" >&2
      return 1
    fi
    
    sleep 0.1
  done
  
  # Create lock file with process ID
  echo $$ > "$lockfile"
  return 0
}

_gwt_release_lock() {
  local lockfile="$1"
  
  if [[ -f "$lockfile" ]]; then
    local lock_pid=$(cat "$lockfile" 2>/dev/null || echo "")
    # Only remove if we own the lock
    if [[ "$lock_pid" == "$$" ]]; then
      rm -f "$lockfile"
    fi
  fi
  return 0
}
