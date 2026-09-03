#!/usr/bin/env bash
#
# Points git at scripts/hooks, so the hooks are version-controlled rather
# than living unshared in .git/hooks. Run once per clone.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

git config core.hooksPath scripts/hooks
chmod +x scripts/hooks/* 2>/dev/null || true

echo "Hooks installed: git will now run scripts/hooks/pre-push before a push."
echo "Skip one with: git push --no-verify"
