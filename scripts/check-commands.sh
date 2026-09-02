#!/usr/bin/env bash
#
# Every command the frontend calls must be in generate_handler![].
#
# A missing one is invisible until someone clicks the thing. Rust compiles,
# the frontend compiles, and the test harness mocks `invoke` by name so it
# passes there too - it only fails in the built app, at the moment a user
# needs it. That is exactly how the editable status list shipped broken:
# get_status_defs and set_status_defs both existed, and neither was
# registered.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

called="$(grep -oh 'invoke("[a-z_]*"' src/*.js | sed 's/invoke("//;s/"//' | sort -u)"
registered="$(sed -n '/generate_handler!\[/,/^        \]/p' src-tauri/src/lib.rs \
  | grep -o 'commands::[a-z_]*' | sed 's/commands:://' | sort -u)"

missing="$(comm -23 <(echo "$called") <(echo "$registered") || true)"

if [[ -n "$missing" ]]; then
  echo "Called from src/ but not registered in generate_handler![]:" >&2
  echo "$missing" | sed 's/^/  /' >&2
  echo >&2
  echo "Add them to src-tauri/src/lib.rs, or the app fails at runtime." >&2
  exit 1
fi

echo "All $(echo "$called" | wc -l | tr -d ' ') commands called from the frontend are registered."
