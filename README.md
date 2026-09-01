# Job Tracker

A native, installable desktop app for tracking job applications. Lives in
the tray/menu bar; a global hotkey pops open a small capture panel so you
can screenshot a job posting, paste it, and have the fields pulled out
for you — reviewed and saved straight into an Excel workbook.

- **Tauri v2** (Rust backend + the OS's own WebView — no bundled Chromium)
- **Vanilla HTML/CSS/JS** frontend — no React, no bundler, no `npm install`
- No background polling, no timers, no server process. The app is inert
  until you invoke it via the tray icon or the global hotkey.
- **Updates itself.** One check on launch, at most once a day; a new
  version downloads and installs on its own, waiting until you have no
  unsaved entry open. Both the checking and the auto-install are
  switchable in Settings.
- Ships as a macOS `.dmg` and a Windows `.msi`/`.exe`.

## How it works

1. Apply on LinkedIn (or anywhere), screenshot the page.
2. Press `Cmd+Shift+J` (macOS) / `Ctrl+Shift+J` (Windows) from anywhere.
3. Press `Cmd+V` / `Ctrl+V` in the capture panel.
4. Company, position, location, work type, employment type, salary
   range, job ID, posted date, and notes get pulled out automatically —
   see **Extraction methods** below for how.
5. Review/correct the fields in the form, hit Enter to save.
6. The row is written into your `JobApplications.xlsx`.

No screenshot? Click **Skip screenshot** for a blank form — the app is
fully usable with zero API calls and zero setup.

## The calendar

The **Calendar** tab turns the workbook into a month grid: each day
carries the number of applications saved with that Date Applied, shaded
from pale to solid against the busiest day of the month on screen — so
a productive week and a quiet one are distinguishable without reading a
single row.

- **Month** and **Year** views. The year view is a GitHub-contributions
  style grid — one column per week, one square per day — that opens
  scrolled to whatever is worth looking at (today, or the first day with
  anything on it). Clicking a square drops into that month with the day
  selected.
- Arrows page month to month, or year to year in the year view.
  **Today** jumps back and selects today.
- Clicking a day lists that day's applications underneath; clicking one
  of those opens it in the edit form, the same as clicking a row in the
  Applications tab.
- Above the grid: the month's total, how many days you were active,
  your busiest day, and the current run of consecutive days with at
  least one application out.
- Days from the neighbouring months fill out the first and last weeks.
  They're greyed, never shaded, and don't skew the month's colour
  scale; clicking one pages to that month.

The counts come from the Date Applied column, which is the date the
capture was saved — so screenshots captured and saved on the day you
applied land on the right square with no extra bookkeeping. Rows whose
date can't be read (hand-edited to something odd, or left blank) are
counted out and reported in a line under the grid rather than silently
dropped.

## Extraction methods

Switch between these anytime in Settings — no restart needed:

- **Tesseract (default, free, fully offline).** Shells out to a locally
  installed [Tesseract](https://github.com/tesseract-ocr/tesseract)
  binary, then guesses which recognized text block is the company vs.
  the position vs. the location using layout heuristics (the job title
  is almost always the single largest text on the page). Meaningfully
  less accurate than Claude — it can't actually *understand* the image,
  just read text off it — so the full raw OCR text is always attached to
  the Notes field for you to cross-check and fix by hand. Requires
  Tesseract to be installed and on your `PATH`:
  - macOS: `brew install tesseract`
  - Windows: the [UB-Mannheim installer](https://github.com/UB-Mannheim/tesseract/wiki) (check "Add to PATH")
  - Linux: `apt install tesseract-ocr` or your distro's equivalent

  If Tesseract isn't found, Settings tells you so and you can switch to
  Claude, or just use manual entry.
- **Claude API (opt-in, most accurate, needs a key).** Sends the
  screenshot to Claude, which actually reads the page the way you would
  and returns structured JSON. Needs an Anthropic API key (see below).

Either way, nothing is ever invented — a field that isn't visible (or
that the extractor isn't confident about) comes back empty for you to
fill in, never guessed.

## Installing

### macOS

One-liner (downloads the latest release, installs to `/Applications`,
clears the Gatekeeper quarantine flag, and launches it):

```bash
curl -fsSL https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.sh | bash
```

Or via Homebrew:

```bash
brew tap Carkappa/snaptrack https://github.com/Carkappa/Snaptrack
brew install --cask job-tracker
```

Apple Silicon only — CI builds an `aarch64` binary. On an Intel Mac,
build from source with `cargo tauri build`.

### Windows

One-liner in PowerShell (downloads the latest release installer and runs
it silently):

```powershell
irm https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.ps1 | iex
```

Or via [Scoop](https://scoop.sh):

```powershell
scoop bucket add snaptrack https://github.com/Carkappa/Snaptrack
scoop install snaptrack/job-tracker
```

### Manual download

Grab the `.dmg` (macOS) or `-setup.exe` / `.msi` (Windows) from the
[Releases page](https://github.com/Carkappa/Snaptrack/releases). See
[About the unsigned builds](#about-the-unsigned-builds) for the
Gatekeeper/SmartScreen warnings you'll need to click through.

## Project layout

```
src-tauri/            Rust backend (the actual application logic)
  src/
    lib.rs            App setup: tray icon, global shortcut, window behavior
    commands.rs        All #[tauri::command]s exposed to the frontend (with tests)
    extraction.rs      Anthropic API call + JSON parsing (with tests)
    local_ocr.rs         Tesseract-backed extraction + layout heuristics (with tests)
    excel.rs            xlsx read/write, CSV export, timestamped backups (with tests)
    updates.rs          Update check, auto-install, progress events (with tests)
    keychain.rs         API key storage via the keyring crate
    models.rs            Shared data types
  capabilities/         Tauri v2 permission grants for the main window
  icons/                 App icons (.icns / .ico / .png)
  tauri.conf.json         Window, bundle, and tray configuration
scripts/set-version.sh  Bumps the version everywhere it appears, from one command
src/                    Frontend — plain index.html + styles.css + app.js
    calendar.js           Date math + month/year aggregation behind the Calendar tab
CLAUDE.md               Repo conventions and traps, for anyone (or anything) editing it
tests/                  Browser-run frontend tests (no npm, no runner)
  calendar.test.html     Pure-logic tests for src/calendar.js
  ui-harness.html        The real UI with the Tauri bridge mocked, for clicking through
.github/workflows/      CI: Rust + frontend tests on every push, installers on a tag
```

## Prerequisites

- **Rust** (stable), via [rustup](https://rustup.rs) or `brew install rust`
- **Tauri CLI**: `cargo install tauri-cli --version "^2.0.0" --locked`
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Windows: the [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
  with the "Desktop development with C++" workload, and
  [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)
  (preinstalled on Windows 11 and most Windows 10 machines)
- **Tesseract** (optional, but it's the default extraction method — see
  above for install commands). The app builds and runs fine without it;
  screenshot extraction just falls back to telling you it's missing
  until you install it or switch to Claude in Settings.

There is no Node.js/npm dependency — the frontend is plain files served
directly from `src/`, so there's no `npm install` step at all.

## Running it locally

```bash
cargo tauri dev
```

This launches the app with hot-reload for the Rust side and live file
serving for the frontend. The window starts hidden in the tray on first
run in dev mode too — use the hotkey or click the tray icon to open it.

## Running the tests

```bash
cd src-tauri
cargo test
```

This also runs on every push and pull request — see
`.github/workflows/test.yml`, which additionally runs the frontend suite
below in headless Chrome.

Covers: stripping ```` ``` ```` fences from Claude's response, parsing
(and gracefully failing to parse) extracted JSON, the Tesseract
layout-heuristic field guesser (largest text = title, line above it =
company, regex-based location/salary/job-ID/date detection), the Excel
read/write round-trip, CSV export, timestamped backups and pruning,
duplicate-key matching (case-insensitive, trimmed), and the
once-a-day update-check throttle. A separate
`tests/full_pipeline.rs` integration suite drives the real
`#[tauri::command]` functions (not just internal modules) against a
temporary workbook, including unzipping the saved `.xlsx` to confirm
the status dropdown, fill colors, and hyperlink are really in the file.

Two tests are `#[ignore]`d because they depend on state outside the
test itself — run them manually when you want to sanity-check the real
OS integration:

```bash
# Reads whatever image is really on your clipboard right now:
cargo test --test full_pipeline -- --ignored reads_real_clipboard_image --nocapture

# Runs real Tesseract extraction against a screenshot file and prints what it guessed:
OCR_TEST_IMAGE=/path/to/screenshot.png \
  cargo test --test full_pipeline -- --ignored extracts_from_a_real_screenshot --nocapture
```

### Frontend tests

The frontend has no build step and no npm dependencies, so its tests are
two HTML files you open in a browser. Serve the repo root and visit them:

```bash
python3 -m http.server 8000
```

- <http://localhost:8000/tests/calendar.test.html> — the calendar's date
  math and aggregation: date parsing (including hand-edited and invalid
  values), leap years, the 6x7 month grid and the 53-week year grid with
  their padding, heat-level scaling, per-month and per-year totals,
  streaks across month boundaries, and year wrapping. The page title reads
  `PASS n/n` or `FAIL n/n`, with a per-test list. **This one also runs in
  CI**, in headless Chrome, so a red result fails the build.
- <http://localhost:8000/tests/ui-harness.html> — the real `index.html`
  and `app.js`, with `window.__TAURI__` replaced by a mock that serves
  fixture rows. Every tab, the capture form, the save flow, and the
  calendar are clickable without building or launching the app.

## Building the installers

```bash
cargo tauri build
```

Run this **on the target OS** — a macOS `.dmg` can only be built on
macOS, and a Windows `.msi`/`.exe` only on Windows (Tauri doesn't cross-
compile the platform installer, even though the Rust code itself is
portable). Output lands in `src-tauri/target/release/bundle/`:

- macOS: `dmg/Job Tracker_<version>_aarch64.dmg` (or `x64` on Intel Macs)
- Windows: `msi/Job Tracker_<version>_x64_en-US.msi` and
  `nsis/Job Tracker_<version>_x64-setup.exe`

### First-run setup

On first launch (or whenever no API key is stored), the app shows a
setup screen asking for your Anthropic API key. It's stored in the OS
keychain (Keychain Access on macOS, Credential Manager on Windows) via
the `keyring` crate — never written to a plaintext file or read from an
environment variable. You can skip this screen entirely and use the app
in manual-entry mode; add the key later from Settings if you change your
mind.

By default, the workbook is created at `~/Documents/JobApplications.xlsx`.
Change the path from the Settings tab at any time.

## Beyond the basics

- **Edit any saved row.** Click a row in the Applications tab (anywhere
  but the Status dropdown) to reopen it in the full edit form.
- **Delete a row.** Hover a row and click the &times; on the right. It
  asks first, and the workbook is backed up before the rewrite, so a
  mistake is recoverable from `backups/`. Deleting checks that the row is
  still the one you were looking at, in case the workbook changed in Excel
  while the app had it listed.
- **Sort the list.** Click Date, Company, Position, or Status to sort by
  it; click again to reverse. Newest-first by default, since the workbook
  itself is append-ordered. Sorting is display-only — it never reorders
  the rows in the file.
- **CSV export.** One click in the Applications tab writes a sibling
  `JobApplications.csv` next to the workbook.
- **Openable links.** The `link` cell in the Applications table opens the
  posting in your real browser. Only `http`/`https` are accepted — a row's
  URL came off a screenshot or a spreadsheet cell, not from this app, so
  the scheme is checked rather than trusted.
- **Automatic backups.** Every save copies the previous workbook into a
  `backups/` folder next to it first (timestamped, capped at the last
  10), so a bad edit never loses prior data.
- **Screenshot archive.** When a save came from a screenshot, a copy is
  kept in `JobApplications_screenshots/` next to the workbook, named by
  date/company/position — handy if a listing gets taken down later.
- **Status counts.** A small `12 Applied · 3 Interviewing · ...` line
  above the Applications list, computed from what's already loaded.
- **Calendar.** A month grid shading each day by how many applications
  went out that day, so a slow week is visible at a glance. See below.
- **Automatic updates.** A newer release installs itself and restarts
  into the new version, with a progress bar and an off switch. See
  below.

## Automatic updates

Installed copies check GitHub Releases for a newer version and offer to
install it in place. This keeps the app's "no background work" promise:

- **One check per launch, at most one a day.** No timer, no polling.
  The check is a single HTTPS request made when the window initialises,
  and it's skipped entirely if one already went out in the last 24
  hours.
- **It installs itself.** By default a found update downloads, verifies
  its signature, installs, and restarts into the new version without
  being asked. A banner above the tabs shows the version and a download
  progress bar throughout, so it's never silent.
- **It never restarts over your work.** If the capture form has an
  unsaved entry in it, the install holds off and the banner offers
  **Install & restart** instead. It goes ahead on its own the moment you
  save that entry.
- **Two off switches.** Settings → Updates has "Check for a new version
  on launch" and "Install updates automatically, without asking".
  Unchecking the second gives you the ask-first banner; unchecking the
  first stops the automatic check altogether. **Check now** works either
  way, and ignores both the once-a-day throttle and the checking
  preference — you asked for that one explicitly.
- **Failures are inert.** Offline, endpoint down, or updates not
  configured — the check fails quietly and the app carries on. A failed
  *install* puts the banner back into its actionable state and surfaces
  the same retry toast as a failed save.

### Setting it up (repo owner, one time)

Updates are signed, so the app will only install a build that came from
your private key. Until that key exists, the app reports "no update
signing key configured" instead of checking, and everything else works
normally.

1. Generate a keypair (keep the passphrase somewhere safe):

   ```bash
   cd src-tauri
   cargo tauri signer generate -w ~/.tauri/snaptrack.key
   ```

2. Paste the **public** key it prints into `src-tauri/tauri.conf.json`,
   replacing `REPLACE_WITH_YOUR_TAURI_PUBLIC_KEY` under
   `plugins.updater.pubkey`. This one is meant to be committed.

3. Add two repository secrets (Settings → Secrets and variables →
   Actions) so CI can sign each release:

   | Secret | Value |
   | --- | --- |
   | `TAURI_SIGNING_PRIVATE_KEY` | contents of `~/.tauri/snaptrack.key` |
   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | the passphrase you chose |

   Never commit the private key itself.

4. Tag and push a release as usual. The workflow bundles the installers,
   signs them, and attaches a `latest.json` manifest — which is what the
   endpoint in `tauri.conf.json` points at:

   ```
   https://github.com/Carkappa/Snaptrack/releases/latest/download/latest.json
   ```

**The release has to be published, not left as a draft.** The workflow
creates releases as drafts on purpose so you can check the artifacts
first; GitHub doesn't serve draft assets, so installed apps see nothing
until you hit Publish.

### Bumping the version

`src-tauri/Cargo.toml` is the source of truth — `tauri.conf.json` has no
`version` field at all and inherits it. The two package manifests have to
carry a literal copy, so one script keeps all three in step:

```bash
./scripts/set-version.sh 0.2.0
```

An installer whose version isn't newer than what's installed is simply
never offered as an update, and nothing about that failure is visible, so
the release workflow also refuses to build when the tag and the crate
version disagree.

## CI: building both installers automatically

`.github/workflows/release.yml` builds both installers on a
`macos-latest` + `windows-latest` matrix using
[`tauri-apps/tauri-action`](https://github.com/tauri-apps/tauri-action).
It fires on any pushed tag matching `v*` (or manually via "Run workflow"),
and drafts a GitHub Release with both artifacts attached. It needs no
GitHub Container Registry or repo secrets beyond the default
`GITHUB_TOKEN`.

To cut a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## About the unsigned builds

These builds are **not code-signed or notarized** — that requires a
paid Apple Developer account and a Windows code-signing certificate,
neither of which this project provisions. That means:

**macOS (Gatekeeper):** double-clicking the `.dmg`'s app the first time
shows *"Job Tracker" can't be opened because Apple cannot check it for
malicious software* (or, on older macOS, *"is damaged and can't be
opened"* — same root cause, just a more alarming wording). To open it
anyway: right-click (or Control-click) the app → **Open** → **Open** in
the confirmation dialog. You only need to do this once. Alternatively,
after copying the app to `/Applications`, run:

```bash
xattr -cr "/Applications/Job Tracker.app"
```

**Windows (SmartScreen):** running the installer shows *"Windows
protected your PC" / "Microsoft Defender SmartScreen prevented an
unrecognized app from starting."* Click **More info**, then **Run
anyway**.

Neither warning means the app is unsafe — it's the standard OS response
to any installer that isn't signed with a paid certificate. If you want
to remove these warnings for your own distribution, you'll need an Apple
Developer Program membership (for `codesign`/`notarytool`) and a Windows
Authenticode certificate, then wire the relevant secrets into
`tauri.conf.json`'s `bundle.macOS`/`bundle.windows` signing fields and
the CI workflow's environment.

## Design notes / deviations

- The window is resizable (default 480×600, `minWidth`/`minHeight` also
  480×600) rather than fixed-size, so the Applications list is usable
  without paste-panel content getting clipped. It still opens at the
  spec'd 480×600 quick-capture size every time.
- Closing the window (⌘W / the titlebar close button) hides it instead
  of quitting — the app keeps running via the tray icon until you choose
  **Quit** from the tray menu.
- The list view re-reads the workbook only when you open the
  Applications tab — there's no polling or file-watching.
