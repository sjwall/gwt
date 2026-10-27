#!/usr/bin/env zsh
# Tests for `gwt cd` (commands/cd.sh).
#
# Exit code contract:
#   1 - invalid argument count (expected exactly 1)
#   2 - multiple matching worktrees in the current repository
#   3 - multiple matching worktrees across tracked repositories
#   4 - no matching worktree

test_cd_no_arguments_returns_1() {
  local repo=$(make_repo demo)
  cd "$repo"
  local before=$PWD

  invoke_gwt cd

  assert_status 1
  assert_contains "$run_stderr" "unknown command"
  assert_stdout_empty
  assert_pwd "$before"
}

test_cd_too_many_arguments_returns_1() {
  local repo=$(make_repo demo)
  add_worktree "$repo" feature >/dev/null
  cd "$repo"

  invoke_gwt cd feature extra

  assert_status 1
  assert_contains "$run_stderr" "unknown command"
  assert_pwd "$repo"
}

test_cd_exact_match_changes_directory_and_prints_path() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$repo"

  invoke_gwt cd feature

  assert_status 0
  assert_eq "$wt" "$run_stdout"
  assert_pwd "$wt"
}

test_cd_exact_match_is_case_insensitive() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" Feature)
  cd "$repo"

  invoke_gwt cd feature

  assert_status 0
  assert_eq "$wt" "$run_stdout"
  assert_pwd "$wt"
}

test_cd_matches_a_unique_substring() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature-login)
  cd "$repo"

  invoke_gwt cd login

  assert_status 0
  assert_eq "$wt" "$run_stdout"
  assert_pwd "$wt"
}

test_cd_prefers_exact_match_over_substring() {
  local repo=$(make_repo demo)
  local exact=$(add_worktree "$repo" alpha)
  add_worktree "$repo" alpha-extra >/dev/null
  cd "$repo"

  invoke_gwt cd alpha

  assert_status 0
  assert_pwd "$exact"
}

test_cd_multiple_matches_in_current_repo_returns_2() {
  local repo=$(make_repo demo)
  local one=$(add_worktree "$repo" feature-login)
  local two=$(add_worktree "$repo" feature-logout)
  cd "$repo"
  local before=$PWD

  invoke_gwt cd feature

  assert_status 2
  assert_contains "$run_stderr" "multiple worktrees match 'feature'"
  assert_contains "$run_stderr" "$one"
  assert_contains "$run_stderr" "$two"
  assert_stdout_empty
  assert_pwd "$before"
}

test_cd_no_match_returns_4() {
  local repo=$(make_repo demo)
  add_worktree "$repo" feature >/dev/null
  cd "$repo"

  invoke_gwt cd nope

  assert_status 4
  assert_contains "$run_stderr" "no matching worktree found for 'nope'"
  assert_stdout_empty
  assert_pwd "$repo"
}

test_cd_finds_worktree_in_tracked_repo() {
  local main=$(make_repo demo)
  local other=$(make_repo other)
  local wt=$(add_worktree "$other" remote-feature)
  track_repo "$other"
  cd "$main"

  invoke_gwt cd remote-feature

  assert_status 0
  assert_eq "$wt" "$run_stdout"
  assert_pwd "$wt"
}

test_cd_multiple_matches_across_tracked_repos_returns_3() {
  local main=$(make_repo demo)
  local repo_a=$(make_repo repo-a)
  local repo_b=$(make_repo repo-b)
  local wt_a=$(add_worktree "$repo_a" feature-one)
  local wt_b=$(add_worktree "$repo_b" feature-two)
  track_repo "$repo_a"
  track_repo "$repo_b"
  cd "$main"
  local before=$PWD

  invoke_gwt cd feature

  assert_status 3
  assert_contains "$run_stderr" "multiple worktrees match 'feature'"
  assert_contains "$run_stderr" "$wt_a"
  assert_contains "$run_stderr" "$wt_b"
  assert_stdout_empty
  assert_pwd "$before"
}

test_cd_ignores_untracked_repos() {
  local main=$(make_repo demo)
  local other=$(make_repo other)
  add_worktree "$other" hidden-feature >/dev/null
  cd "$main"

  invoke_gwt cd hidden-feature

  assert_status 4
  assert_stdout_empty
  assert_pwd "$main"
}

test_cd_ignores_missing_tracked_repo_paths() {
  local repo=$(make_repo demo)
  track_repo "$SANDBOX/repos/does-not-exist"
  cd "$repo"

  invoke_gwt cd anything

  assert_status 4
  assert_contains "$run_stderr" "no matching worktree"
}

test_cd_from_inside_a_worktree_finds_sibling() {
  local repo=$(make_repo demo)
  local first=$(add_worktree "$repo" first)
  local second=$(add_worktree "$repo" second)
  cd "$first"

  invoke_gwt cd second

  assert_status 0
  assert_eq "$second" "$run_stdout"
  assert_pwd "$second"
}
