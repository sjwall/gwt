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
INSTALL_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/gwt"

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

# Download and extract the full repository tree (commands/, utils/, etc.) from
# a GitHub tarball. Raw per-file downloads cannot enumerate directories, and a
# hand-maintained file list drifts as files are added. Falls back to the tarball
# URL override or derives the codeload URL from REPO_URL.
#
# Usage: download_gwt_tree <destination-dir>
# Returns non-zero if the download or extraction fails.
download_gwt_tree() {
  _dest="$1"
  _tmp="${TMPDIR:-/tmp}/gwt-install-$$"
  ( umask 077 && mkdir -p "$_tmp" ) || return 1

  _cleanup() { rm -rf "$_tmp"; }
  trap '_cleanup' EXIT INT TERM

  _tarball="$_tmp/gwt.tar.gz"
  _tarball_url="${GWT_TARBALL_URL:-}"
  if [ -z "$_tarball_url" ]; then
    _slug="$(printf '%s\n' "$REPO_URL" | sed -e 's|\.git$||' -e 's|.*github\.com/||')"
    case "$_slug" in
      */*) _tarball_url="https://codeload.github.com/$_slug/tar.gz/refs/heads/$BRANCH" ;;
      *)
        error "Cannot derive a tarball URL from GWT_REPO_URL='$REPO_URL'."
        error "Set GWT_TARBALL_URL to a repository tarball, or install git."
        return 1
        ;;
    esac
  fi

  info "Downloading repository archive from $_tarball_url"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$_tarball_url" -o "$_tarball" || return 1
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$_tarball" "$_tarball_url" || return 1
  else
    return 1
  fi

  tar -xzf "$_tarball" -C "$_tmp" || return 1

  # The archive extracts into a single top-level directory (e.g. gwt-main).
  _src_dir=""
  for _d in "$_tmp"/*/; do
    if [ -d "$_d" ]; then
      _src_dir="$_d"
      break
    fi
  done
  if [ -z "$_src_dir" ] || [ ! -f "${_src_dir}gwt.sh" ]; then
    error "Downloaded archive did not contain the expected gwt files."
    return 1
  fi

  mkdir -p "$_dest"
  # Copy the full tree, including dotfiles (e.g. .agents/skills/gwt/SKILL.md).
  cp -R "${_src_dir}." "$_dest/" || return 1

  rm -rf "$_tmp"
  trap - EXIT INT TERM
}

# Parse command line options
SKILLS_ARG=""
DRY_RUN=0

while [ $# -gt 0 ]; do
  case "$1" in
    --skills=*)
      SKILLS_ARG="${1#--skills=}"
      shift
      ;;
    --skills)
      if [ $# -gt 1 ]; then
        SKILLS_ARG="$2"
        shift 2
      else
        error "--skills requires an argument"
        exit 1
      fi
      ;;
    --no-skills)
      SKILLS_ARG="none"
      shift
      ;;
    --dir=*)
      INSTALL_DIR="${1#--dir=}"
      shift
      ;;
    --dir)
      if [ $# -gt 1 ]; then
        INSTALL_DIR="$2"
        shift 2
      else
        error "--dir requires an argument"
        exit 1
      fi
      ;;
    -n|--dry-run)
      DRY_RUN=1
      shift
      ;;
    --dry-run=*)
      case "${1#--dry-run=}" in
        0|false|no) DRY_RUN=0 ;;
        *) DRY_RUN=1 ;;
      esac
      shift
      ;;
    -h|--help)
      echo "Usage: install.sh [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --skills=<targets>   Symlink skills to global agent directories"
      echo "                       (comma-separated: agents, opencode, claude, gemini, all, none)"
      echo "  --no-skills          Skip symlinking skills (same as --skills=none)"
      echo "  --dir=<path>         Installation directory (default: ~/.local/share/gwt)"
      echo "  -n, --dry-run        Perform a dry run without making any changes"
      echo "  -h, --help           Show this help message"
      exit 0
      ;;
    *)
      warn "Unknown option: $1"
      shift
      ;;
  esac
done

if [ "$DRY_RUN" -eq 1 ]; then
  info "Running in dry-run mode. No changes will be made."
fi

IS_UPGRADE=0

# Create parent directory if needed
if [ "$DRY_RUN" -eq 1 ]; then
  if [ ! -d "$(dirname "$INSTALL_DIR")" ]; then
    info "Would create directory $(dirname "$INSTALL_DIR")"
  fi
else
  mkdir -p "$(dirname "$INSTALL_DIR")"
fi

# Clone or update repository
if [ -d "$INSTALL_DIR/.git" ]; then
  IS_UPGRADE=1
  if [ "$DRY_RUN" -eq 1 ]; then
    info "Existing git repository found at $INSTALL_DIR."
    if command -v git >/dev/null 2>&1; then
      info "Would update git repository (git pull origin $BRANCH)"
    else
      warn "git command not found. Keeping existing repository."
    fi
  else
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
  fi
elif [ -d "$INSTALL_DIR" ] && [ -f "$INSTALL_DIR/gwt.sh" ]; then
  IS_UPGRADE=1
  if [ "$DRY_RUN" -eq 1 ]; then
    info "Existing installation found at $INSTALL_DIR."
    if command -v curl >/dev/null 2>&1; then
      info "Would download updated files from $RAW_URL using curl"
    elif command -v wget >/dev/null 2>&1; then
      info "Would download updated files from $RAW_URL using wget"
    else
      warn "Neither curl nor wget was found to update files. Keeping existing files."
    fi
  else
    info "Existing installation found at $INSTALL_DIR. Updating..."
    if command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1; then
      download_gwt_tree "$INSTALL_DIR" || {
        error "Failed to download gwt updates. Please install git and try again."
        exit 1
      }
    else
      warn "Neither curl nor wget was found to update files. Keeping existing files."
    fi
  fi
else
  if [ "$DRY_RUN" -eq 1 ]; then
    info "Would install gwt to $INSTALL_DIR..."
    if command -v git >/dev/null 2>&1; then
      info "Would clone repository from $REPO_URL ($BRANCH)"
    elif command -v curl >/dev/null 2>&1; then
      info "Would download files from $RAW_URL using curl"
    elif command -v wget >/dev/null 2>&1; then
      info "Would download files from $RAW_URL using wget"
    else
      error "Neither git, curl, nor wget was found. Please install one of them and try again."
      exit 1
    fi
  else
    info "Installing gwt to $INSTALL_DIR..."
    if command -v git >/dev/null 2>&1; then
      info "Cloning repository..."
      git clone --depth=1 --branch "$BRANCH" "$REPO_URL" "$INSTALL_DIR"
    elif command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1; then
      download_gwt_tree "$INSTALL_DIR" || {
        error "Failed to download gwt. Please install git and try again."
        exit 1
      }
    else
      error "Neither git, curl, nor wget was found. Please install one of them and try again."
      exit 1
    fi
  fi
fi

if [ "$DRY_RUN" -eq 0 ]; then
  # Verify the install is functional: gwt.sh sources commands/*.sh and
  # utils/*.sh on every invocation, so those directories must exist too.
  _missing=""
  [ ! -f "$INSTALL_DIR/gwt.sh" ] && _missing="gwt.sh"
  [ -z "$_missing" ] && [ ! -d "$INSTALL_DIR/commands" ] && _missing="commands/"
  [ -z "$_missing" ] && [ ! -d "$INSTALL_DIR/utils" ] && _missing="utils/"
  if [ -n "$_missing" ]; then
    error "Install is incomplete: missing $_missing in $INSTALL_DIR"
    exit 2
  fi

  chmod +x "$INSTALL_DIR/gwt.sh"
  chmod +x "$INSTALL_DIR/skills.sh" 2>/dev/null || true
fi

# Configure shell profile
detect_profile() {
  if [ -n "$ZDOTDIR" ] && [ -f "$ZDOTDIR/.zshrc" ]; then
    echo "$ZDOTDIR/.zshrc"
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
  if [ "$DRY_RUN" -eq 1 ]; then
    info "Would add source line to $PROFILE_FILE"
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
fi

# Skills management
_skills_script=""
_cleanup_tmp_skills=0

if [ -f "$INSTALL_DIR/skills.sh" ]; then
  _skills_script="$INSTALL_DIR/skills.sh"
elif [ -f "$(dirname "$0")/skills.sh" ]; then
  _skills_script="$(dirname "$0")/skills.sh"
elif [ -f "./skills.sh" ]; then
  _skills_script="./skills.sh"
fi

if [ -z "$_skills_script" ] || [ ! -f "$_skills_script" ]; then
  _tmp_skills="$(mktemp 2>/dev/null || echo "/tmp/gwt-skills-$$.sh")"
  if command -v curl >/dev/null 2>&1; then
    if curl -fsSL "$RAW_URL/skills.sh" -o "$_tmp_skills" 2>/dev/null; then
      _skills_script="$_tmp_skills"
      _cleanup_tmp_skills=1
    fi
  elif command -v wget >/dev/null 2>&1; then
    if wget -qO "$_tmp_skills" "$RAW_URL/skills.sh" 2>/dev/null; then
      _skills_script="$_tmp_skills"
      _cleanup_tmp_skills=1
    fi
  fi
fi

if [ -n "$_skills_script" ] && [ -f "$_skills_script" ]; then
  _dry_flag=""
  [ "$DRY_RUN" -eq 1 ] && _dry_flag="-n"

  if [ -n "$SKILLS_ARG" ]; then
    sh "$_skills_script" --dir="$INSTALL_DIR" $_dry_flag "$SKILLS_ARG"
  elif [ "$IS_UPGRADE" -eq 1 ]; then
    if [ -e /dev/tty ] && [ -r /dev/tty ] && [ -w /dev/tty ] || [ -t 0 ]; then
      sh "$_skills_script" --dir="$INSTALL_DIR" $_dry_flag --prompt
    else
      sh "$_skills_script" --dir="$INSTALL_DIR" $_dry_flag --sync
    fi
  elif [ -e /dev/tty ] && [ -r /dev/tty ] && [ -w /dev/tty ] || [ -t 0 ]; then
    sh "$_skills_script" --dir="$INSTALL_DIR" $_dry_flag --prompt
  else
    sh "$_skills_script" --dir="$INSTALL_DIR" $_dry_flag none
  fi

  if [ "$_cleanup_tmp_skills" -eq 1 ]; then
    rm -f "$_tmp_skills"
  fi
fi

echo ""
if [ "$DRY_RUN" -eq 1 ]; then
  info "Dry run complete! No changes were made."
else
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
fi
