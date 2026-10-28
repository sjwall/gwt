#!/usr/bin/env zsh
# Tests for `gwt pull` / `gwt p` (commands/pull.sh).
#
# Exit code contract:
#   16 - missing required argument for `--ide` option
#   17 - invalid argument count (expected exactly 1 branch name)
#   20 - `git fetch origin` command failed
#   21 - `git worktree add` command failed

test_pull_fetches_remote_branch_and_creates_worktree() {
  local repo=$(make_origin_repo demo)
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  cd "$repo"

  invoke_gwt pull --ide none feature

  assert_status 0
  assert_pwd "$wt"
  assert_eq "feature" "$(git -C "$wt" branch --show-current)"
  assert_file_contains "$wt/branch.txt" "feature"
}

test_pull_shorthand_p_works() {
  local repo=$(make_origin_repo demo)
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  cd "$repo"

  invoke_gwt p --ide none feature

  assert_status 0
  assert_pwd "$wt"
}

test_pull_reuses_existing_local_branch() {
  # push_branch leaves a local feature branch behind; then advance origin
  # ahead of it. The worktree must reflect the local branch, not origin.
  local repo=$(make_origin_repo demo)
  push_branch "$repo" feature
  echo second > "$repo/second.txt"
  git -C "$repo" add second.txt
  git -C "$repo" commit -qm second
  git -C "$repo" push -q origin main:feature
  local wt="${repo:h}/gwt-demo/feature"
  cd "$repo"

  invoke_gwt pull --ide none feature

  assert_status 0
  assert_pwd "$wt"
  assert_eq "feature" "$(git -C "$wt" branch --show-current)"
  assert_file_contains "$wt/branch.txt" "feature"
  assert_file_absent "$wt/second.txt"
}

test_pull_launches_ide_in_new_worktree() {
  local repo=$(make_origin_repo demo)
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt pull feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_pull_ide_none_skips_launch() {
  local repo=$(make_origin_repo demo)
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt pull --ide none feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_absent "$log"
}

test_pull_ide_none_accepted_after_name() {
  local repo=$(make_origin_repo demo)
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt pull feature --ide none

  assert_status 0
  assert_pwd "$wt"
  assert_file_absent "$log"
}

test_pull_ide_override_launches_named_ide() {
  local repo=$(make_origin_repo demo)
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_cmd other-ide)
  cd "$repo"

  invoke_gwt pull --ide other-ide feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
  assert_file_absent "$SANDBOX/bin/fake-ide.log"
}

test_pull_ide_equals_form_launches_named_ide() {
  local repo=$(make_origin_repo demo)
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_cmd other-ide)
  cd "$repo"

  invoke_gwt pull --ide=other-ide feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_pull_runs_yarn_when_lockfile_present() {
  local repo=$(make_origin_repo demo)
  touch "$repo/yarn.lock"
  git -C "$repo" add yarn.lock
  git -C "$repo" commit -qm "add yarn.lock"
  git -C "$repo" push -q origin main
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_cmd yarn)
  cd "$repo"

  invoke_gwt pull --ide none feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_pull_no_install_skips_yarn() {
  local repo=$(make_origin_repo demo)
  touch "$repo/yarn.lock"
  git -C "$repo" add yarn.lock
  git -C "$repo" commit -qm "add yarn.lock"
  git -C "$repo" push -q origin main
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_cmd yarn)
  cd "$repo"

  invoke_gwt pull --ide none --no-install feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_absent "$log"
}

test_pull_missing_ide_argument_returns_16() {
  local repo=$(make_origin_repo demo)
  cd "$repo"

  invoke_gwt pull --ide

  assert_status 16
  assert_contains "$run_stderr" "--ide requires an argument"
}

test_pull_no_arguments_returns_17() {
  local repo=$(make_origin_repo demo)
  cd "$repo"

  invoke_gwt pull

  assert_status 17
  assert_contains "$run_stderr" "unknown command: 'pull '"
  assert_stdout_empty
}

test_pull_too_many_arguments_returns_17() {
  local repo=$(make_origin_repo demo)
  cd "$repo"

  invoke_gwt pull feature extra

  assert_status 17
  assert_contains "$run_stderr" "unknown command: 'pull feature extra'"
  assert_stdout_empty
}

test_pull_without_origin_remote_returns_20() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt pull --ide none feature

  assert_status 20
  assert_pwd "$repo"
}

test_pull_nonexistent_remote_branch_returns_20() {
  local repo=$(make_origin_repo demo)
  cd "$repo"

  invoke_gwt pull --ide none nope

  assert_status 20
  assert_pwd "$repo"
}

test_pull_existing_worktree_returns_21() {
  local repo=$(make_origin_repo demo)
  push_branch "$repo" feature
  local wt="${repo:h}/gwt-demo/feature"
  cd "$repo"

  invoke_gwt pull --ide none feature
  assert_status 0
  cd "$repo"

  invoke_gwt pull --ide none feature

  assert_status 21
  assert_pwd "$repo"
}
