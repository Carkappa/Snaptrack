# Homebrew Cask for Job Tracker.
#
#   brew tap Carkappa/snaptrack https://github.com/Carkappa/Snaptrack
#   brew install --cask job-tracker
#
# The repo (or at least its release assets) must be public for this to
# work for anyone but you. `version` is set by ./scripts/set-version.sh
# before a release; both `sha256` lines by ./scripts/update-checksums.sh
# <version> after the release has built, since it hashes what CI produced.
cask "job-tracker" do
  # CI builds both Mac architectures; Homebrew picks the right one.
  arch arm: "aarch64", intel: "x64"

  version "0.10.2"
  sha256 arm:   "6093f2af412cfcc1455652d984f9f18d1905980d1ec29aeccdaa72b76fa6adcf",
         intel: "d46c47ed899fbe38fae81c50e92591878049999f2889ae42a7d968831901dc3b"

  url "https://github.com/Carkappa/Snaptrack/releases/download/v#{version}/Job.Tracker_#{version}_#{arch}.dmg",
      verified: "github.com/Carkappa/Snaptrack/"
  name "Job Tracker"
  desc "Tray-based job application tracker that saves straight to Excel"
  homepage "https://github.com/Carkappa/Snaptrack"

  depends_on macos: ">= :catalina"

  app "Job Tracker.app"

  postflight do
    # The build is unsigned, so clear the quarantine flag Gatekeeper sets.
    system_command "/usr/bin/xattr",
                   args: ["-cr", "#{appdir}/Job Tracker.app"],
                   sudo: false
  end

  zap trash: [
    "~/Library/Application Support/com.justindu.jobtracker",
    "~/Library/Caches/com.justindu.jobtracker",
    "~/Library/HTTPStorages/com.justindu.jobtracker",
    "~/Library/Preferences/com.justindu.jobtracker.plist",
    "~/Library/Saved Application State/com.justindu.jobtracker.savedState",
    "~/Library/WebKit/com.justindu.jobtracker",
  ]
end
