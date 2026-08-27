#!/bin/sh
#
# gwt installer
# https://github.com/sjwall/gwt
#
# Usage:
#   curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/sjwall/gwt/main/install.sh | sh
#

set -e

# Configuration
REPO_URL="${GWT_REPO_URL:-https://github.com/sjwall/gwt.git}"
RAW_URL="${GWT_RAW_URL:-https://raw.githubusercontent.com/sjwall/gwt/main}"
BRANCH="${GWT_BRANCH:-main}"
INSTALL_DIR="${GWT_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/gwt}"

# Setup colors if running in a terminal
if [ -t 1 ]; then
  BOLD="\033[1m"
  GREEN="\033[32m"
  BLUE="\033[34m"
  YELLOW="\033[33m"
  RED="\033[31m"
  RESET="\033[0m"
else
  BOLD=""
  GREEN=""
  BLUE=""
  YELLOW=""
  RED=""
  RESET=""
fi

info() {
  printf "${BLUE}==>${RESET} ${BOLD}%s${RESET}\n" "$1"
}

success() {
  printf "${GREEN}==>${RESET} ${BOLD}%s${RESET}\n" "$1"
}

warn() {
  printf "${YELLOW}warning:${RESET} %s\n" "$1"
}

error() {
  printf "${RED}error:${RESET} %s\n" "$1" >&2
}

IS_UPGRADE=0

# Create parent directory if needed
mkdir -p "$(dirname "$INSTALL_DIR")"

# Clone or update repository
if [ -d "$INSTALL_DIR/.git" ]; then
  IS_UPGRADE=1
  info "Existing git repository found at $INSTALL_DIR. Updating..."
  if command -v git >/dev/null 2>&1; then
    (
      cd "$INSTALL_DIR"
      git fetch origin "$BRANCH" 2>/dev/null || true
      git checkout -q "$BRANCH" 2>/dev/null || true
      git pull --ff-only origin "$BRANCH" 2>/dev/null || git pull --ff-only 2>/dev/null || warn "Could not fast-forward update git repository. Using existing files."
    )
  else
    warn "git command not found. Keeping existing repository."
  fi
elif [ -d "$INSTALL_DIR" ] && [ -f "$INSTALL_DIR/gwt.sh" ]; then
  IS_UPGRADE=1
  info "Existing installation found at $INSTALL_DIR. Updating..."
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$RAW_URL/gwt.sh" -o "$INSTALL_DIR/gwt.sh"
    curl -fsSL "$RAW_URL/README.adoc" -o "$INSTALL_DIR/README.adoc" 2>/dev/null || true
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$INSTALL_DIR/gwt.sh" "$RAW_URL/gwt.sh"
    wget -qO "$INSTALL_DIR/README.adoc" "$RAW_URL/README.adoc" 2>/dev/null || true
  else
    warn "Neither curl nor wget was found to update files. Keeping existing files."
  fi
else
  info "Installing gwt to $INSTALL_DIR..."
  if command -v git >/dev/null 2>&1; then
    info "Cloning repository..."
    git clone --depth=1 --branch "$BRANCH" "$REPO_URL" "$INSTALL_DIR"
  elif command -v curl >/dev/null 2>&1; then
    info "Downloading gwt.sh..."
    mkdir -p "$INSTALL_DIR"
    curl -fsSL "$RAW_URL/gwt.sh" -o "$INSTALL_DIR/gwt.sh"
    curl -fsSL "$RAW_URL/README.adoc" -o "$INSTALL_DIR/README.adoc" 2>/dev/null || true
  elif command -v wget >/dev/null 2>&1; then
    info "Downloading gwt.sh..."
    mkdir -p "$INSTALL_DIR"
    wget -qO "$INSTALL_DIR/gwt.sh" "$RAW_URL/gwt.sh"
    wget -qO "$INSTALL_DIR/README.adoc" "$RAW_URL/README.adoc" 2>/dev/null || true
  else
    error "Neither git, curl, nor wget was found. Please install one of them and try again."
    exit 1
  fi
fi

# Verify gwt.sh exists
if [ ! -f "$INSTALL_DIR/gwt.sh" ]; then
  error "Failed to locate $INSTALL_DIR/gwt.sh"
  exit 1
fi

chmod +x "$INSTALL_DIR/gwt.sh"

# Configure shell profile
detect_profile() {
  if [ -n "$ZDOTDIR" ] && [ -f "$ZDOTDIR/.zshrc" ]; then
    echo "$ZDOTDIR/.zshrc"
  elif [ -f "$HOME/.zshrc" ]; then
    echo "$HOME/.zshrc"
  elif [ -f "$HOME/.bashrc" ]; then
    echo "$HOME/.bashrc"
  elif [ -f "$HOME/.profile" ]; then
    echo "$HOME/.profile"
  else
    echo "$HOME/.zshrc"
  fi
}

PROFILE_FILE="$(detect_profile)"

case "$INSTALL_DIR" in
  "$HOME"/*)
    FORMATTED_PATH="\$HOME/${INSTALL_DIR#"$HOME"/}"
    ;;
  *)
    FORMATTED_PATH="$INSTALL_DIR"
    ;;
esac

SOURCE_LINE="[ -f \"$FORMATTED_PATH/gwt.sh\" ] && source \"$FORMATTED_PATH/gwt.sh\""

if [ -f "$PROFILE_FILE" ] && grep -q "gwt.sh" "$PROFILE_FILE" 2>/dev/null; then
  info "gwt is already configured in $PROFILE_FILE"
else
  info "Adding source line to $PROFILE_FILE..."
  mkdir -p "$(dirname "$PROFILE_FILE")"
  {
    echo ""
    echo "# gwt (git worktree helper)"
    echo "$SOURCE_LINE"
  } >> "$PROFILE_FILE"
  success "Added gwt to $PROFILE_FILE"
fi

echo ""
if [ "$IS_UPGRADE" -eq 1 ]; then
  success "gwt upgraded successfully!"
else
  success "gwt installed successfully!"
fi
echo ""
echo "To start using gwt, reload your shell configuration:"
echo "  source $PROFILE_FILE"
echo ""
echo "Or start a new terminal session."
