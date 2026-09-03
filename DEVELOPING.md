# Developing Job Tracker

Everything a maintainer needs. The [README](README.md) is for people who
just want to use the app.

See also [CLAUDE.md](CLAUDE.md), which records the constraints that are
deliberate and the traps that have actually cost time here.

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
    stats.js              Status breakdown, response rate, donut arcs (Applications tab)
    format.js             HTML/attribute escaping for everything rendered
CLAUDE.md               Repo conventions and traps, for anyone (or anything) editing it
tests/                  Browser-run frontend tests (no npm, no runner)
  calendar.test.html     Pure-logic tests for src/calendar.js
  stats.test.html        Pure-logic tests for src/stats.js
  format.test.html       Escaping tests, including the attribute-injection shapes
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
- **Tesseract** (optional). Windows and macOS default to the OCR engine
  the OS already ships, so you only need Tesseract to work on that code
  path, or on Linux. Without it the app builds and runs fine; the
  Tesseract method just reports that it is missing.

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
  `PASS n/n` or `FAIL n/n`, with a per-test list.
- <http://localhost:8000/tests/stats.test.html> — the status breakdown,
  the response rate (silence vs. a reply, with withdrawn applications
  excluded from both sides), and the donut geometry.
  Both of the above **run in CI** in headless Chrome, so a red result
  fails the build.
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

### Regenerating the keypair (only if it is lost or compromised)

Updates are signed, so the app will only install a build that came from
the project's private key. **This is configured** — the public key is in
`tauri.conf.json` and CI holds the private half as a repository secret.

A fork won't have those secrets, and nothing breaks there: releases build
without updater artifacts and the app reports "no update signing key
configured" instead of failing.

Replacing the key means every installed copy stops accepting updates —
they verify against the public key baked into the build they are running —
so those users have to reinstall by hand once. Only do this if the private
key is lost or exposed.

1. Generate a keypair. Either the Tauri CLI, or minisign, which needs no
   Rust toolchain:

   ```bash
   minisign -G -p ~/.tauri/snaptrack.key.pub -s ~/.tauri/snaptrack.key
   ```

   **Give it a password.** A passwordless (`-W`) key is not a working
   shortcut: macOS built with one fine and Windows failed with `failed to
   decode secret key: incorrect updater private key password`, both with
   and without `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` set to an empty
   string. minisign will take the password on stdin if you need this
   scripted.

   On Windows, normalise both files to LF line endings afterwards —
   minisign writes CRLF, which the parser Tauri uses does not strip.

2. Put **base64 of the whole `.pub` file** into `src-tauri/tauri.conf.json`
   as `plugins.updater.pubkey` — not the key line on its own. Tauri
   base64-decodes that value and hands the result to minisign's parser,
   which wants the full two-line file. This one is meant to be committed.

3. Add two repository secrets (Settings → Secrets and variables →
   Actions) so CI can sign each release:

   | Secret | Value |
   | --- | --- |
   | `TAURI_SIGNING_PRIVATE_KEY` | **base64 of** `~/.tauri/snaptrack.key` |
   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | only for a passworded key — see below |

   With a passwordless (`-W`) key, leave that second secret **unset and
   absent from the workflow's `env:`**. An empty value is not the same as
   no value: macOS accepted it and Windows failed the build with "Wrong
   password for that key".

   ```bash
   base64 -w0 ~/.tauri/snaptrack.key | gh secret set TAURI_SIGNING_PRIVATE_KEY
   ```

   Never commit the private key itself, and keep a backup of it somewhere
   safe — losing it is what forces every user to reinstall.

4. Tag and push a release as usual. The workflow bundles the installers,
   signs them, and attaches a `latest.json` manifest — which is what the
   endpoint in `tauri.conf.json` points at:

   ```
   https://github.com/Carkappa/Snaptrack/releases/latest/download/latest.json
   ```

**Tagging is the whole process.** The workflow builds the installers,
attaches them, and publishes the release. Don't create a release from the
tag page yourself - that makes a second, empty release on the same tag and
leaves the built installers behind.

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
