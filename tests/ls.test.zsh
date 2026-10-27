#!/usr/bin/env zsh
# Tests for `gwt ls` / `gwt list` (commands/ls.sh).
#
# Exit code contract:
#   43 - invalid argument count (more than 1 repository specified)
#   44 - multiple exact repository matches
#   45 - multiple repository name matches
#   46 - multiple repository path matches
#   47 - no matching repository found for query

test_ls_lists_worktrees_for_all_tracked_repos() {
  local one=$(make_repo one)
  local two=$(make_repo two)
  local wt_one=$(add_worktree "$one" feature-one)
  local wt_two=$(add_worktree "$two" feature-two)
  track_repo "$one"
  track_repo "$two"
  cd "$one"

  invoke_gwt ls

  assert_status 0
  assert_contains "$run_output" "$wt_one"
  assert_contains "$run_output" "$wt_two"
  assert_contains "$run_output" "----------"
}

test_ls_includes_current_repo_even_when_untracked() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$repo"

  invoke_gwt ls

  assert_status 0
  assert_contains "$run_output" "$wt"
}

test_ls_ignores_missing_tracked_paths() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  track_repo "$repo"
  track_repo "$SANDBOX/repos/does-not-exist"
  cd "$repo"

  invoke_gwt ls

  assert_status 0
  assert_contains "$run_output" "$wt"
  assert_not_contains "$run_output" "does-not-exist"
}

test_ls_shorthand_list_works() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$repo"

  invoke_gwt list

  assert_status 0
  assert_contains "$run_output" "$wt"
}

test_ls_passes_flags_to_git_worktree_list() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  cd "$repo"

  invoke_gwt ls --porcelain

  assert_status 0
  assert_contains "$run_output" "worktree $wt"
}

test_ls_filters_by_exact_repo_name() {
  local one=$(make_repo demo)
  local two=$(make_repo other)
  local wt_one=$(add_worktree "$one" feature-one)
  local wt_two=$(add_worktree "$two" feature-two)
  track_repo "$one"
  track_repo "$two"
  cd "$one"

  invoke_gwt ls demo

  assert_status 0
  assert_contains "$run_output" "$wt_one"
  assert_not_contains "$run_output" "$wt_two"
}

test_ls_filter_is_case_insensitive() {
  local one=$(make_repo demo)
  local two=$(make_repo other)
  local wt_one=$(add_worktree "$one" feature-one)
  local wt_two=$(add_worktree "$two" feature-two)
  track_repo "$one"
  track_repo "$two"
  cd "$one"

  invoke_gwt ls DEMO

  assert_status 0
  assert_contains "$run_output" "$wt_one"
  assert_not_contains "$run_output" "$wt_two"
}

test_ls_filter_matches_unique_name_substring() {
  local one=$(make_repo demo)
  local two=$(make_repo other)
  local wt_one=$(add_worktree "$one" feature-one)
  local wt_two=$(add_worktree "$two" feature-two)
  track_repo "$one"
  track_repo "$two"
  cd "$two"

  invoke_gwt ls dem

  assert_status 0
  assert_contains "$run_output" "$wt_one"
  assert_not_contains "$run_output" "$wt_two"
}

test_ls_filter_prefers_exact_over_substring() {
  local one=$(make_repo demo)
  local two=$(make_repo demo-extra)
  local wt_one=$(add_worktree "$one" feature-one)
  local wt_two=$(add_worktree "$two" feature-two)
  track_repo "$one"
  track_repo "$two"
  cd "$one"

  invoke_gwt ls demo

  assert_status 0
  assert_contains "$run_output" "$wt_one"
  assert_not_contains "$run_output" "$wt_two"
}

test_ls_filter_matches_by_path_substring() {
  local one=$(make_repo demo)
  local two=$(make_repo other)
  local wt_one=$(add_worktree "$one" feature-one)
  local wt_two=$(add_worktree "$two" feature-two)
  track_repo "$one"
  track_repo "$two"
  cd "$two"

  invoke_gwt ls "repos/demo"

  assert_status 0
  assert_contains "$run_output" "$wt_one"
  assert_not_contains "$run_output" "$wt_two"
}

test_ls_too_many_args_returns_43() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt ls demo extra

  assert_status 43
  assert_contains "$run_stderr" "unknown command: 'list demo extra'"
  assert_stdout_empty
}

test_ls_multiple_exact_matches_return_44() {
  local one=$(make_repo a/demo)
  local two=$(make_repo b/demo)
  track_repo "$one"
  track_repo "$two"
  cd "$one"

  invoke_gwt ls demo

  assert_status 44
  assert_contains "$run_stderr" "multiple repositories match 'demo'"
  assert_contains "$run_stderr" "$one"
  assert_contains "$run_stderr" "$two"
  assert_stdout_empty
}

test_ls_multiple_name_matches_return_45() {
  local one=$(make_repo alpha-demo)
  local two=$(make_repo beta-demo)
  track_repo "$one"
  track_repo "$two"
  cd "$one"

  invoke_gwt ls demo

  assert_status 45
  assert_contains "$run_stderr" "multiple repositories match 'demo'"
  assert_contains "$run_stderr" "$one"
  assert_contains "$run_stderr" "$two"
  assert_stdout_empty
}

test_ls_multiple_path_matches_return_46() {
  local one=$(make_repo alpha)
  local two=$(make_repo beta)
  track_repo "$one"
  track_repo "$two"
  cd "$one"

  invoke_gwt ls repos

  assert_status 46
  assert_contains "$run_stderr" "multiple repositories match 'repos'"
  assert_contains "$run_stderr" "$one"
  assert_contains "$run_stderr" "$two"
  assert_stdout_empty
}

test_ls_no_match_returns_47() {
  local repo=$(make_repo demo)
  track_repo "$repo"
  cd "$repo"

  invoke_gwt ls nope

  assert_status 47
  assert_contains "$run_stderr" "no matching repository found for 'nope'"
  assert_stdout_empty
}
