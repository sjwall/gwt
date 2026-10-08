#!/bin/zsh
# Pull the dependencies for the workspace
# Usage: _gwt_pull_dependencies
_gwt_pull_dependencies() {
  if [ -f "yarn.lock" ]; then
    if command -v yarn &> /dev/null; then
      yarn
    else
      # TODO: log warning
    fi
  elif [ -f "package-lock.json" ]; then
    if command -v npm &> /dev/null; then
      npm install
    else
      # TODO: log warning
    fi
  fi
  if [ -f "Cargo.lock" ]; then
    if command -v cargo &> /dev/null; then
      cargo fetch
    else
      # TODO: log warning
    fi
  fi
}
