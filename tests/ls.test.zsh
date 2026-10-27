#!/usr/bin/env zsh
# Tests for `gwt ls` / `gwt list` (commands/ls.sh).
#
# Exit code contract regression tests: the documented codes for `list` are
# 43-47, but the implementation returned 39-43 (colliding with `clone`).

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
