#!/usr/bin/env zsh
# Tests for `gwt devcon` (commands/devcon.sh).
#
# `gwt devcon` configures the devcontainer.json of the repository (or
# worktree) the command is run from so the container mounts a gwt docker
# volume named after the devcontainer's `name` field:
#   * picks `.devcontainer.json` at the repo root, falling back to
#     `.devcontainer/devcontainer.json`
#   * sets `.mounts` to a volume entry (or appends one) with source/target
#     derived from the devcontainer's `name` ("gwt-<name>")
#   * creates the matching docker volume when docker is available
#
# Exit code contract:
#  56 - not inside a git repository
#  57 - no devcontainer configuration found
#   1 - docker is present but the volume create fails
#
# All tests stub `docker` on $PATH (the sandbox bin dir comes first), so the
# host docker daemon is never touched.

test_devcon_not_inside_a_git_repository_returns_56() {
  cd "$SANDBOX/work"

  invoke_gwt devcon

  assert_status 56
  assert_contains "$run_stderr" "gwt: not inside a git repository"
  assert_stdout_empty
}

test_devcon_without_devcontainer_config_returns_57() {
  local repo=$(make_repo demo)
  cd "$repo"

  invoke_gwt devcon

  assert_status 57
  assert_contains "$run_stdout" "gwt: no devcontainer configuration. Please setup first."
  assert_eq "" "$run_stderr"
}

test_devcon_uses_root_devcontainer_json() {
  local repo=$(make_repo demo)
  cat > "$repo/.devcontainer.json" <<'EOF'
{ "name": "rootproj" }
EOF
  cd "$repo"
  local docker_log=$(fake_cmd docker)

  invoke_gwt devcon

  assert_status 0
  assert_contains "$run_stdout" "gwt: setting gwt-rootproj mount"
  assert_file_contains "$repo/.devcontainer.json" '"source": "gwt-rootproj"'
  assert_file_contains "$repo/.devcontainer.json" '"target": "/workspaces/gwt-rootproj"'
  assert_file_contains "$docker_log" "volume create gwt-rootproj"
}

test_devcon_uses_nested_devcontainer_devcontainer_json() {
  local repo=$(make_repo demo)
  mkdir -p "$repo/.devcontainer"
  cat > "$repo/.devcontainer/devcontainer.json" <<'EOF'
{ "name": "myproj", "image": "mcr.microsoft.com/devcontainers/base:latest" }
EOF
  cd "$repo"
  local docker_log=$(fake_cmd docker)

  invoke_gwt devcon

  assert_status 0
  assert_contains "$run_stdout" "gwt: setting gwt-myproj mount"
  assert_file_contains "$repo/.devcontainer/devcontainer.json" '"name": "myproj"'
  assert_file_contains "$repo/.devcontainer/devcontainer.json" '"source": "gwt-myproj"'
  assert_file_contains "$repo/.devcontainer/devcontainer.json" '"target": "/workspaces/gwt-myproj"'
  assert_file_contains "$docker_log" "volume create gwt-myproj"
}

test_devcon_prefers_root_devcontainer_json_over_nested() {
  local repo=$(make_repo demo)
  cat > "$repo/.devcontainer.json" <<'EOF'
{ "name": "rootproj" }
EOF
  mkdir -p "$repo/.devcontainer"
  cat > "$repo/.devcontainer/devcontainer.json" <<'EOF'
{ "name": "nested" }
EOF
  cd "$repo"
  fake_cmd docker

  invoke_gwt devcon

  assert_status 0
  assert_contains "$run_stdout" "gwt: setting gwt-rootproj mount"
  assert_file_contains "$repo/.devcontainer.json" '"source": "gwt-rootproj"'
  # the nested configuration must be left untouched
  assert_not_contains "$(<"$repo/.devcontainer/devcontainer.json")" "gwt-"
}

test_devcon_appends_gwt_mount_to_existing_mounts() {
  local repo=$(make_repo demo)
  mkdir -p "$repo/.devcontainer"
  cat > "$repo/.devcontainer/devcontainer.json" <<'EOF'
{ "name": "other", "mounts": [{ "type": "volume", "source": "other-vol", "target": "/other" }] }
EOF
  cd "$repo"
  local docker_log=$(fake_cmd docker)

  invoke_gwt devcon

  assert_status 0
  assert_contains "$run_stdout" "gwt: appending gwt-other mount"
  local config=$(<"$repo/.devcontainer/devcontainer.json")
  assert_contains "$config" '"source": "other-vol"'
  assert_contains "$config" '"source": "gwt-other"'
  assert_file_contains "$docker_log" "volume create gwt-other"
}

test_devcon_second_run_reports_mount_already_set() {
  local repo=$(make_repo demo)
  mkdir -p "$repo/.devcontainer"
  cat > "$repo/.devcontainer/devcontainer.json" <<'EOF'
{ "name": "myproj" }
EOF
  cd "$repo"
  local docker_log=$(fake_cmd docker)

  invoke_gwt devcon
  assert_status 0

  # only the first run should ask docker to create the volume
  : > "$docker_log"

  invoke_gwt devcon

  assert_status 0
  assert_contains "$run_stdout" "gwt: gwt-myproj mount already set."
  assert_contains "$run_stdout" "gwt: docker volume gwt-myproj already exists."
  assert_not_contains "$run_stdout" "setting gwt-myproj mount"
  assert_not_contains "$run_stdout" "appending gwt-myproj mount"
  # the configuration must not gain a duplicate mount entry
  local config=$(<"$repo/.devcontainer/devcontainer.json")
  assert_eq 1 $(grep -c '"source": "gwt-myproj"' "$repo/.devcontainer/devcontainer.json")
  # no new volume create request on the second run
  assert_contains "$(<"$docker_log")" "volume ls"
  assert_not_contains "$(<"$docker_log")" "volume create"
}

test_devcon_warns_when_docker_is_not_installed() {
  local repo=$(make_repo demo)
  mkdir -p "$repo/.devcontainer"
  cat > "$repo/.devcontainer/devcontainer.json" <<'EOF'
{ "name": "myproj" }
EOF
  cd "$repo"
  # CI runners and hosts may have a real docker on PATH (e.g. /usr/bin/docker
  # on ubuntu, where /bin is a symlink to /usr/bin), so PATH trimming alone
  # cannot hide it. Shim `command` instead so the docker lookup fails.
  command() {
    if [[ "$1" == -v && "$2" == docker ]]; then
      return 1
    fi
    builtin command "$@"
  }
  invoke_gwt devcon

  assert_status 0
  assert_contains "$run_stdout" "gwt: docker is not installed, unable to create volume."
  # the devcontainer configuration is still updated
  assert_file_contains "$repo/.devcontainer/devcontainer.json" '"source": "gwt-myproj"'
}

test_devcon_docker_failure_is_reported_as_nonzero_status() {
  local repo=$(make_repo demo)
  mkdir -p "$repo/.devcontainer"
  cat > "$repo/.devcontainer/devcontainer.json" <<'EOF'
{ "name": "myproj" }
EOF
  cd "$repo"
  # a docker stub whose `volume create` fails
  cat > "$SANDBOX/bin/docker" <<EOF
#!/bin/sh
exit 1
EOF
  chmod +x "$SANDBOX/bin/docker"

  invoke_gwt devcon

  assert_status 1
  assert_contains "$run_stdout" "gwt: setting gwt-myproj mount"
}

test_devcon_configures_the_worktree_copy_when_run_inside_a_worktree() {
  local repo=$(make_repo demo)
  local wt=$(add_worktree "$repo" feature)
  mkdir -p "$wt/.devcontainer"
  cat > "$wt/.devcontainer/devcontainer.json" <<'EOF'
{ "name": "wtproj" }
EOF
  cd "$wt"
  fake_cmd docker

  invoke_gwt devcon

  assert_status 0
  assert_contains "$run_stdout" "gwt: setting gwt-wtproj mount"
  assert_file_contains "$wt/.devcontainer/devcontainer.json" '"source": "gwt-wtproj"'
  # the main repository's configuration is untouched
  assert_file_absent "$repo/.devcontainer/devcontainer.json"
}
