#!/usr/bin/env zsh
# Tests for `gwt remove` / `gwt rm` (commands/remove.sh).
#
# Exit code contract:
#   13 - cannot remove main repository (a worktree target must be specified)
#   15 - `git worktree remove` command failed

test_remove_from_worktree_removes_and_returns_to_main() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt remove

  assert_status 0
  assert_contains "$run_output" "$repo"
  assert_pwd "$repo"
  assert_file_absent "$wt"
  assert_eq "" "$(git -C "$repo" branch --list feature)"
}

test_remove_shorthand_rm_works() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$wt"

  invoke_gwt rm

  assert_status 0
  assert_pwd "$repo"
  assert_file_absent "$wt"
}

test_remove_from_main_without_target_returns_13() {
  local repo=$(make_repo demo)
  add_worktree "$repo" feature >/dev/null
  cd "$repo"

  invoke_gwt remove

  assert_status 13
  assert_contains "$run_stderr" "cannot remove main repository"
  assert_stdout_empty
}

test_remove_by_name_from_main_repo() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$repo"

  invoke_gwt remove feature

  assert_status 0
  assert_file_absent "$wt"
  assert_pwd "$repo"
}

test_remove_by_path_from_main_repo() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$repo"

  invoke_gwt remove "$wt"

  assert_status 0
  assert_file_absent "$wt"
  assert_pwd "$repo"
}

test_remove_dirty_worktree_without_force_returns_15() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  echo change > "$wt/file.txt"
  cd "$wt"

  invoke_gwt remove

  assert_status 15
  assert_pwd "$wt"
  assert_contains "$run_stderr" "contains modified or untracked files"
  # the worktree survives the failed removal
  [[ -d "$wt" ]] || _assert_fail "expected worktree to survive: $wt"
}

test_remove_f_force_removes_dirty_worktree() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  echo change > "$wt/file.txt"
  cd "$wt"

  invoke_gwt remove -f

  assert_status 0
  assert_pwd "$repo"
  assert_file_absent "$wt"
}

test_remove_force_long_flag_removes_dirty_worktree() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  echo change > "$wt/file.txt"
  cd "$wt"

  invoke_gwt remove --force

  assert_status 0
  assert_file_absent "$wt"
}

test_remove_force_variant_flag_removes_dirty_worktree() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  echo change > "$wt/file.txt"
  cd "$wt"

  invoke_gwt remove -force

  assert_status 0
  assert_file_absent "$wt"
}
