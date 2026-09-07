#!/bin/sh
#
# gwt agent skills manager
# https://github.com/sjwall/gwt
#

set -e

# Setup colors if running in a terminal
if [ -t 1 ]; then
  BOLD="\033[1m"
  GREEN="\033[32m"
  BLUE="\033[34m"
  YELLOW="\033[33m"
  RED="\033[31m"
  DIM="\033[2m"
  RESET="\033[0m"
else
  BOLD=""
  GREEN=""
  BLUE=""
  YELLOW=""
  RED=""
  DIM=""
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

format_display_path() {
  case "$1" in
    "$HOME"/*)
      echo "~/${1#"$HOME"/}"
      ;;
    *)
      echo "$1"
      ;;
  esac
}

is_interactive() {
  if [ -e /dev/tty ] && [ -r /dev/tty ] && [ -w /dev/tty ]; then
    return 0
  fi
  if [ -t 0 ]; then
    return 0
  fi
  return 1
}

# Configuration
INSTALL_DIR="${GWT_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/gwt}"
DRY_RUN=0
ACTION=""
CUSTOM_SELECTION=""

while [ $# -gt 0 ]; do
  case "$1" in
    --dir=*|--install-dir=*)
      INSTALL_DIR="${1#*=}"
      shift
      ;;
    --dir|--install-dir)
      if [ $# -gt 1 ]; then
        INSTALL_DIR="$2"
        shift 2
      else
        error "$1 requires an argument"
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
    --skills=*)
      CUSTOM_SELECTION="${1#--skills=}"
      shift
      ;;
    --skills)
      if [ $# -gt 1 ]; then
        CUSTOM_SELECTION="$2"
        shift 2
      else
        error "--skills requires an argument"
        exit 1
      fi
      ;;
    --no-skills)
      CUSTOM_SELECTION="none"
      shift
      ;;
    --sync|sync|update|--update)
      ACTION="sync"
      shift
      ;;
    --list|list|ls)
      ACTION="list"
      shift
      ;;
    --prompt|prompt|interactive)
      ACTION="prompt"
      shift
      ;;
    -h|--help|help)
      ACTION="help"
      shift
      ;;
    *)
      if [ -n "$CUSTOM_SELECTION" ]; then
        CUSTOM_SELECTION="$CUSTOM_SELECTION $1"
      else
        CUSTOM_SELECTION="$1"
      fi
      shift
      ;;
  esac
done

show_help() {
  echo "Usage: gwt skills [OPTIONS] [TARGETS]"
  echo ""
  echo "Manage global agent skill symlinks for gwt."
  echo ""
  echo "Targets:"
  echo "  agents, opencode, claude, gemini, all, none"
  echo "  (or comma-separated combinations, e.g. 'claude,gemini' or numbers 1-6)"
  echo ""
  echo "Options:"
  echo "  list, ls             List current symlink status for all agents"
  echo "  sync                 Update symlinks for currently linked agents"
  echo "  -n, --dry-run        Perform a dry run without making any changes"
  echo "  --dir=<path>         Installation directory (default: ~/.local/share/gwt)"
  echo "  -h, --help           Show this help message"
  echo ""
  echo "If run without arguments in an interactive shell, an interactive prompt is shown."
}

# Resolve skills source directory
find_skills_src() {
  if [ -d "$INSTALL_DIR/.agents/skills" ]; then
    echo "$INSTALL_DIR/.agents/skills"
  elif [ -d "$(dirname "$0")/.agents/skills" ]; then
    echo "$(dirname "$0")/.agents/skills"
  elif [ -d "./.agents/skills" ]; then
    echo "./.agents/skills"
  else
    echo "$INSTALL_DIR/.agents/skills"
  fi
}

SKILLS_SRC="$(find_skills_src)"

get_target_dirs() {
  _target="$1"
  case "$_target" in
    agents)
      echo "${AGENTS_CONFIG_DIR:-$HOME/.agents}/skills"
      ;;
    opencode)
      echo "${OPENCODE_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/opencode}/skills"
      ;;
    claude)
      echo "${CLAUDE_CONFIG_DIR:-$HOME/.claude}/skills"
      ;;
    gemini)
      if [ -n "$GEMINI_SKILLS_DIR" ]; then
        echo "$GEMINI_SKILLS_DIR"
      elif [ -n "$ANTIGRAVITY_SKILLS_DIR" ]; then
        echo "$ANTIGRAVITY_SKILLS_DIR"
      else
        echo "${ANTIGRAVITY_CONFIG_DIR:-$HOME/.gemini/antigravity-cli}/skills"
        echo "${GEMINI_CONFIG_DIR:-$HOME/.gemini/config}/skills"
      fi
      ;;
  esac
}

is_skill_linked() {
  _dest_dir="$1"
  _skill_src="$2"
  _name="$(basename "$_skill_src")"
  _target="$_dest_dir/$_name"

  [ -L "$_target" ] || return 1

  _link="$(readlink "$_target" 2>/dev/null || true)"
  if [ "$_link" = "$_skill_src" ]; then
    return 0
  fi

  if [ -d "$_target" ] && [ -d "$_skill_src" ]; then
    _target_res="$(cd -P "$_target" 2>/dev/null && pwd || true)"
    _src_res="$(cd -P "$_skill_src" 2>/dev/null && pwd || true)"
    if [ -n "$_target_res" ] && [ "$_target_res" = "$_src_res" ]; then
      return 0
    fi
  fi

  case "$_link" in
    */.agents/skills/"$_name"|*/.agents/skills/"$_name"/*)
      return 0
      ;;
  esac

  return 1
}

is_target_linked() {
  _tgt="$1"
  _dirs="$(get_target_dirs "$_tgt")"

  if [ -d "$SKILLS_SRC" ]; then
    for _skill in "$SKILLS_SRC"/*; do
      [ -d "$_skill" ] || continue
      for _d in $_dirs; do
        if is_skill_linked "$_d" "$_skill"; then
          return 0
        fi
      done
    done
  else
    for _d in $_dirs; do
      if is_skill_linked "$_d" "$SKILLS_SRC/gwt"; then
        return 0
      fi
    done
  fi

  return 1
}

DETECTED_AGENTS=0
DETECTED_OPENCODE=0
DETECTED_CLAUDE=0
DETECTED_GEMINI=0
DETECTED_TARGETS=""

detect_linked_targets() {
  DETECTED_AGENTS=0
  DETECTED_OPENCODE=0
  DETECTED_CLAUDE=0
  DETECTED_GEMINI=0
  DETECTED_TARGETS=""

  if is_target_linked agents; then
    DETECTED_AGENTS=1
    DETECTED_TARGETS="${DETECTED_TARGETS:+$DETECTED_TARGETS, }agents"
  fi
  if is_target_linked opencode; then
    DETECTED_OPENCODE=1
    DETECTED_TARGETS="${DETECTED_TARGETS:+$DETECTED_TARGETS, }opencode"
  fi
  if is_target_linked claude; then
    DETECTED_CLAUDE=1
    DETECTED_TARGETS="${DETECTED_TARGETS:+$DETECTED_TARGETS, }claude"
  fi
  if is_target_linked gemini; then
    DETECTED_GEMINI=1
    DETECTED_TARGETS="${DETECTED_TARGETS:+$DETECTED_TARGETS, }gemini"
  fi
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
      5|all|1-4)
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

  _disp_target="$(format_display_path "$_target")"
  _disp_src="$(format_display_path "$_src")"

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

unlink_skill() {
  _src="$1"
  _dest_dir="$2"
  _name="$(basename "$_src")"
  _target="$_dest_dir/$_name"
  _disp_target="$(format_display_path "$_target")"

  if is_skill_linked "$_dest_dir" "$_src"; then
    if [ "$DRY_RUN" -eq 1 ]; then
      info "Would remove symlink: $_disp_target"
    else
      rm -f "$_target"
      success "Removed symlink: $_disp_target"
    fi
  fi
}

link_dir_skills() {
  _dest_dir="$1"
  if [ -d "$SKILLS_SRC" ]; then
    for _skill in "$SKILLS_SRC"/*; do
      [ -d "$_skill" ] || continue
      symlink_skill "$_skill" "$_dest_dir"
    done
  elif [ "$DRY_RUN" -eq 1 ]; then
    symlink_skill "$SKILLS_SRC/gwt" "$_dest_dir"
  fi
}

unlink_dir_skills() {
  _dest_dir="$1"
  if [ -d "$SKILLS_SRC" ]; then
    for _skill in "$SKILLS_SRC"/*; do
      [ -d "$_skill" ] || continue
      unlink_skill "$_skill" "$_dest_dir"
    done
  elif [ -d "$_dest_dir" ]; then
    unlink_skill "$SKILLS_SRC/gwt" "$_dest_dir"
  fi
}

apply_target_selection() {
  _target="$1"
  _should_link="$2"
  _dirs="$(get_target_dirs "$_target")"
  for _dir in $_dirs; do
    [ -n "$_dir" ] || continue
    if [ "$_should_link" -eq 1 ]; then
      link_dir_skills "$_dir"
    else
      unlink_dir_skills "$_dir"
    fi
  done
}

apply_all_targets() {
  if [ ! -d "$SKILLS_SRC" ] && [ "$DRY_RUN" -eq 0 ]; then
    warn "Skills directory not found at $SKILLS_SRC; skipping skill symlinking."
    return 0
  fi

  detect_linked_targets

  _has_selected=0
  if [ "$LINK_AGENTS" -eq 1 ] || [ "$LINK_OPENCODE" -eq 1 ] || [ "$LINK_CLAUDE" -eq 1 ] || [ "$LINK_GEMINI" -eq 1 ]; then
    _has_selected=1
  fi

  if [ "$_has_selected" -eq 0 ] && [ -z "$DETECTED_TARGETS" ]; then
    info "No agent skills selected or currently linked."
    return 0
  fi

  if [ "$_has_selected" -eq 0 ]; then
    info "Unlinking agent skills..."
  else
    echo ""
    info "Symlinking skills..."
  fi

  apply_target_selection agents "$LINK_AGENTS"
  apply_target_selection opencode "$LINK_OPENCODE"
  apply_target_selection claude "$LINK_CLAUDE"
  apply_target_selection gemini "$LINK_GEMINI"
}

cmd_sync() {
  detect_linked_targets
  if [ -z "$DETECTED_TARGETS" ]; then
    info "No linked agent skills detected."
    return 0
  fi

  if [ ! -d "$SKILLS_SRC" ] && [ "$DRY_RUN" -eq 0 ]; then
    warn "Skills directory not found at $SKILLS_SRC; skipping skill symlinking."
    return 0
  fi

  info "Detected linked agent skills: $DETECTED_TARGETS"
  info "Updating skills symlinks..."

  if [ "$DETECTED_AGENTS" -eq 1 ]; then
    apply_target_selection agents 1
  fi
  if [ "$DETECTED_OPENCODE" -eq 1 ]; then
    apply_target_selection opencode 1
  fi
  if [ "$DETECTED_CLAUDE" -eq 1 ]; then
    apply_target_selection claude 1
  fi
  if [ "$DETECTED_GEMINI" -eq 1 ]; then
    apply_target_selection gemini 1
  fi
}

cmd_list() {
  detect_linked_targets
  echo "Agent skill symlink status:"
  _print_status() {
    _tgt="$1"
    _disp="$2"
    _linked="$3"
    if [ "$_linked" -eq 1 ]; then
      printf "  %-10s ${GREEN}[linked]${RESET}     (%s)\n" "$_tgt" "$_disp"
    else
      printf "  %-10s ${DIM}[not linked]${RESET} (%s)\n" "$_tgt" "$_disp"
    fi
  }

  _print_status "agents" "~/.agents/skills" "$DETECTED_AGENTS"
  _print_status "opencode" "~/.config/opencode/skills" "$DETECTED_OPENCODE"
  _print_status "claude" "~/.claude/skills" "$DETECTED_CLAUDE"
  _print_status "gemini" "~/.gemini/antigravity-cli/skills, ~/.gemini/config/skills" "$DETECTED_GEMINI"
}

cmd_prompt() {
  detect_linked_targets

  _status_tag() {
    if [ "$1" -eq 1 ]; then
      printf " ${GREEN}[linked]${RESET}"
    fi
  }

  echo ""
  info "Symlink skills to global agent directories?"
  echo "Select targets to link skills (comma-separated or numbers):"
  printf "  1) agents       (~/.agents/skills)%s\n" "$(_status_tag "$DETECTED_AGENTS")"
  printf "  2) opencode     (~/.config/opencode/skills)%s\n" "$(_status_tag "$DETECTED_OPENCODE")"
  printf "  3) claude       (~/.claude/skills)%s\n" "$(_status_tag "$DETECTED_CLAUDE")"
  printf "  4) gemini       (~/.gemini/antigravity-cli/skills, ~/.gemini/config/skills)%s\n" "$(_status_tag "$DETECTED_GEMINI")"
  echo "  5) all"
  echo "  6) none"
  echo ""

  if [ -n "$DETECTED_TARGETS" ]; then
    _default_prompt="$DETECTED_TARGETS"
  else
    _default_prompt="none"
  fi

  if [ -e /dev/tty ] && [ -r /dev/tty ] && [ -w /dev/tty ]; then
    printf "Enter choice(s) [default: %s]: " "$_default_prompt" > /dev/tty
    read -r SKILLS_CHOICE < /dev/tty || SKILLS_CHOICE=""
  elif [ -t 0 ]; then
    printf "Enter choice(s) [default: %s]: " "$_default_prompt"
    read -r SKILLS_CHOICE || SKILLS_CHOICE=""
  else
    SKILLS_CHOICE="$_default_prompt"
  fi

  [ -z "$SKILLS_CHOICE" ] && SKILLS_CHOICE="$_default_prompt"
  parse_skills_selection "$SKILLS_CHOICE"
  apply_all_targets
}

# Main execution dispatch
if [ "$ACTION" = "help" ]; then
  show_help
  exit 0
elif [ "$ACTION" = "sync" ]; then
  cmd_sync
elif [ "$ACTION" = "list" ]; then
  cmd_list
elif [ "$ACTION" = "prompt" ]; then
  cmd_prompt
elif [ -n "$CUSTOM_SELECTION" ]; then
  parse_skills_selection "$CUSTOM_SELECTION"
  apply_all_targets
else
  if is_interactive; then
    cmd_prompt
  else
    cmd_list
  fi
fi
