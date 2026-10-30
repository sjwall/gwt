#!/usr/bin/env zsh
# Tests for `gwt upgrade` (commands/upgrade.sh).
#
# `gwt upgrade` runs `git pull` inside the gwt installation directory, so every
# test overrides GWT_DIR to point at a sandbox-local clone of the real gwt
# repository. The harness default (GWT_DIR=$REPO_ROOT) must never be used here,
# and neither may the real installation at ~/.local/share/gwt.
#
# Exit code contract:
#  34 - gwt repository not found at target directory
#  35 - git pull command failed during upgrade

test_upgrade_returns_34_when_gwt_dir_is_not_a_git_repo() {
  # gwt() sources commands/upgrade.sh from $GWT_DIR before running it, so the
  # fake installation needs that file but must not be a git repository.
  local fake_dir="$SANDBOX/gwt-install"
  mkdir -p "$fake_dir/commands"
  cp "$REPO_ROOT/commands/upgrade.sh" "$fake_dir/commands/upgrade.sh"

  GWT_DIR="$fake_dir"
  invoke_gwt upgrade

  assert_status 34
  assert_contains "$run_stderr" "gwt: repository not found at $fake_dir"
  assert_stdout_empty
}

test_upgrade_returns_35_when_git_pull_fails() {
  local clone="$SANDBOX/gwt-clone"
  local origin="$SANDBOX/origin.git"
  local diverge="$SANDBOX/diverge"

  # Clone the real gwt repository so gwt() can source its command scripts.
  _run_ok git clone -q "$REPO_ROOT" "$clone"
  _run_ok git -C "$clone" config user.name "gwt tests"
  _run_ok git -C "$clone" config user.email "gwt-tests@example.com"

  # Build a bare origin whose tip conflicts with a local commit in the clone,
  # so `git pull` fails with a merge conflict. All remotes are local file
  # paths, so git can never prompt for credentials.
  _run_ok git clone -q --bare "$REPO_ROOT" "$origin"
  _run_ok git clone -q "$origin" "$diverge"
  _run_ok git -C "$diverge" config user.name "gwt tests"
  _run_ok git -C "$diverge" config user.email "gwt-tests@example.com"
  echo remote-change >> "$diverge/README.adoc"
  _run_ok git -C "$diverge" commit -qam remote-change
  _run_ok git -C "$diverge" push -q origin HEAD

  echo local-change >> "$clone/README.adoc"
  _run_ok git -C "$clone" commit -qam local-change

  # Repoint the clone's origin at the divergent bare repository. The remote
  # name is unchanged, so the existing branch tracking keeps working.
  _run_ok git -C "$clone" remote set-url origin "$origin"
  _run_ok git -C "$clone" config pull.rebase false

  GWT_DIR="$clone"
  invoke_gwt upgrade

  assert_status 35
  assert_contains "$run_stdout" "Upgrading gwt at $clone"
}

test_upgrade_pulls_from_origin_and_succeeds() {
  local clone="$SANDBOX/gwt-clone"

  # A pristine clone of the real repository: origin is the local REPO_ROOT and
  # branch tracking is configured by git clone, so the pull is up to date.
  _run_ok git clone -q "$REPO_ROOT" "$clone"

  GWT_DIR="$clone"
  invoke_gwt upgrade

  assert_status 0
  assert_contains "$run_stdout" "Upgrading gwt at $clone"
}
