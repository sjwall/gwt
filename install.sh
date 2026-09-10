#!/bin/sh
#
# install.sh - Install gwt (Git WorkTree Helper)
#
# By default, downloads and installs the latest version from GitHub Releases.
# Also supports installing specific releases, local pre-built binaries,
# or running in-place.
#
set -e

REPO="sjwall/gwt"
DEFAULT_BIN_DIR="$HOME/.local/bin"
BIN_DIR="$DEFAULT_BIN_DIR"
DEFAULT_SHARE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/gwt"
SHARE_DIR="$DEFAULT_SHARE_DIR"
RELEASE_TAG=""
EXPLICIT_RELEASE=0
IN_PLACE=0
USE_LOCAL=0
CUSTOM_BINARY=""
ACTION="install"
TMP_DIR=""
SKILLS_ARG=""
NO_SKILLS=0
DRY_RUN=0

cleanup() {
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
        rm -rf "$TMP_DIR"
    fi
}
trap cleanup EXIT INT TERM

show_help() {
    cat <<EOF
Usage: $0 [OPTIONS]

Installs gwt (Git WorkTree Helper) and configures shell integration.
By default, downloads and installs the latest release from GitHub ($REPO).

OPTIONS:
    -r, --release <TAG>   Release version/tag to install (e.g. 'v0.1.0' or 'latest', default: latest)
    -v, --version <TAG>   Alias for --release
    --bin-dir <DIR>       Directory to install the binary to (default: \$HOME/.local/bin)
    --local               Install local pre-built binary instead of downloading
    --in-place            Point shell integration directly to pre-built binary without copying
    --binary <PATH>       Specify custom path to pre-built gwt binary
    --skills <TARGETS>    Symlink skills to global agent directories
                          (comma-separated: agents, opencode, claude, gemini, all, none)
    --no-skills           Skip symlinking skills (same as --skills=none)
    -n, --dry-run         Perform a dry run without making any changes
    -u, --uninstall       Remove installed binary, clean up shell integration, and unlink skills
    -h, --help            Show this help message

EXAMPLES:
    # Install latest release via curl
    curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/sjwall/gwt/main/install.sh | sh

    # Install specific release via curl
    curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/sjwall/gwt/main/install.sh | sh -s -- --release v0.1.0

    # Install latest release using local script
    ./install.sh

    # Install specific release using local script
    ./install.sh --release v0.1.0

    # Install from local build
    ./install.sh --local
EOF
}

detect_profile() {
    if [ -n "$ZDOTDIR" ] && [ -f "$ZDOTDIR/.zshrc" ]; then
        echo "$ZDOTDIR/.zshrc"
    elif [ -f "$HOME/.zshrc" ]; then
        echo "$HOME/.zshrc"
    elif [ -f "$HOME/.bashrc" ]; then
        echo "$HOME/.bashrc"
    elif [ -f "$HOME/.bash_profile" ]; then
        echo "$HOME/.bash_profile"
    elif [ -f "$HOME/.profile" ]; then
        echo "$HOME/.profile"
    else
        echo "$HOME/.zshrc"
    fi
}

clean_legacy_profile_lines() {
    target_file="$1"
    if [ -f "$target_file" ] && grep -q -E "gwt\.sh|gwt --shell-wrapper|gwt --init|_gwt_wrapper" "$target_file" 2>/dev/null; then
        tmp_clean="$(mktemp 2>/dev/null || mktemp -t gwt-clean)"
        awk '!/gwt\.sh/ && !/gwt --shell-wrapper/ && !/gwt --init/ && !/_gwt_wrapper/ && !/# gwt \(git worktree helper\)/' "$target_file" > "$tmp_clean"
        mv "$tmp_clean" "$target_file"
    fi
}

uninstall() {
    echo "==> Uninstalling gwt..."

    # 1. Unlink agent skills if gwt binary is available
    if command -v gwt >/dev/null 2>&1; then
        echo "Removing agent skill symlinks..."
        gwt skills none 2>/dev/null || true
    elif [ -x "$BIN_DIR/gwt" ]; then
        echo "Removing agent skill symlinks..."
        "$BIN_DIR/gwt" skills none 2>/dev/null || true
    elif [ -x "$BIN_DIR/gwt.exe" ]; then
        echo "Removing agent skill symlinks..."
        "$BIN_DIR/gwt.exe" skills none 2>/dev/null || true
    fi

    # 2. Remove installed binary
    if [ -f "$BIN_DIR/gwt" ]; then
        rm -f "$BIN_DIR/gwt"
        echo "Removed binary: $BIN_DIR/gwt"
    fi
    if [ -f "$BIN_DIR/gwt.exe" ]; then
        rm -f "$BIN_DIR/gwt.exe"
        echo "Removed binary: $BIN_DIR/gwt.exe"
    fi

    # 3. Clean up shell profile
    PROFILE_FILE="$(detect_profile)"
    if [ -f "$PROFILE_FILE" ]; then
        if grep -q -E "gwt\.sh|gwt --shell-wrapper|gwt --init|_gwt_wrapper" "$PROFILE_FILE" 2>/dev/null; then
            echo "Removing shell integration from $PROFILE_FILE..."
            clean_legacy_profile_lines "$PROFILE_FILE"
            echo "Cleaned up shell configuration in $PROFILE_FILE"
        fi
    fi

    # 4. Remove shared data directory if present and not a full git repo
    if [ -d "$SHARE_DIR" ] && [ ! -d "$SHARE_DIR/.git" ]; then
        rm -rf "$SHARE_DIR"
        echo "Removed data directory: $SHARE_DIR"
    fi

    echo "==> gwt has been successfully uninstalled."
    exit 0
}

resolve_latest_release() {
    # 1. Try URL redirect of /releases/latest (fastest, unauthenticated, no rate limit)
    LATEST_URL="$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest" 2>/dev/null || true)"
    RESOLVED="$(basename "$LATEST_URL")"
    if [ -n "$RESOLVED" ] && [ "$RESOLVED" != "latest" ]; then
        echo "$RESOLVED"
        return 0
    fi

    # 2. Try GitHub API latest release
    API_RESPONSE="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null || true)"
    RESOLVED="$(echo "$API_RESPONSE" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/')"
    if [ -n "$RESOLVED" ]; then
        echo "$RESOLVED"
        return 0
    fi

    # 3. Fallback to HTML releases page (scrapes first tag link, no rate limit)
    RESOLVED="$(curl -fsSL "https://github.com/${REPO}/releases" 2>/dev/null | grep -o '/releases/tag/[^"'\''?]*' | head -n 1 | sed 's|/releases/tag/||')"
    if [ -n "$RESOLVED" ]; then
        echo "$RESOLVED"
        return 0
    fi

    # 4. Fallback to GitHub API list (for prereleases)
    API_RESPONSE="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases" 2>/dev/null || true)"
    RESOLVED="$(echo "$API_RESPONSE" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/')"
    if [ -n "$RESOLVED" ]; then
        echo "$RESOLVED"
        return 0
    fi

    return 1
}

# Parse command-line arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --release=*)
            RELEASE_TAG="${1#*=}"
            EXPLICIT_RELEASE=1
            shift
            ;;
        -r|--release|-v|--version)
            if [ -z "$2" ] || [ "${2#-}" != "$2" ]; then
                echo "Error: $1 requires a release tag argument." >&2
                exit 1
            fi
            RELEASE_TAG="$2"
            EXPLICIT_RELEASE=1
            shift 2
            ;;
        --version=*)
            RELEASE_TAG="${1#*=}"
            EXPLICIT_RELEASE=1
            shift
            ;;
        --bin-dir=*)
            BIN_DIR="${1#*=}"
            shift
            ;;
        --bin-dir)
            if [ -z "$2" ] || [ "${2#-}" != "$2" ]; then
                echo "Error: --bin-dir requires a directory argument." >&2
                exit 1
            fi
            BIN_DIR="$2"
            shift 2
            ;;
        --binary=*)
            CUSTOM_BINARY="${1#*=}"
            shift
            ;;
        --binary)
            if [ -z "$2" ] || [ "${2#-}" != "$2" ]; then
                echo "Error: --binary requires a path argument." >&2
                exit 1
            fi
            CUSTOM_BINARY="$2"
            shift 2
            ;;
        --in-place)
            IN_PLACE=1
            shift
            ;;
        --local)
            USE_LOCAL=1
            shift
            ;;
        --skills=*)
            SKILLS_ARG="${1#*=}"
            shift
            ;;
        --skills)
            if [ -z "$2" ] || [ "${2#-}" != "$2" ]; then
                echo "Error: --skills requires a target argument." >&2
                exit 1
            fi
            SKILLS_ARG="$2"
            shift 2
            ;;
        --no-skills)
            NO_SKILLS=1
            shift
            ;;
        -n|--dry-run)
            DRY_RUN=1
            shift
            ;;
        --dry-run=*)
            case "${1#*=}" in
                0|false|no) DRY_RUN=0 ;;
                *) DRY_RUN=1 ;;
            esac
            shift
            ;;
        -u|--uninstall)
            ACTION="uninstall"
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo "Error: Unknown option '$1'." >&2
            echo "Run '$0 --help' for usage." >&2
            exit 1
            ;;
    esac
done

if [ "$ACTION" = "uninstall" ]; then
    uninstall
fi

if [ "$EXPLICIT_RELEASE" -eq 1 ] && [ -n "$CUSTOM_BINARY" ]; then
    echo "Error: Cannot specify both --release/--version and --binary." >&2
    exit 1
fi

if [ "$EXPLICIT_RELEASE" -eq 1 ] && [ "$IN_PLACE" -eq 1 ]; then
    echo "Error: Cannot specify both --release/--version and --in-place." >&2
    exit 1
fi

if [ "$EXPLICIT_RELEASE" -eq 1 ] && [ "$USE_LOCAL" -eq 1 ]; then
    echo "Error: Cannot specify both --release/--version and --local." >&2
    exit 1
fi

if [ -n "$CUSTOM_BINARY" ] && [ "$USE_LOCAL" -eq 1 ]; then
    echo "Error: Cannot specify both --binary and --local." >&2
    exit 1
fi

if [ "$NO_SKILLS" -eq 1 ] && [ -n "$SKILLS_ARG" ] && [ "$SKILLS_ARG" != "none" ]; then
    echo "Error: Cannot specify both --skills and --no-skills." >&2
    exit 1
fi

OS="$(uname -s)"
EXE_SUFFIX=""
case "$OS" in
    Darwin)
        OS_TYPE="Darwin"
        ;;
    Linux)
        OS_TYPE="Linux"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        OS_TYPE="Windows"
        EXE_SUFFIX=".exe"
        ;;
    *)
        echo "Error: Unsupported operating system: '$OS'." >&2
        exit 1
        ;;
esac

BIN_NAME="gwt${EXE_SUFFIX}"

echo "==> Setting up gwt..."

# 1. Locate or download binary
SOURCE_BIN=""
PROJECT_DIR="$(cd "$(dirname "$0")" 2>/dev/null && pwd)"

if [ -n "$CUSTOM_BINARY" ]; then
    if [ ! -f "$CUSTOM_BINARY" ]; then
        echo "Error: Specified binary does not exist: $CUSTOM_BINARY" >&2
        exit 1
    fi
    SOURCE_BIN="$CUSTOM_BINARY"
    echo "Using specified binary: $SOURCE_BIN"

elif [ "$IN_PLACE" -eq 1 ] || [ "$USE_LOCAL" -eq 1 ]; then
    if [ -f "$PROJECT_DIR/target/release/gwt$EXE_SUFFIX" ]; then
        SOURCE_BIN="$PROJECT_DIR/target/release/gwt$EXE_SUFFIX"
    elif [ -f "$PROJECT_DIR/target/debug/gwt$EXE_SUFFIX" ]; then
        SOURCE_BIN="$PROJECT_DIR/target/debug/gwt$EXE_SUFFIX"
    elif [ -f "$PROJECT_DIR/gwt$EXE_SUFFIX" ]; then
        SOURCE_BIN="$PROJECT_DIR/gwt$EXE_SUFFIX"
    elif command -v "gwt$EXE_SUFFIX" >/dev/null 2>&1; then
        SOURCE_BIN="$(command -v "gwt$EXE_SUFFIX")"
    elif command -v gwt >/dev/null 2>&1; then
        SOURCE_BIN="$(command -v gwt)"
    fi

    if [ -z "$SOURCE_BIN" ] || [ ! -f "$SOURCE_BIN" ]; then
        echo "Error: Local pre-built gwt binary not found!" >&2
        echo "Please build the project first:" >&2
        echo "    cargo build --release" >&2
        echo "" >&2
        echo "Or omit --local/--in-place to download from GitHub releases." >&2
        exit 1
    fi
    echo "Found local pre-built binary: $SOURCE_BIN"

elif [ "$EXPLICIT_RELEASE" -eq 0 ] && \
     [ "$0" != "sh" ] && [ "$0" != "-sh" ] && [ "$0" != "bash" ] && [ "$0" != "zsh" ] && \
     [ -f "$PROJECT_DIR/gwt$EXE_SUFFIX" ] && [ -f "$PROJECT_DIR/LICENSE" ] && [ ! -d "$PROJECT_DIR/.git" ] && [ ! -f "$PROJECT_DIR/Cargo.toml" ]; then
    # Running from an unpacked release archive
    SOURCE_BIN="$PROJECT_DIR/gwt$EXE_SUFFIX"
    echo "Using bundled release binary: $SOURCE_BIN"

else
    # Default: Download from GitHub Releases
    if ! command -v curl >/dev/null 2>&1; then
        echo "Error: 'curl' is required to download gwt." >&2
        exit 1
    fi

    TAG="$RELEASE_TAG"
    if [ -z "$TAG" ] || [ "$TAG" = "latest" ]; then
        echo "==> Fetching latest release info for ${REPO}..."
        TAG="$(resolve_latest_release)" || true
    fi

    if [ -z "$TAG" ]; then
        echo "Error: Could not determine the release version to install from ${REPO}." >&2
        echo "Please specify a release version using: $0 --release <TAG>" >&2
        exit 1
    fi

    # Normalize tag (prepend 'v' if it starts with a digit)
    case "$TAG" in
        v*) ;;
        [0-9]*) TAG="v$TAG" ;;
    esac

    echo "Selected release: $TAG"

    # Architecture detection
    ARCH="$(uname -m)"
    if [ "$OS_TYPE" = "Darwin" ]; then
        if [ "$ARCH" = "x86_64" ] && [ "$(sysctl -in sysctl.proc_translated 2>/dev/null)" = "1" ]; then
            ARCH="arm64"
        fi

        case "$ARCH" in
            arm64|aarch64)
                TARGET="aarch64-apple-darwin"
                ;;
            x86_64|amd64)
                TARGET="x86_64-apple-darwin"
                ;;
            *)
                TARGET="universal-apple-darwin"
                ;;
        esac
    elif [ "$OS_TYPE" = "Linux" ]; then
        case "$ARCH" in
            arm64|aarch64)
                TARGET="aarch64-unknown-linux-gnu"
                ;;
            x86_64|amd64)
                TARGET="x86_64-unknown-linux-gnu"
                ;;
            *)
                TARGET="${ARCH}-unknown-linux-gnu"
                ;;
        esac
    elif [ "$OS_TYPE" = "Windows" ]; then
        case "$ARCH" in
            arm64|aarch64)
                TARGET="aarch64-pc-windows-msvc"
                ;;
            *)
                TARGET="x86_64-pc-windows-msvc"
                ;;
        esac
    fi

    TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'gwt-install')"

    ASSET_NAME="gwt-${TAG}-${TARGET}.tar.gz"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET_NAME}"

    echo "==> Downloading gwt ${TAG} (${TARGET})..."
    if ! curl --proto '=https' --tlsv1.2 -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/$ASSET_NAME"; then
        if [ "$OS_TYPE" = "Darwin" ] && [ "$TARGET" != "universal-apple-darwin" ]; then
            echo "Target-specific archive not found, trying universal binary..."
            TARGET="universal-apple-darwin"
            ASSET_NAME="gwt-${TAG}-${TARGET}.tar.gz"
            DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET_NAME}"
            if ! curl --proto '=https' --tlsv1.2 -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/$ASSET_NAME"; then
                echo "Error: Failed to download release asset for tag '${TAG}'." >&2
                echo "URL: $DOWNLOAD_URL" >&2
                exit 1
            fi
        elif [ "$OS_TYPE" = "Windows" ]; then
            # Try zip fallback on Windows
            ASSET_NAME="gwt-${TAG}-${TARGET}.zip"
            DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET_NAME}"
            if ! curl --proto '=https' --tlsv1.2 -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/$ASSET_NAME"; then
                echo "Error: Failed to download release asset for tag '${TAG}'." >&2
                echo "URL: $DOWNLOAD_URL" >&2
                exit 1
            fi
        else
            echo "Error: Failed to download release asset for tag '${TAG}'." >&2
            echo "URL: $DOWNLOAD_URL" >&2
            exit 1
        fi
    fi

    # Verify checksum if SHA256SUMS.txt is available
    CHECKSUM_URL="https://github.com/${REPO}/releases/download/${TAG}/SHA256SUMS.txt"
    if curl --proto '=https' --tlsv1.2 -fsSL "$CHECKSUM_URL" -o "$TMP_DIR/SHA256SUMS.txt" 2>/dev/null; then
        EXPECTED_SHA="$(grep "[[:space:]]${ASSET_NAME}\$" "$TMP_DIR/SHA256SUMS.txt" 2>/dev/null | awk '{print $1}')"
        if [ -n "$EXPECTED_SHA" ]; then
            if command -v shasum >/dev/null 2>&1; then
                ACTUAL_SHA="$(shasum -a 256 "$TMP_DIR/$ASSET_NAME" | awk '{print $1}')"
            elif command -v sha256sum >/dev/null 2>&1; then
                ACTUAL_SHA="$(sha256sum "$TMP_DIR/$ASSET_NAME" | awk '{print $1}')"
            else
                ACTUAL_SHA=""
            fi

            if [ -n "$ACTUAL_SHA" ]; then
                if [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
                    echo "Error: Checksum verification failed for ${ASSET_NAME}!" >&2
                    echo "  Expected: $EXPECTED_SHA" >&2
                    echo "  Actual:   $ACTUAL_SHA" >&2
                    exit 1
                fi
                echo "Checksum verified: $ACTUAL_SHA"
            fi
        fi
    fi

    # Extract archive
    echo "==> Extracting $ASSET_NAME..."
    case "$ASSET_NAME" in
        *.tar.gz|*.tgz)
            if ! command -v tar >/dev/null 2>&1; then
                echo "Error: 'tar' is required to extract gwt archive." >&2
                exit 1
            fi
            tar -xzf "$TMP_DIR/$ASSET_NAME" -C "$TMP_DIR"
            ;;
        *.zip)
            if command -v unzip >/dev/null 2>&1; then
                unzip -q "$TMP_DIR/$ASSET_NAME" -d "$TMP_DIR"
            elif command -v tar >/dev/null 2>&1; then
                tar -xf "$TMP_DIR/$ASSET_NAME" -C "$TMP_DIR"
            else
                echo "Error: 'unzip' or 'tar' is required to extract zip archive." >&2
                exit 1
            fi
            ;;
    esac

    DOWNLOADED_BIN="$(find "$TMP_DIR" -type f \( -name "gwt" -o -name "gwt.exe" \) | head -n 1)"
    if [ -z "$DOWNLOADED_BIN" ] || [ ! -f "$DOWNLOADED_BIN" ]; then
        echo "Error: Could not locate 'gwt' binary in extracted archive." >&2
        exit 1
    fi
    chmod +x "$DOWNLOADED_BIN"
    SOURCE_BIN="$DOWNLOADED_BIN"
fi

# 2. Determine target binary path
if [ "$IN_PLACE" -eq 1 ]; then
    TARGET_BIN="$(cd "$(dirname "$SOURCE_BIN")" && pwd)/$(basename "$SOURCE_BIN")"
    echo "Using binary in-place: $TARGET_BIN"
else
    echo "Installing binary to $BIN_DIR/$BIN_NAME..."
    if [ "$DRY_RUN" -eq 1 ]; then
        echo "[dry-run] Would copy $SOURCE_BIN to $BIN_DIR/$BIN_NAME"
    else
        mkdir -p "$BIN_DIR"
        cp "$SOURCE_BIN" "$BIN_DIR/$BIN_NAME"
        chmod +x "$BIN_DIR/$BIN_NAME"
    fi
    TARGET_BIN="$BIN_DIR/$BIN_NAME"
fi

# 3. Setup shared data directory and agent skills
if [ "$DRY_RUN" -eq 0 ]; then
    mkdir -p "$SHARE_DIR/.agents/skills/gwt"
    if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ] && [ -d "$TMP_DIR/.agents/skills" ]; then
        cp -R "$TMP_DIR/.agents/skills"/* "$SHARE_DIR/.agents/skills/" 2>/dev/null || true
    elif [ -d "$PROJECT_DIR/.agents/skills" ]; then
        cp -R "$PROJECT_DIR/.agents/skills"/* "$SHARE_DIR/.agents/skills/" 2>/dev/null || true
    else
        curl -fsSL "https://raw.githubusercontent.com/${REPO}/main/.agents/skills/gwt/SKILL.md" -o "$SHARE_DIR/.agents/skills/gwt/SKILL.md" 2>/dev/null || true
    fi

    # Copy wrapper and completion scripts to data directory if present
    if [ -f "$PROJECT_DIR/_gwt_wrapper" ]; then
        cp "$PROJECT_DIR/_gwt_wrapper" "$SHARE_DIR/_gwt_wrapper" 2>/dev/null || true
    fi
    if [ -f "$PROJECT_DIR/_gwt" ]; then
        cp "$PROJECT_DIR/_gwt" "$SHARE_DIR/_gwt" 2>/dev/null || true
    fi
fi

# 4. Configure shell integration
PROFILE_FILE="$(detect_profile)"

if [ "$DRY_RUN" -eq 1 ]; then
    echo "[dry-run] Would clean legacy gwt.sh references and ensure shell integration in $PROFILE_FILE"
else
    # Remove any old gwt.sh references
    if [ -f "$PROFILE_FILE" ] && grep -q "gwt\.sh" "$PROFILE_FILE" 2>/dev/null; then
        echo "Removing legacy gwt.sh configuration from $PROFILE_FILE..."
        clean_legacy_profile_lines "$PROFILE_FILE"
    fi

    # Check if shell wrapper is already present
    if [ -f "$PROFILE_FILE" ] && grep -q -E "gwt --shell-wrapper|gwt --init|_gwt_wrapper" "$PROFILE_FILE" 2>/dev/null; then
        echo "Shell integration already present in $PROFILE_FILE"
    else
        echo "Adding shell integration to $PROFILE_FILE..."
        mkdir -p "$(dirname "$PROFILE_FILE")"
        {
            echo ""
            echo "# gwt (git worktree helper)"
            echo "eval \"\$(gwt --shell-wrapper)\""
        } >> "$PROFILE_FILE"
        echo "Added shell wrapper to $PROFILE_FILE"
    fi
fi

# 5. Link agent skills
dry_flag=""
[ "$DRY_RUN" -eq 1 ] && dry_flag="-n"

if [ "$NO_SKILLS" -eq 1 ]; then
    echo "Skipping skills symlinking (--no-skills specified)."
elif [ -n "$SKILLS_ARG" ]; then
    if [ "$SKILLS_ARG" != "none" ]; then
        echo "Configuring agent skills: $SKILLS_ARG..."
        if [ "$DRY_RUN" -eq 0 ]; then
            "$TARGET_BIN" skills --dir="$SHARE_DIR" "$SKILLS_ARG" 2>/dev/null || true
        else
            echo "[dry-run] Would run: $TARGET_BIN skills --dir=$SHARE_DIR $SKILLS_ARG"
        fi
    fi
elif [ -t 0 ] || ([ -e /dev/tty ] && [ -r /dev/tty ] && [ -w /dev/tty ]); then
    if [ "$DRY_RUN" -eq 0 ]; then
        "$TARGET_BIN" skills --dir="$SHARE_DIR" --prompt </dev/tty || true
    fi
else
    if [ "$DRY_RUN" -eq 0 ]; then
        "$TARGET_BIN" skills --dir="$SHARE_DIR" --sync 2>/dev/null || true
    fi
fi

UNINSTALL_CMD="$0 --uninstall"
case "$0" in
    sh|-sh|bash|zsh)
        UNINSTALL_CMD="curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --uninstall"
        ;;
esac

echo ""
echo "================================================================="
echo "                  gwt Installation Complete                      "
echo "================================================================="
echo "Binary:        $TARGET_BIN"
echo "Shell config:  $PROFILE_FILE"
echo "Data dir:      $SHARE_DIR"
echo ""

case ":$PATH:" in
    *:"$(dirname "$TARGET_BIN")":*) ;;
    *)
        echo "PATH Notice:"
        echo "  '$(dirname "$TARGET_BIN")' is not in your current PATH."
        echo "  Consider adding it to your shell configuration (e.g. ~/.zshrc):"
        echo "      export PATH=\"$(dirname "$TARGET_BIN"):\$PATH\""
        echo ""
        ;;
esac

echo "To start using gwt, reload your shell configuration:"
echo "  source $PROFILE_FILE"
echo ""
echo "Or start a new terminal session."
echo ""
echo "Management commands:"
echo "  Manage skills: gwt skills"
echo "  Uninstall:     $UNINSTALL_CMD"
echo "================================================================="
