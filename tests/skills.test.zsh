#!/usr/bin/env zsh
# Tests for `gwt skills` (commands/skills.sh + skills.sh).
#
# Exit code contract:
#   51 - skills helper script not found

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
