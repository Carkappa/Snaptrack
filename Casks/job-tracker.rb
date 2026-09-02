# Homebrew Cask for Job Tracker.
#
#   brew tap Carkappa/snaptrack https://github.com/Carkappa/Snaptrack
#   brew install --cask job-tracker
#
# The repo (or at least its release assets) must be public for this to
# work for anyone but you. `version` is set by ./scripts/set-version.sh
# before a release; `sha256` by ./scripts/update-checksums.sh <version>
# after the release has built, since it hashes what CI actually produced.
cask "job-tracker" do
  version "0.8.0"
  sha256 "0be7df9a515f5a2789295784496bdf7e2f2d59a1791ec0eccb7d0b8855a90bdb"

  url "https://github.com/Carkappa/Snaptrack/releases/download/v#{version}/Job.Tracker_#{version}_aarch64.dmg",
      verified: "github.com/Carkappa/Snaptrack/"
  name "Job Tracker"
  desc "Tray-based job application tracker that saves straight to Excel"
  homepage "https://github.com/Carkappa/Snaptrack"

  # CI builds an Apple Silicon binary only. For Intel Macs, build from
  # source with `cargo tauri build`.
  depends_on arch: :arm64
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
