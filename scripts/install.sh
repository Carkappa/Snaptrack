#!/bin/bash
# Job Tracker - macOS terminal installer.
#
#   curl -fsSL https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.sh | bash
#
# Downloads the latest release .dmg, installs Job Tracker.app into
# /Applications, and clears the quarantine flag (the build is unsigned,
# so Gatekeeper would otherwise refuse to open it).

set -euo pipefail

REPO="Carkappa/Snaptrack"
APP_NAME="Job Tracker.app"
INSTALL_DIR="/Applications"

info() { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
fail() { printf '\033[1;31mError:\033[0m %s\n' "$1" >&2; exit 1; }

[ "$(uname -s)" = "Darwin" ] || fail "This installer is for macOS. On Windows use scripts/install.ps1."

info "Looking up the latest release of $REPO"
API_URL="https://api.github.com/repos/$REPO/releases/latest"
DMG_URL=$(curl -fsSL "$API_URL" \
  | grep -o '"browser_download_url": *"[^"]*\.dmg"' \
  | head -1 \
  | sed 's/.*"browser_download_url": *"\(.*\)"/\1/')

[ -n "$DMG_URL" ] || fail "No .dmg found in the latest release. If the repo is private, download it manually from https://github.com/$REPO/releases"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
DMG_PATH="$TMP_DIR/job-tracker.dmg"

info "Downloading $(basename "$DMG_URL")"
curl -fsSL "$DMG_URL" -o "$DMG_PATH"

info "Mounting the disk image"
MOUNT_POINT=$(hdiutil attach -nobrowse -noverify -noautoopen "$DMG_PATH" \
  | grep -o '/Volumes/.*' | head -1)
[ -n "$MOUNT_POINT" ] || fail "Could not mount the downloaded disk image."
trap 'hdiutil detach "$MOUNT_POINT" >/dev/null 2>&1 || true; rm -rf "$TMP_DIR"' EXIT

if [ -d "$INSTALL_DIR/$APP_NAME" ]; then
  info "Replacing the existing install at $INSTALL_DIR/$APP_NAME"
  rm -rf "${INSTALL_DIR:?}/${APP_NAME:?}"
fi

info "Installing to $INSTALL_DIR"
cp -R "$MOUNT_POINT/$APP_NAME" "$INSTALL_DIR/"

info "Clearing the quarantine flag (this build is unsigned)"
xattr -cr "$INSTALL_DIR/$APP_NAME"

info "Done. Launching Job Tracker - look for the icon in your menu bar."
open -a "$INSTALL_DIR/$APP_NAME"

cat <<'EOF'

  Job Tracker is now running in your menu bar.
    - Press Cmd+Shift+J from anywhere to open the capture panel
    - Cmd+V pastes a screenshot of a job posting
    - Esc hides the window; Quit from the tray menu to exit

  Screenshot extraction uses Tesseract by default (free, offline):
    brew install tesseract
  Or switch to the Claude API in Settings for better accuracy.

EOF
