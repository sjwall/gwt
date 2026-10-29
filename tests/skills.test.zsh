#!/usr/bin/env zsh
# Tests for `gwt skills` (commands/skills.sh + skills.sh).
#
# Exit code contract:
#   51 - skills helper script not found
#
# The skills source is $GWT_DIR/.agents/skills/gwt; target dirs resolve under
# the sandboxed HOME / XDG_CONFIG_HOME. The bare `gwt skills` invocation is
# deliberately untested: it reads from /dev/tty when available and would hang
# an interactive test run.

_skills_src() {
  print -r -- "$GWT_DIR/.agents/skills/gwt"
}

test_skills_list_shows_all_targets_not_linked() {
  invoke_gwt skills list

  assert_status 0
  assert_contains "$run_output" "agents"
  assert_contains "$run_output" "opencode"
  assert_contains "$run_output" "claude"
  assert_contains "$run_output" "gemini"
  assert_contains "$run_output" "[not linked]"
}

test_skills_no_args_non_interactive_lists() {
  # Bare `gwt skills` is interactive-by-design (reads /dev/tty when
  # available), so only the explicit `list` action is tested here.
  invoke_gwt skills list

  assert_status 0
  assert_contains "$run_output" "Agent skill symlink status"
}

test_skills_links_named_target() {
  invoke_gwt skills claude

  assert_status 0
  assert_contains "$run_output" "Symlinked"
  assert_symlink "$HOME/.claude/skills/gwt" "$(_skills_src)"
}

test_skills_list_shows_linked_after_linking() {
  invoke_gwt skills claude >/dev/null

  invoke_gwt skills list

  assert_status 0
  assert_contains "$run_output" "claude"
  assert_contains "$run_output" "[linked]"
}

test_skills_comma_separated_targets() {
  invoke_gwt skills claude,opencode

  assert_status 0
  assert_symlink "$HOME/.claude/skills/gwt" "$(_skills_src)"
  assert_symlink "$XDG_CONFIG_HOME/opencode/skills/gwt" "$(_skills_src)"
  assert_file_absent "$HOME/.agents/skills/gwt"
}

test_skills_all_links_every_target() {
  invoke_gwt skills all

  assert_status 0
  assert_symlink "$HOME/.agents/skills/gwt" "$(_skills_src)"
  assert_symlink "$XDG_CONFIG_HOME/opencode/skills/gwt" "$(_skills_src)"
  assert_symlink "$HOME/.claude/skills/gwt" "$(_skills_src)"
  assert_symlink "$HOME/.gemini/antigravity-cli/skills/gwt" "$(_skills_src)"
  assert_symlink "$HOME/.gemini/config/skills/gwt" "$(_skills_src)"
}

test_skills_number_selection_works() {
  invoke_gwt skills 3

  assert_status 0
  assert_symlink "$HOME/.claude/skills/gwt" "$(_skills_src)"
}

test_skills_dry_run_makes_no_changes() {
  invoke_gwt skills -n claude

  assert_status 0
  assert_contains "$run_output" "Would symlink"
  assert_file_absent "$HOME/.claude/skills/gwt"
}

test_skills_none_unlinks_linked_targets() {
  invoke_gwt skills claude >/dev/null
  assert_symlink "$HOME/.claude/skills/gwt" "$(_skills_src)"

  invoke_gwt skills none

  assert_status 0
  assert_contains "$run_output" "Removed symlink"
  assert_file_absent "$HOME/.claude/skills/gwt"
}

test_skills_none_with_nothing_linked_is_a_no_op() {
  invoke_gwt skills none

  assert_status 0
  assert_contains "$run_output" "No agent skills selected or currently linked"
}

test_skills_relink_is_idempotent() {
  invoke_gwt skills claude >/dev/null

  invoke_gwt skills claude

  assert_status 0
  assert_contains "$run_output" "already linked"
  assert_symlink "$HOME/.claude/skills/gwt" "$(_skills_src)"
}

test_skills_unlink_removes_only_gwt_skill_links() {
  invoke_gwt skills claude >/dev/null
  mkdir -p "$HOME/.claude/skills"
  ln -s /some/other/skill "$HOME/.claude/skills/other-skill"

  invoke_gwt skills none

  assert_file_absent "$HOME/.claude/skills/gwt"
  assert_symlink "$HOME/.claude/skills/other-skill" "/some/other/skill"
}

test_skills_help_shows_usage() {
  invoke_gwt skills help

  assert_status 0
  assert_contains "$run_output" "Usage: gwt skills"
}

test_skills_sync_no_links_is_a_no_op() {
  invoke_gwt skills sync

  assert_status 0
  assert_contains "$run_output" "No linked agent skills detected"
}

test_skills_sync_updates_stale_links() {
  # A link pointing at an old install path still counts as linked (path
  # suffix match), and sync rewrites it to the current skills source.
  mkdir -p "$HOME/.claude/skills"
  ln -s /old/install/.agents/skills/gwt "$HOME/.claude/skills/gwt"

  invoke_gwt skills sync

  assert_status 0
  assert_contains "$run_output" "Updating skills symlinks"
  assert_symlink "$HOME/.claude/skills/gwt" "$(_skills_src)"
}

test_skills_sync_ignores_removed_links() {
  # sync only refreshes links it can detect; a removed link is not re-created.
  invoke_gwt skills claude >/dev/null
  rm -f "$HOME/.claude/skills/gwt"

  invoke_gwt skills sync

  assert_status 0
  assert_contains "$run_output" "No linked agent skills detected"
  assert_file_absent "$HOME/.claude/skills/gwt"
}

test_skills_missing_helper_returns_51() {
  # The 51 path requires the wrapper to be loaded from a directory that has
  # no skills.sh (as in a broken install), and GWT_DIR to lack one too.
  # Sourcing a copy of the real wrapper from an isolated dir simulates it.
  mkdir -p "$SANDBOX/isolated"
  cp "$REPO_ROOT/commands/skills.sh" "$SANDBOX/isolated/skills-wrapper.sh"
  source "$SANDBOX/isolated/skills-wrapper.sh"
  GWT_DIR="$SANDBOX/nowhere"

  run_status=0
  _gwt_skills list > "$SANDBOX/skills.out" 2> "$SANDBOX/skills.err" || run_status=$?
  run_stdout=$(<"$SANDBOX/skills.out")
  run_stderr=$(<"$SANDBOX/skills.err")

  assert_status 51
  assert_contains "$run_stderr" "skills helper not found"
  assert_stdout_empty
}
