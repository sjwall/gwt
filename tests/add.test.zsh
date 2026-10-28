#!/usr/bin/env zsh
# Tests for `gwt add` / bare `gwt <name>` (commands/create.sh).
#
# Exit code contract:
#   23 - missing required argument for `--ide` option
#   24 - invalid argument count (expected exactly 1 branch name)
#   27 - `git worktree add` command failed

test_add_creates_worktree_and_changes_directory() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  cd "$repo"

  invoke_gwt add feature

  assert_status 0
  assert_pwd "$wt"
  assert_eq "feature" "$(git -C "$wt" branch --show-current)"
}

test_add_bare_invocation_works() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  cd "$repo"

  invoke_gwt feature

  assert_status 0
  assert_pwd "$wt"
}

test_add_launches_ide_in_new_worktree() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt add feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_add_ide_none_skips_launch() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_ide)
  cd "$repo"

  invoke_gwt add --ide none feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_absent "$log"
}

test_add_ide_override_launches_named_ide() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_cmd other-ide)
  cd "$repo"

  invoke_gwt add --ide other-ide feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
  assert_file_absent "$SANDBOX/bin/fake-ide.log"
}

test_add_ide_equals_form_launches_named_ide() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_cmd other-ide)
  cd "$repo"

  invoke_gwt add --ide=other-ide feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_add_runs_yarn_when_lockfile_present() {
  local repo=$(make_repo demo)
  touch "$repo/yarn.lock"
  git -C "$repo" add yarn.lock
  git -C "$repo" commit -qm "add yarn.lock"
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_cmd yarn)
  cd "$repo"

  invoke_gwt add --ide none feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_add_no_install_skips_yarn() {
  local repo=$(make_repo demo)
  touch "$repo/yarn.lock"
  git -C "$repo" add yarn.lock
  git -C "$repo" commit -qm "add yarn.lock"
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_cmd yarn)
  cd "$repo"

  invoke_gwt add --ide none --no-install feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_absent "$log"
}

test_add_agent_launches_configured_agent() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_agent)
  cd "$repo"

  invoke_gwt add -a feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_add_agent_equals_form_launches_named_agent() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_cmd other-agent)
  cd "$repo"

  invoke_gwt add --agent=other-agent feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_add_agent_none_skips_launch() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_agent)
  cd "$repo"

  invoke_gwt add -a none feature

  assert_status 0
  assert_pwd "$wt"
  assert_file_absent "$log"
}

test_add_agent_option_after_name_launches_agent() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  local log=$(fake_agent)
  cd "$repo"

  invoke_gwt add feature -a

  assert_status 0
  assert_pwd "$wt"
  assert_file_contains "$log" "$wt"
}

test_add_missing_ide_argument_returns_23() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt add --ide

  assert_status 23
  assert_contains "$run_stderr" "--ide requires an argument"
}

test_add_no_arguments_returns_24() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt add

  assert_status 24
  assert_contains "$run_stderr" "unknown command: ''"
  assert_stdout_empty
}

test_add_too_many_arguments_returns_24() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt add feature extra

  assert_status 24
  assert_contains "$run_stderr" "unknown command: 'feature extra'"
  assert_stdout_empty
}

test_add_existing_branch_creates_worktree() {
  local repo=$(make_repo demo)
  git -C "$repo" branch feature
  local wt="${repo:h}/gwt-demo/feature"
  cd "$repo"

  invoke_gwt add --ide none feature

  assert_status 0
  assert_pwd "$wt"
  assert_eq "feature" "$(git -C "$wt" branch --show-current)"
}

test_add_already_checked_out_branch_returns_27() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt add main

  assert_status 27
  assert_pwd "$repo"
}

test_add_existing_worktree_returns_27() {
  local repo=$(make_repo demo)
  local wt="${repo:h}/gwt-demo/feature"
  cd "$repo"

  invoke_gwt add --ide none feature
  assert_status 0
  cd "$repo"

  invoke_gwt add --ide none feature

  assert_status 27
  assert_pwd "$repo"
}
