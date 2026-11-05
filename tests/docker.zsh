#!/usr/bin/env zsh
# Run the gwt tests in Docker, matching the GitHub Actions environment
# (.github/workflows/tests.yml: ubuntu-latest with zsh, git, and jq).
#
# Usage:
#   tests/docker.zsh                     run every tests/*.test.zsh
#   tests/docker.zsh tests/cd.test.zsh   run specific files (passed to run.zsh)
#   tests/docker.zsh --build             rebuild the image before running
#   tests/docker.zsh --shell             open a shell in the test container
#
# The repository is bind-mounted at /workspace, so edits take effect without
# rebuilding. Exported GWT_* variables (e.g. GWT_TEST_KEEP=1) are passed
# through to the container.

emulate -L zsh

typeset -r IMAGE=gwt-tests
typeset -r REPO_ROOT=${${0:A}:h:h}

local build=0 shell=0
local -a passthrough
local arg
for arg in "$@"; do
  case "$arg" in
    --build) build=1 ;;
    --shell) shell=1 ;;
    --help|-h)
      print -r -- 'Usage: tests/docker.zsh [options] [test files...]'
      print -r -- ''
      print -r -- 'Run the gwt tests in Docker, matching the GitHub Actions environment.'
      print -r -- ''
      print -r -- 'Options:'
      print -r -- '  --build   rebuild the image before running'
      print -r -- '  --shell   open a shell in the test container'
      print -r -- '  --help    show this help'
      print -r -- ''
      print -r -- 'Any other arguments are passed to tests/run.zsh, and exported GWT_*'
      print -r -- 'variables (e.g. GWT_TEST_KEEP=1) are passed through to the container.'
      exit 0
      ;;
    *)
      passthrough+=("$arg")
      ;;
  esac
done

if ! (( $+commands[docker] )); then
  print -ru2 -- "docker.zsh: docker is not installed or not on PATH"
  exit 1
fi
if ! docker info >/dev/null 2>&1; then
  print -ru2 -- "docker.zsh: docker daemon is not running"
  exit 1
fi

if (( build )) || ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  print -r -- "# building image $IMAGE"
  docker build -t "$IMAGE" "$REPO_ROOT" || exit $?
fi

# CI checks out the repo fresh, so it never sees "dubious ownership" errors;
# the bind mount makes the files host-owned, so replicate the runner by
# trusting all directories before the tests run.
local -r setup='git config --global --add safe.directory "*" && exec "$@"'

# Forward every exported GWT_* variable into the container.
local -a env_args=()
local line name
for line in ${(f)"$(env)"}; do
  name=${line%%=*}
  [[ "$name" == GWT_* ]] && env_args+=(-e "$name")
done

local -a mount=("-v" "$REPO_ROOT:/workspace" "-w" "/workspace")

if (( shell )); then
  exec docker run --rm -it "${mount[@]}" "$IMAGE" \
    sh -c 'git config --global --add safe.directory "*" && exec bash' sh
fi

docker run --rm "${mount[@]}" "${env_args[@]}" "$IMAGE" \
  sh -c "$setup" sh zsh tests/run.zsh "${passthrough[@]}"
