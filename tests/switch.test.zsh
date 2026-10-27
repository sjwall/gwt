#!/usr/bin/env zsh
# Tests for `gwt switch` / `gwt s` (commands/switch.sh).
#
# Exit code contract:
#   11 - missing required argument for `--ide` option
#   12 - invalid argument count (expected exactly 1 worktree name)
#   cd exit codes (1-4) are propagated unchanged.

test_switch_switches_and_launches_ide() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt switch feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_switch_shorthand_s_works() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt s feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_switch_matches_a_unique_substring() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature-login)
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt switch login

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_switch_from_worktree_finds_sibling() {
  local repo=$(make_repo demo)
  local first=$(add_worktree "$repo" first)
  local second=$(add_worktree "$repo" second)
  local log=$(fake_ide)
  cd "$first"

  invoke_gwt switch second

  assert_status 0
  assert_pwd "$second"
  assert_file_contains "$log" "$second"
}

test_switch_ide_none_skips_launch() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt switch --ide none feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_absent "$log"
}

test_switch_ide_none_accepted_after_name() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt switch feature --ide none

  assert_status 0
  assert_pwd "$wt"
  assert_file_absent "$log"
}

test_switch_ide_override_launches_named_ide() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local log=$(fake_cmd other-ide)
  cd "$repo"

  invoke_gwt switch --ide other-ide feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
  assert_file_absent "$SANDBOX/bin/fake-ide.log"
}

test_switch_ide_equals_form_launches_named_ide() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local log=$(fake_cmd other-ide)
  cd "$repo"

  invoke_gwt switch --ide=other-ide feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_switch_missing_ide_argument_returns_11() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt switch --ide

  assert_status 11
  assert_contains "$run_stderr" "--ide requires an argument"
}

test_switch_no_arguments_returns_12() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt switch

  assert_status 12
  assert_contains "$run_stderr" "unknown command: switch ''"
  assert_stdout_empty
}

test_switch_too_many_arguments_returns_12() {
  local repo=$(make_repo demo)
  add_worktree "$repo" feature >/dev/null
  cd "$repo"

  invoke_gwt switch feature extra

  assert_status 12
  assert_contains "$run_stderr" "unknown command: switch 'feature extra'"
  assert_stdout_empty
}

test_switch_no_match_propagates_cd_exit_4() {
  local repo=$(make_repo demo)
  add_worktree "$repo" feature >/dev/null
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt switch nope

  assert_status 4
  assert_contains "$run_stderr" "no matching worktree found for 'nope'"
  assert_file_absent "$log"
}

test_switch_multiple_matches_propagate_cd_exit_2() {
  local repo=$(make_repo demo)
  local one=$(add_worktree "$repo" feature-login)
  local two=$(add_worktree "$repo" feature-logout)
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt switch feature

  assert_status 2
  assert_contains "$run_stderr" "multiple worktrees match 'feature'"
  assert_contains "$run_stderr" "$one"
  assert_contains "$run_stderr" "$two"
  assert_file_absent "$log"
}
