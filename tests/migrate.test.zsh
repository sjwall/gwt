#!/usr/bin/env zsh
# Tests for `gwt migrate` (commands/migrate.sh).
#
# Exit code contract:
#   52 - not inside a git repository
#   53 - unknown argument or option
#   54 - failed to determine target worktree directory location
#   55 - `git worktree move` command failed

test_migrate_outside_a_git_repository_returns_52() {
  # $PWD starts in $SANDBOX/work, which is not a git repository.
  invoke_gwt migrate

  assert_status 52
  assert_contains "$run_stderr" "not inside a git repository"
  assert_stdout_empty
}

test_migrate_unknown_argument_returns_53() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt migrate extra-arg

  assert_status 53
  assert_contains "$run_stderr" "unknown command: 'migrate extra-arg'"
}

test_migrate_unknown_option_returns_53() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt migrate --bogus

  assert_status 53
  assert_contains "$run_stderr" "unknown command: 'migrate --bogus'"
}

test_migrate_moves_misplaced_worktree_to_expected_location() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)   # $SANDBOX/worktrees/feature
  local expected="$SANDBOX/repos/gwt-demo/feature"
  cd "$repo"

  invoke_gwt migrate

  assert_status 0
  assert_contains "$run_output" "Moving '$wt' to '$expected'"
  assert_file_absent "$wt"
  [[ -d "$expected" ]] || _assert_fail "expected moved worktree at: $expected"

  # The moved worktree is still functional and registered under the new path.
  _run_ok git -C "$expected" status --short
  local list=$(git -C "$repo" worktree list --porcelain)
  assert_contains "$list" "$expected"
  assert_not_contains "$list" "$wt"
}

test_migrate_leaves_matching_worktrees_alone() {
  local repo=$(make_repo demo)
  mkdir -p "$SANDBOX/repos/gwt-demo"
  local wt="$SANDBOX/repos/gwt-demo/feature"
  _run_ok git -C "$repo" worktree add -q "$wt" -b feature
  cd "$repo"

  invoke_gwt migrate

  assert_status 0
  assert_contains "$run_stdout" "all worktrees match the expected path pattern"
  [[ -d "$wt" ]] || _assert_fail "worktree should not have been moved: $wt"
  local list=$(git -C "$repo" worktree list --porcelain)
  assert_contains "$list" "$wt"
}

test_migrate_dry_run_prints_the_move_without_doing_it() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local expected="$SANDBOX/repos/gwt-demo/feature"
  cd "$repo"

  invoke_gwt migrate --dry-run

  assert_status 0
  assert_contains "$run_stdout" "Would move '$wt' to '$expected'"
  assert_not_contains "$run_output" "Moving '$wt'"
  # Worktree stays at the old path and is untouched.
  [[ -d "$wt" ]] || _assert_fail "dry-run moved the worktree: $wt"
  assert_file_absent "$expected"
  local list=$(git -C "$repo" worktree list --porcelain)
  assert_contains "$list" "$wt"
}

test_migrate_short_n_flag_prints_the_move_without_doing_it() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local expected="$SANDBOX/repos/gwt-demo/feature"
  cd "$repo"

  invoke_gwt migrate -n

  assert_status 0
  assert_contains "$run_stdout" "Would move '$wt' to '$expected'"
  [[ -d "$wt" ]] || _assert_fail "dry-run moved the worktree: $wt"
  assert_file_absent "$expected"
}

test_migrate_moves_multiple_misplaced_worktrees() {
  local repo=$(make_repo demo)
  local one=$(add_worktree "$repo" feature-one)
  local two=$(add_worktree "$repo" feature-two)
  local exp_one="$SANDBOX/repos/gwt-demo/feature-one"
  local exp_two="$SANDBOX/repos/gwt-demo/feature-two"
  cd "$repo"

  invoke_gwt migrate

  assert_status 0
  assert_contains "$run_output" "Moving '$one' to '$exp_one'"
  assert_contains "$run_output" "Moving '$two' to '$exp_two'"
  assert_file_absent "$one"
  assert_file_absent "$two"
  [[ -d "$exp_one" && -d "$exp_two" ]] ||
    _assert_fail "expected moved worktrees at: $exp_one and $exp_two"

  local list=$(git -C "$repo" worktree list --porcelain)
  assert_contains "$list" "$exp_one"
  assert_contains "$list" "$exp_two"
  assert_not_contains "$list" "$one"
  assert_not_contains "$list" "$two"
  _run_ok git -C "$exp_one" status --short
  _run_ok git -C "$exp_two" status --short
}

test_migrate_dry_run_when_nothing_needs_moving() {
  local repo=$(make_repo demo)
  mkdir -p "$SANDBOX/repos/gwt-demo"
  local wt="$SANDBOX/repos/gwt-demo/feature"
  _run_ok git -C "$repo" worktree add -q "$wt" -b feature
  cd "$repo"

  invoke_gwt migrate --dry-run

  assert_status 0
  assert_contains "$run_stdout" "all worktrees match the expected path pattern"
  assert_not_contains "$run_stdout" "Would move"
  [[ -d "$wt" ]] || _assert_fail "worktree should still exist: $wt"
}

test_migrate_from_inside_misplaced_worktree_cds_to_new_location() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  local expected="$SANDBOX/repos/gwt-demo/feature"
  cd "$wt"

  invoke_gwt migrate

  assert_status 0
  assert_pwd "$expected"
  _run_ok git -C "$expected" status --short
}
