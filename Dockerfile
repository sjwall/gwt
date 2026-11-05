# Mirrors the GitHub Actions test environment (.github/workflows/tests.yml):
# ubuntu-latest with zsh installed. The runner image also ships git and jq,
# which the tests rely on, so they are installed explicitly here.
#
# Run the tests with tests/docker.zsh, which bind-mounts this repository at
# /workspace (so no rebuild is needed after edits).
FROM ubuntu:24.04

RUN apt-get update \
    && apt-get install -y --no-install-recommends zsh git jq \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

CMD ["zsh", "tests/run.zsh"]
