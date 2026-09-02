#!/usr/bin/env bash
# Bumps the app version everywhere it appears, from one command:
#
#   ./scripts/set-version.sh 0.2.0
#
# `src-tauri/Cargo.toml` is the source of truth. `tauri.conf.json` has no
# version field at all - Tauri falls back to Cargo.toml - so the only other
# copies are the two package manifests, which have to carry a literal version
# because package managers read them straight from the repo.
#
# Getting this wrong is quiet rather than loud: an installer built with a
# version that isn't newer than what's installed is simply never offered as an
# update. Hence one command, and a CI guard that the tag matches.
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <version>   (e.g. $0 0.2.0)" >&2
  exit 64
fi

version="$1"
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: '$version' is not a MAJOR.MINOR.PATCH version" >&2
  exit 64
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Only the [package] version at the top of the file, never a dependency's.
perl -0pi -e "s/^version = \"[^\"]*\"/version = \"$version\"/m" src-tauri/Cargo.toml
perl -pi -e "s/^  version \"[^\"]*\"/  version \"$version\"/" Casks/job-tracker.rb
perl -pi -e "s/^    \"version\": \"[^\"]*\"/    \"version\": \"$version\"/" bucket/job-tracker.json

# Scoop's concrete download URL carries the version twice, and it is not the
# same field as "version". Missing it left the manifest claiming one version
# while pointing at another release's installer - which, once that release was
# deleted, meant every `scoop install` 404'd.
perl -pi -e "s{releases/download/v[^/]+/Job\.Tracker_[^_]+_x64-setup\.exe}{releases/download/v$version/Job.Tracker_${version}_x64-setup.exe}" bucket/job-tracker.json

echo "Set version to $version in:"
echo "  src-tauri/Cargo.toml   (source of truth; tauri.conf.json inherits it)"
echo "  Casks/job-tracker.rb"
echo "  bucket/job-tracker.json   (version and download URL)"
echo
echo "Next: commit, then 'git tag v$version && git push origin v$version'."
echo "Once the release has built, run ./scripts/update-checksums.sh $version"
echo "so Homebrew and Scoop verify what they download."
