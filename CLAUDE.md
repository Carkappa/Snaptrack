# Working in this repo

A Tauri v2 desktop app for tracking job applications. Rust backend, vanilla
frontend, an Excel workbook as the datastore. See `README.md` for what it
does and how to install it; this file is about how to change it.

## The constraints that shape everything

These are deliberate. Check before breaking one.

- **No npm, no bundler, no build step for the frontend.** `src/` is plain
  `index.html` + `styles.css` + `app.js` + `calendar.js`, loaded as-is by
  the webview. `tauri.conf.json` points `frontendDist` straight at `src/`.
  Adding a framework or a bundler is a large change, not a small one.
- **No background work.** No polling, no timers, no server process. The app
  is inert until the tray icon or the global hotkey wakes it. The one
  network call that isn't user-initiated is a single update check when the
  window initialises, throttled to once a day and switchable off.
- **The frontend talks to Rust only through `invoke`.** No plugin JS
  packages — where a Tauri plugin is needed, it is used from Rust behind a
  `#[tauri::command]`. That is why `window.__TAURI__` can be mocked wholesale
  in the test harness.
- **The workbook is a file the user also owns.** They can have it open in
  Excel, hand-edit it, or re-sort it while the app is running. Reads are
  tolerant (several date formats, blank rows skipped) and destructive writes
  verify what they are about to touch rather than trusting an index.

## Testing

There is no cargo or Node in some environments this gets worked on from.
**CI is the verification of record** — `.github/workflows/test.yml` runs both
suites on every push, and both must be green.

```bash
cd src-tauri && cargo test          # Rust: unit + integration
python3 -m http.server 8000         # then open the two pages below
```

- `tests/calendar.test.html` — pure-logic tests for `src/calendar.js`. Page
  title reads `PASS n/n`. **This runs in CI** in headless Chrome.
- `tests/ui-harness.html` — the real `index.html` and `app.js` with
  `window.__TAURI__` mocked. Every tab, form, and flow is clickable without
  building the app. Drive it from the browser console or a browser tool;
  `window.__harness` exposes the fixture rows, recorded calls, failure flags
  (`installUpdateFails`, `openUrlFails`), and `emit()` for Rust-side events.
  Not automated — it is a manual tool.

When you touch `calendar.js`, add to `calendar.test.html`. When you touch a
command, prefer a unit test next to it, or `src-tauri/tests/full_pipeline.rs`
if it needs a real `AppHandle` and workbook.

## Things that will bite you

- **`[hidden]` needs the explicit rule in `styles.css`.** The UA's
  `[hidden] { display: none }` loses to any author rule setting `display`,
  and several panels set one. Removing `[hidden] { display: none !important }`
  makes the setup overlay cover the app permanently.
- **Row indices are workbook positions.** The applications list sorts and
  filters for display only; `data-index` stays the index into
  `allApplications`, which is what every write command addresses. Do not
  renumber it.
- **Bump the version with `./scripts/set-version.sh`.** `Cargo.toml` is the
  source of truth and `tauri.conf.json` has no `version` field. A release
  whose version is not newer than what is installed is silently never offered
  as an update; the release workflow guards the tag against the crate version.
- **Releases are drafts.** GitHub does not serve draft assets, so the updater
  sees nothing until the release is published by hand.
- **Updates are unsigned until a key exists.** `tauri.conf.json` carries a
  placeholder `pubkey`; `updates.rs` detects it and reports that plainly
  instead of failing with a signature error. See the README for the setup.
- **Don't use `cargo test --verbose` in CI.** It prints a full rustc
  invocation per crate and buries the actual error.
