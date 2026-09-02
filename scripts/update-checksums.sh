#!/usr/bin/env bash
#
# Points the Homebrew Cask and the Scoop manifest at a published release's
# real checksums.
#
#   ./scripts/update-checksums.sh 0.6.0
#
# Run it after the release workflow has finished, since it downloads the
# installers CI produced. Without this both install paths carry placeholder
# checksums, which means Scoop refuses the download outright and Homebrew
# verifies nothing at all.
set -euo pipefail

version="${1:-}"
if [[ -z "$version" ]]; then
  echo "usage: $0 <version>   e.g. $0 0.6.0" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

base="https://github.com/Carkappa/Snaptrack/releases/download/v$version"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fetch_sha() {
  local name="$1"
  echo "  downloading $name" >&2
  if ! curl -fsSL -o "$tmp/$name" "$base/$name"; then
    echo "Could not download $name - has the release finished building?" >&2
    exit 1
  fi
  sha256sum < "$tmp/$name" | cut -d' ' -f1
}

echo "Fetching v$version installers..."
exe_sha="$(fetch_sha "Job.Tracker_${version}_x64-setup.exe")"
arm_sha="$(fetch_sha "Job.Tracker_${version}_aarch64.dmg")"
intel_sha="$(fetch_sha "Job.Tracker_${version}_x64.dmg")"

# The Cask carries one hash per Mac architecture.
perl -pi -e "s/^  sha256 arm:.*/  sha256 arm:   \"$arm_sha\",/" Casks/job-tracker.rb
perl -pi -e "s/^         intel:.*/         intel: \"$intel_sha\"/" Casks/job-tracker.rb
perl -pi -e "s/^(\s*)\"hash\": \".*\"/\${1}\"hash\": \"$exe_sha\"/" bucket/job-tracker.json

echo
echo "Casks/job-tracker.rb     arm   $arm_sha"
echo "                         intel $intel_sha"
echo "bucket/job-tracker.json  hash  $exe_sha"
echo
echo "Commit these - the install paths do not verify anything until you do."
