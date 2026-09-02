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

  version "0.8.0"
  sha256 arm:   "61444441881376e1ecb12e211bb472b9a958db5c78c331d6396c94704eee2a82",
         intel: "283c3156d666b7ba173d49c839e7fd225fdd889c12c7b83e34427263ac00b1c7"

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
