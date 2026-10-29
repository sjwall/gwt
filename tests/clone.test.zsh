#!/usr/bin/env zsh
# Tests for `gwt clone` (commands/clone.sh).
#
# Exit code contract:
#   39 - invalid argument count (no arguments provided)
#   40 - git clone command failed
#   23 - missing required argument for `--ide` option
#   44 - missing required argument for `--agent` option (undocumented in README)

test_clone_clones_and_tracks_repository() {
  local origin="$SANDBOX/origins/demo.git"
  make_origin_repo demo >/dev/null
  local dest="$SANDBOX/work/demo"

  invoke_gwt clone --ide none "$origin" demo

  assert_status 0
  assert_pwd "$dest"
  assert_file_contains "$(_repos_file)" "$dest"
}

test_clone_without_directory_argument_uses_repo_name() {
  local origin="$SANDBOX/origins/demo.git"
  make_origin_repo demo >/dev/null
  local dest="$SANDBOX/work/demo"

  invoke_gwt clone --ide none "$origin"

  assert_status 0
  assert_pwd "$dest"
  assert_file_contains "$(_repos_file)" "$dest"
}

test_clone_no_arguments_returns_39() {
  invoke_gwt clone

  assert_status 39
  assert_contains "$run_stderr" "Usage: gwt clone"
  assert_stdout_empty
}

test_clone_only_options_returns_39() {
  invoke_gwt clone --ide none

  assert_status 39
  assert_contains "$run_stderr" "Usage: gwt clone"
  assert_stdout_empty
}

test_clone_failure_returns_40() {
  invoke_gwt clone --ide none "$SANDBOX/does-not-exist.git"

  assert_status 40
  assert_contains "$run_stderr" "does-not-exist"
}

test_clone_missing_ide_argument_returns_23() {
  invoke_gwt clone --ide

  assert_status 23
  assert_contains "$run_stderr" "--ide requires an argument"
}

test_clone_missing_agent_argument_returns_44() {
  invoke_gwt clone --agent

  assert_status 44
  assert_contains "$run_stderr" "--agent requires an argument"
}

test_clone_ide_override_launches_named_ide() {
  local origin="$SANDBOX/origins/demo.git"
  make_origin_repo demo >/dev/null
  local dest="$SANDBOX/work/demo"
  local log=$(fake_cmd fake-ide)

  invoke_gwt clone --ide fake-ide "$origin" demo

  assert_status 0
  assert_pwd "$dest"
  assert_file_contains "$log" "$dest"
}

test_clone_agent_launches_named_agent() {
  local origin="$SANDBOX/origins/demo.git"
  make_origin_repo demo >/dev/null
  local dest="$SANDBOX/work/demo"
  local log=$(fake_cmd fake-agent)

  invoke_gwt clone --agent fake-agent "$origin" demo

  assert_status 0
  assert_pwd "$dest"
  assert_file_contains "$log" "$dest"
}

test_clone_agent_equals_form_launches_named_agent() {
  local origin="$SANDBOX/origins/demo.git"
  make_origin_repo demo >/dev/null
  local dest="$SANDBOX/work/demo"
  local log=$(fake_cmd fake-agent)

  invoke_gwt clone --agent=fake-agent "$origin" demo

  assert_status 0
  assert_pwd "$dest"
  assert_file_contains "$log" "$dest"
}

test_clone_runs_yarn_when_lockfile_present() {
  local repo=$(make_origin_repo demo)
  touch "$repo/yarn.lock"
  git -C "$repo" add yarn.lock
  git -C "$repo" commit -qm "add yarn.lock"
  git -C "$repo" push -q origin main
  local origin="$SANDBOX/origins/demo.git"
  local dest="$SANDBOX/work/demo"
  local log=$(fake_cmd yarn)

  invoke_gwt clone --ide none "$origin" demo

  assert_status 0
  assert_pwd "$dest"
  assert_file_contains "$log" "$dest"
}

test_clone_no_install_skips_yarn() {
  local repo=$(make_origin_repo demo)
  touch "$repo/yarn.lock"
  git -C "$repo" add yarn.lock
  git -C "$repo" commit -qm "add yarn.lock"
  git -C "$repo" push -q origin main
  local origin="$SANDBOX/origins/demo.git"
  local dest="$SANDBOX/work/demo"
  local log=$(fake_cmd yarn)

  invoke_gwt clone --ide none --no-install "$origin" demo

  assert_status 0
  assert_pwd "$dest"
  assert_file_absent "$log"
}
