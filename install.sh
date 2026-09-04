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
    if command -v curl >/dev/null 2>&1; then
      curl -fsSL "$RAW_URL/gwt.sh" -o "$INSTALL_DIR/gwt.sh"
      curl -fsSL "$RAW_URL/_gwt" -o "$INSTALL_DIR/_gwt" 2>/dev/null || true
      curl -fsSL "$RAW_URL/README.adoc" -o "$INSTALL_DIR/README.adoc" 2>/dev/null || true
      mkdir -p "$INSTALL_DIR/.agents/skills/gwt"
      curl -fsSL "$RAW_URL/.agents/skills/gwt/SKILL.md" -o "$INSTALL_DIR/.agents/skills/gwt/SKILL.md" 2>/dev/null || true
    elif command -v wget >/dev/null 2>&1; then
      wget -qO "$INSTALL_DIR/gwt.sh" "$RAW_URL/gwt.sh"
      wget -qO "$INSTALL_DIR/_gwt" "$RAW_URL/_gwt" 2>/dev/null || true
      wget -qO "$INSTALL_DIR/README.adoc" "$RAW_URL/README.adoc" 2>/dev/null || true
      mkdir -p "$INSTALL_DIR/.agents/skills/gwt"
      wget -qO "$INSTALL_DIR/.agents/skills/gwt/SKILL.md" "$RAW_URL/.agents/skills/gwt/SKILL.md" 2>/dev/null || true
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
    elif command -v curl >/dev/null 2>&1; then
      info "Downloading gwt.sh..."
      mkdir -p "$INSTALL_DIR"
      curl -fsSL "$RAW_URL/gwt.sh" -o "$INSTALL_DIR/gwt.sh"
      curl -fsSL "$RAW_URL/_gwt" -o "$INSTALL_DIR/_gwt" 2>/dev/null || true
      curl -fsSL "$RAW_URL/README.adoc" -o "$INSTALL_DIR/README.adoc" 2>/dev/null || true
      mkdir -p "$INSTALL_DIR/.agents/skills/gwt"
      curl -fsSL "$RAW_URL/.agents/skills/gwt/SKILL.md" -o "$INSTALL_DIR/.agents/skills/gwt/SKILL.md" 2>/dev/null || true
    elif command -v wget >/dev/null 2>&1; then
      info "Downloading gwt.sh..."
      mkdir -p "$INSTALL_DIR"
      wget -qO "$INSTALL_DIR/gwt.sh" "$RAW_URL/gwt.sh"
      wget -qO "$INSTALL_DIR/_gwt" "$RAW_URL/_gwt" 2>/dev/null || true
      wget -qO "$INSTALL_DIR/README.adoc" "$RAW_URL/README.adoc" 2>/dev/null || true
      mkdir -p "$INSTALL_DIR/.agents/skills/gwt"
      wget -qO "$INSTALL_DIR/.agents/skills/gwt/SKILL.md" "$RAW_URL/.agents/skills/gwt/SKILL.md" 2>/dev/null || true
    else
      error "Neither git, curl, nor wget was found. Please install one of them and try again."
      exit 1
    fi
  fi
fi

if [ "$DRY_RUN" -eq 0 ]; then
  # Verify gwt.sh exists
  if [ ! -f "$INSTALL_DIR/gwt.sh" ]; then
    error "Failed to locate $INSTALL_DIR/gwt.sh"
    exit 2
  fi

  chmod +x "$INSTALL_DIR/gwt.sh"
fi

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
is_interactive() {
  if [ -e /dev/tty ] && [ -r /dev/tty ] && [ -w /dev/tty ]; then
    return 0
  fi
  if [ -t 0 ]; then
    return 0
  fi
  return 1
}

LINK_AGENTS=0
LINK_OPENCODE=0
LINK_CLAUDE=0
LINK_GEMINI=0

parse_skills_selection() {
  _input="$1"
  LINK_AGENTS=0
  LINK_OPENCODE=0
  LINK_CLAUDE=0
  LINK_GEMINI=0

  _cleaned="$(printf "%s" "$_input" | tr ',;' '  ' | tr '[:upper:]' '[:lower:]')"

  for _token in $_cleaned; do
    case "$_token" in
      1|agents|agent)
        LINK_AGENTS=1
        ;;
      2|opencode)
        LINK_OPENCODE=1
        ;;
      3|claude)
        LINK_CLAUDE=1
        ;;
      4|gemini|antigravity|agy|yourself)
        LINK_GEMINI=1
        ;;
      5|all)
        LINK_AGENTS=1
        LINK_OPENCODE=1
        LINK_CLAUDE=1
        LINK_GEMINI=1
        ;;
      1-4)
        LINK_AGENTS=1
        LINK_OPENCODE=1
        LINK_CLAUDE=1
        LINK_GEMINI=1
        ;;
      0|6|none|no|n|skip)
        LINK_AGENTS=0
        LINK_OPENCODE=0
        LINK_CLAUDE=0
        LINK_GEMINI=0
        return 0
        ;;
      "")
        ;;
      *)
        warn "Unknown skill target: '$_token'"
        ;;
    esac
  done
}

symlink_skill() {
  _src="$1"
  _dest_dir="$2"
  _name="$(basename "$_src")"
  _target="$_dest_dir/$_name"

  case "$_target" in
    "$HOME"/*)
      _disp_target="~/${_target#"$HOME"/}"
      ;;
    *)
      _disp_target="$_target"
      ;;
  esac
  case "$_src" in
    "$HOME"/*)
      _disp_src="~/${_src#"$HOME"/}"
      ;;
    *)
      _disp_src="$_src"
      ;;
  esac

  if [ "$DRY_RUN" -eq 0 ]; then
    mkdir -p "$_dest_dir"
  fi

  if [ -L "$_target" ]; then
    _current="$(readlink "$_target" 2>/dev/null || true)"
    if [ "$_current" = "$_src" ]; then
      info "Skill '$_name' is already linked in $_disp_target"
      return 0
    fi
    if [ "$DRY_RUN" -eq 1 ]; then
      info "Would update symlink: $_disp_target -> $_disp_src"
    else
      rm -f "$_target"
      ln -s "$_src" "$_target"
      success "Updated symlink: $_disp_target -> $_disp_src"
    fi
  elif [ -e "$_target" ]; then
    warn "$_disp_target exists and is not a symlink; skipping"
  else
    if [ "$DRY_RUN" -eq 1 ]; then
      info "Would symlink: $_disp_target -> $_disp_src"
    else
      ln -s "$_src" "$_target"
      success "Symlinked: $_disp_target -> $_disp_src"
    fi
  fi
}

apply_skills_symlinks() {
  if [ "$LINK_AGENTS" -eq 0 ] && [ "$LINK_OPENCODE" -eq 0 ] && [ "$LINK_CLAUDE" -eq 0 ] && [ "$LINK_GEMINI" -eq 0 ]; then
    return 0
  fi

  _skills_src="$INSTALL_DIR/.agents/skills"
  if [ ! -d "$_skills_src" ] && [ -d "./.agents/skills" ]; then
    _skills_src="$(pwd)/.agents/skills"
  fi

  if [ ! -d "$_skills_src" ] && [ "$DRY_RUN" -eq 0 ]; then
    warn "Skills directory not found at $_skills_src; skipping skill symlinking."
    return 0
  fi

  echo ""
  info "Symlinking skills..."

  link_all_skills() {
    _dest_dir="$1"
    if [ -d "$_skills_src" ]; then
      for _skill in "$_skills_src"/*; do
        [ -d "$_skill" ] || continue
        symlink_skill "$_skill" "$_dest_dir"
      done
    elif [ "$DRY_RUN" -eq 1 ]; then
      symlink_skill "$_skills_src/gwt" "$_dest_dir"
    fi
  }

  if [ "$LINK_AGENTS" -eq 1 ]; then
    _agents_dir="${AGENTS_CONFIG_DIR:-$HOME/.agents}/skills"
    link_all_skills "$_agents_dir"
  fi

  if [ "$LINK_OPENCODE" -eq 1 ]; then
    _opencode_dir="${OPENCODE_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/opencode}/skills"
    link_all_skills "$_opencode_dir"
  fi

  if [ "$LINK_CLAUDE" -eq 1 ]; then
    _claude_dir="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/skills"
    link_all_skills "$_claude_dir"
  fi

  if [ "$LINK_GEMINI" -eq 1 ]; then
    if [ -n "$GEMINI_SKILLS_DIR" ]; then
      link_all_skills "$GEMINI_SKILLS_DIR"
    elif [ -n "$ANTIGRAVITY_SKILLS_DIR" ]; then
      link_all_skills "$ANTIGRAVITY_SKILLS_DIR"
    else
      _agy_dir="${ANTIGRAVITY_CONFIG_DIR:-$HOME/.gemini/antigravity-cli}/skills"
      _gemini_dir="${GEMINI_CONFIG_DIR:-$HOME/.gemini/config}/skills"
      link_all_skills "$_agy_dir"
      link_all_skills "$_gemini_dir"
    fi
  fi
}

# Configure skills symlinks
SKILLS_TARGETS="${SKILLS_ARG:-$GWT_SKILLS}"

if [ -n "$SKILLS_TARGETS" ]; then
  parse_skills_selection "$SKILLS_TARGETS"
elif is_interactive; then
  echo ""
  info "Symlink skills to global agent directories?"
  echo "Select targets to link skills (comma-separated or numbers):"
  echo "  1) agents       (~/.agents/skills)"
  echo "  2) opencode     (~/.config/opencode/skills)"
  echo "  3) claude       (~/.claude/skills)"
  echo "  4) gemini       (~/.gemini/antigravity-cli/skills, ~/.gemini/config/skills)"
  echo "  5) all"
  echo "  6) none"
  echo ""
  if [ -e /dev/tty ] && [ -r /dev/tty ] && [ -w /dev/tty ]; then
    printf "Enter choice(s) [default: none]: " > /dev/tty
    read -r SKILLS_CHOICE < /dev/tty || SKILLS_CHOICE=""
  elif [ -t 0 ]; then
    printf "Enter choice(s) [default: none]: "
    read -r SKILLS_CHOICE || SKILLS_CHOICE=""
  else
    SKILLS_CHOICE="none"
  fi
  [ -z "$SKILLS_CHOICE" ] && SKILLS_CHOICE="none"
  parse_skills_selection "$SKILLS_CHOICE"
else
  LINK_AGENTS=0
  LINK_OPENCODE=0
  LINK_CLAUDE=0
  LINK_GEMINI=0
fi

apply_skills_symlinks

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
