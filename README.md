# Job Tracker

A native, installable desktop app for tracking job applications. Lives in
the tray/menu bar; a global hotkey pops open a small capture panel so you
can screenshot a job posting, paste it, and have Claude pull out the
fields for you — reviewed and saved straight into an Excel workbook.

- **Tauri v2** (Rust backend + the OS's own WebView — no bundled Chromium)
- **Vanilla HTML/CSS/JS** frontend — no React, no bundler, no `npm install`
- No background polling, no timers, no server process. The app is inert
  until you invoke it via the tray icon or the global hotkey.
- Ships as a macOS `.dmg` and a Windows `.msi`/`.exe`.

## How it works

1. Apply on LinkedIn (or anywhere), screenshot the page.
2. Press `Cmd+Shift+J` (macOS) / `Ctrl+Shift+J` (Windows) from anywhere.
3. Press `Cmd+V` / `Ctrl+V` in the capture panel.
4. Claude extracts company, position, location, work type, employment
   type, salary range, job ID, posted date, URL, and notes.
5. Review/correct the fields in the form, hit Enter to save.
6. The row is written into your `JobApplications.xlsx`.

No screenshot? Click **Skip screenshot** for a blank form — the app is
fully usable with zero API calls.

## Project layout

```
src-tauri/            Rust backend (the actual application logic)
  src/
    lib.rs            App setup: tray icon, global shortcut, window behavior
    commands.rs        All #[tauri::command]s exposed to the frontend
    extraction.rs      Anthropic API call + JSON parsing (with tests)
    excel.rs            xlsx read/write via calamine + rust_xlsxwriter (with tests)
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
(and gracefully failing to parse) extracted JSON, the Excel
read/write round-trip, atomic-write cleanup, and duplicate-key
matching (case-insensitive, trimmed).

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
