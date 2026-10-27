#!/usr/bin/env zsh
# Sandbox, assertion, and git fixture helpers for gwt tests.
#
# A test body runs inside its own subshell (see run-file.zsh) and starts with
# setup_sandbox already called. That means $PWD, $HOME, $XDG_* and $gwt_dir are
# isolated and `gwt cd` can be asserted by inspecting $PWD directly.

# ---------------------------------------------------------------------------
# Sandbox
# ---------------------------------------------------------------------------

setup_sandbox() {
  SANDBOX=$(mktemp -d "${TMPDIR:-/tmp}/gwt-test.XXXXXX") || return 1
  SANDBOX=$(cd "$SANDBOX" && pwd -P)

  export HOME="$SANDBOX/home"
  export XDG_CONFIG_HOME="$SANDBOX/config"
  export XDG_DATA_HOME="$SANDBOX/data"
  export GWT_DIR="$REPO_ROOT"
  export GWT_IDE=none
  export GWT_AGENT=none
  export GIT_CONFIG_NOSYSTEM=1

  mkdir -p "$HOME" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" \
           "$SANDBOX/bin" "$SANDBOX/repos" "$SANDBOX/worktrees" "$SANDBOX/work"
  export PATH="$SANDBOX/bin:$PATH"

  cd "$SANDBOX/work" || return 1

  source "$REPO_ROOT/gwt.sh"
}

# Run `gwt` in the current shell (so `cd` persists) while capturing its output.
# Non-zero exit codes are expected in these tests, so they are recorded rather
# than propagating.
invoke_gwt() {
  local out="$SANDBOX/last.stdout" err="$SANDBOX/last.stderr"
  : >"$out"
  : >"$err"

  run_status=0
  gwt "$@" >"$out" 2>"$err" || run_status=$?

  run_stdout=$(<"$out")
  run_stderr=$(<"$err")
  if [[ -n "$run_stdout" && -n "$run_stderr" ]]; then
    run_output="${run_stdout}"$'\n'"${run_stderr}"
  elif [[ -n "$run_stdout" ]]; then
    run_output="$run_stdout"
  else
    run_output="$run_stderr"
  fi
  return 0
}

# ---------------------------------------------------------------------------
# Assertions
# ---------------------------------------------------------------------------

_assert_fail() {
  local line
  for line in "$@"; do
    print -ru2 -- "$line"
  done
  exit 1
}

assert_status() {
  local expected=$1
  if (( run_status != expected )); then
    local -a details
    details=(
      "expected exit status: $expected"
      "actual exit status:   $run_status"
    )
    [[ -n "$run_stdout" ]] && details+=("stdout:" "$run_stdout")
    [[ -n "$run_stderr" ]] && details+=("stderr:" "$run_stderr")
    _assert_fail "${details[@]}"
  fi
}

assert_eq() {
  local expected=$1 actual=$2
  [[ "$expected" == "$actual" ]] ||
    _assert_fail "expected: $expected" "actual:   $actual"
}

assert_contains() {
  local haystack=$1 needle=$2
  [[ "$haystack" == *"$needle"* ]] ||
    _assert_fail "expected to contain: $needle" "actual: $haystack"
}

assert_not_contains() {
  local haystack=$1 needle=$2
  [[ "$haystack" != *"$needle"* ]] ||
    _assert_fail "expected not to contain: $needle" "actual: $haystack"
}

assert_stdout_empty() {
  [[ -z "$run_stdout" ]] ||
    _assert_fail "expected empty stdout" "actual: $run_stdout"
}

assert_file_contains() {
  local file=$1 needle=$2 content
  [[ -f "$file" ]] || _assert_fail "file not found: $file"
  content=$(<"$file")
  [[ "$content" == *"$needle"* ]] ||
    _assert_fail "expected $file to contain: $needle" "actual: $content"
}

assert_file_absent() {
  local file=$1
  [[ ! -e "$file" ]] || _assert_fail "expected file to be absent: $file"
}

assert_pwd() {
  local expected=${1:A} actual=${PWD:A}
  [[ "$expected" == "$actual" ]] ||
    _assert_fail "expected PWD: $expected" "actual PWD:   $actual"
}

# ---------------------------------------------------------------------------
# Git fixtures
# ---------------------------------------------------------------------------

_run_ok() {
  "$@" || _assert_fail "fixture command failed: $*"
}

# make_repo <name> -> prints the repository path
make_repo() {
  local name=$1
  local repo="$SANDBOX/repos/$name"

  mkdir -p "$repo"
  _run_ok git -c init.defaultBranch=main init -q "$repo"
  _run_ok git -C "$repo" config user.name "gwt tests"
  _run_ok git -C "$repo" config user.email "gwt-tests@example.com"
  echo init > "$repo/file.txt"
  _run_ok git -C "$repo" add file.txt
  _run_ok git -C "$repo" commit -qm initial

  print -r -- "$repo"
}

# add_worktree <repo> <name> -> prints the worktree path
add_worktree() {
  local repo=$1 name=$2
  local dest="$SANDBOX/worktrees/$name"

  mkdir -p "${dest:h}"
  _run_ok git -C "$repo" worktree add -q "$dest" -b "$name"

  print -r -- "$dest"
}

# track_repo <path> -> records a repo as tracked in the sandbox config
track_repo() {
  mkdir -p "$XDG_CONFIG_HOME/gwt"
  print -r -- "$1" >> "$XDG_CONFIG_HOME/gwt/repos"
}

# ---------------------------------------------------------------------------
# Fake IDE / agent launchers
# ---------------------------------------------------------------------------

# fake_cmd <name> -> prints the log path
# Creates a stub command on $PATH that appends "<cwd> <args...>" to a log
# file, so launch assertions can be made without launching anything real.
fake_cmd() {
  local name=$1
  local log="$SANDBOX/bin/$name.log"

  cat > "$SANDBOX/bin/$name" <<EOF || _assert_fail "failed to write fake command $name"
#!/bin/sh
printf '%s\n' "\$(pwd) \$*" >> "$log"
EOF
  chmod +x "$SANDBOX/bin/$name"

  print -r -- "$log"
}

# fake_ide [name] -> prints the log path; also configures gwt to launch it,
# overriding the sandbox's GWT_IDE=none.
fake_ide() {
  local name=${1:-fake-ide}
  gwt config set ide "$name" >/dev/null
  fake_cmd "$name"
}

# fake_agent [name] -> prints the log path; also configures gwt to launch it,
# overriding the sandbox's GWT_AGENT=none.
fake_agent() {
  local name=${1:-fake-agent}
  gwt config set agent "$name" >/dev/null
  fake_cmd "$name"
}

# ---------------------------------------------------------------------------
# Remote fixtures
# ---------------------------------------------------------------------------

# make_origin_repo <name> -> prints the clone path
# Creates a bare "origin" at $SANDBOX/origins/<name>.git and a working clone
# at $SANDBOX/repos/<name> with an initial commit on main, so commands that
# fetch from origin (pull, clone) have a remote.
make_origin_repo() {
  local name=$1
  local origin="$SANDBOX/origins/$name.git"
  local repo="$SANDBOX/repos/$name"

  mkdir -p "$SANDBOX/origins"
  _run_ok git init -q --bare "$origin"
  _run_ok git -c init.defaultBranch=main init -q "$repo"
  _run_ok git -C "$repo" config user.name "gwt tests"
  _run_ok git -C "$repo" config user.email "gwt-tests@example.com"
  echo init > "$repo/file.txt"
  _run_ok git -C "$repo" add file.txt
  _run_ok git -C "$repo" commit -qm initial
  _run_ok git -C "$repo" remote add origin "$origin"
  _run_ok git -C "$repo" push -q -u origin main

  print -r -- "$repo"
}

# push_branch <repo> <branch> -> commits a file on <branch> and pushes it to
# origin, so `gwt pull <branch>` has something to fetch.
push_branch() {
  local repo=$1 branch=$2
  local prev=$(git -C "$repo" rev-parse --abbrev-ref HEAD)

  _run_ok git -C "$repo" checkout -q -b "$branch"
  echo "$branch" > "$repo/branch.txt"
  _run_ok git -C "$repo" add branch.txt
  _run_ok git -C "$repo" commit -qm "add $branch"
  _run_ok git -C "$repo" push -q origin "$branch"
  _run_ok git -C "$repo" checkout -q "$prev"
}
