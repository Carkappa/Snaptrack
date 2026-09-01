# Job Tracker

A native, installable desktop app for tracking job applications. Lives in
the tray/menu bar; a global hotkey pops open a small capture panel so you
can screenshot a job posting, paste it, and have the fields pulled out
for you — reviewed and saved straight into an Excel workbook.

- **Tauri v2** (Rust backend + the OS's own WebView — no bundled Chromium)
- **Vanilla HTML/CSS/JS** frontend — no React, no bundler, no `npm install`
- No background polling, no timers, no server process. The app is inert
  until you invoke it via the tray icon or the global hotkey.
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

## Project layout

```
src-tauri/            Rust backend (the actual application logic)
  src/
    lib.rs            App setup: tray icon, global shortcut, window behavior
    commands.rs        All #[tauri::command]s exposed to the frontend
    extraction.rs      Anthropic API call + JSON parsing (with tests)
    local_ocr.rs         Tesseract-backed extraction + layout heuristics (with tests)
    excel.rs            xlsx read/write, CSV export, timestamped backups (with tests)
    keychain.rs         API key storage via the keyring crate
    models.rs            Shared data types
  capabilities/         Tauri v2 permission grants for the main window
  icons/                 App icons (.icns / .ico / .png)
  tauri.conf.json         Window, bundle, and tray configuration
src/                    Frontend — plain index.html + styles.css + app.js
.github/workflows/      CI that builds the macOS + Windows installers
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

Covers: stripping ```` ``` ```` fences from Claude's response, parsing
(and gracefully failing to parse) extracted JSON, the Tesseract
layout-heuristic field guesser (largest text = title, line above it =
company, regex-based location/salary/job-ID/date detection), the Excel
read/write round-trip, CSV export, timestamped backups and pruning,
and duplicate-key matching (case-insensitive, trimmed). A separate
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
- **CSV export.** One click in the Applications tab writes a sibling
  `JobApplications.csv` next to the workbook.
- **Automatic backups.** Every save copies the previous workbook into a
  `backups/` folder next to it first (timestamped, capped at the last
  10), so a bad edit never loses prior data.
- **Screenshot archive.** When a save came from a screenshot, a copy is
  kept in `JobApplications_screenshots/` next to the workbook, named by
  date/company/position — handy if a listing gets taken down later.
- **Status counts.** A small `12 Applied · 3 Interviewing · ...` line
  above the Applications list, computed from what's already loaded.

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
