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
python3 -m http.server 8000         # then open the pages below
```

- `tests/calendar.test.html`, `tests/stats.test.html`, `tests/format.test.html`
  — pure-logic tests for the matching `src/` module. Page title reads
  `PASS n/n`. **All run in CI** in headless Chrome.
- `tests/ui-harness.html` — the real `index.html` and `app.js` with
  `window.__TAURI__` mocked. Every tab, form, and flow is clickable without
  building the app. Drive it from the browser console or a browser tool;
  `window.__harness` exposes the fixture rows, recorded calls, failure flags
  (`installUpdateFails`, `openUrlFails`), and `emit()` for Rust-side events.
  Not automated — it is a manual tool.

When you touch `calendar.js` or `stats.js`, add to the matching test page
(and to the `for page in ...` loop in the workflow if you add another). When
you touch a command, prefer a unit test next to it, or
`src-tauri/tests/full_pipeline.rs` if it needs a real `AppHandle` and workbook.

## Look and feel

Two references, deliberately. Apple's system design for the chrome — system
accent blue, hairline separators, segmented controls, grouped inset cards,
iOS switches. And [exelban/stats][stats] for information display: a hero
figure, faint centred section labels, and dense rows of colour chip ->
label -> meter -> value. `.panel`, `.section-label`, `.detail-row` and
`.meter` in `styles.css` are that vocabulary — reuse them rather than
inventing a third style.

Status colours live in one place (the `--st-*` tokens) and are keyed off
`stats.js`'s `STATUS_ORDER`, so the chips, meters and donut cannot disagree
about what a status looks like.

[stats]: https://github.com/exelban/stats

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
- **Never create a release from the tag page.** The workflow publishes the
  release itself. Making one by hand from a tag creates a *second*, empty
  release on that tag - GitHub allows it - and the installers CI built stay
  attached to the other one.
- **The macOS updater needs the `app` bundle target.** Without it,
  `latest.json` carries only Windows entries and a Mac never updates, even
  though the `.dmg` builds fine.
- **The signing key must have a password.** A passwordless (`-W`) minisign
  key builds on macOS and fails on Windows with "Wrong password for that
  key", with and without `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` set to empty.
- **`--config` in the release workflow resolves from the repo root**, not
  from `src-tauri`.
- **Updates were unsigned until a key existed.** `tauri.conf.json` carries a
  placeholder `pubkey`; `updates.rs` detects it and reports that plainly
  instead of failing with a signature error. See the README for the setup.
- **Escape through `format.js`, never `textContent`/`innerHTML`.** That trick
  leaves quotes unescaped, which is fine between tags and an attribute
  injection inside `attr="..."`. Every value rendered here came off a
  screenshot, an OCR pass, or a spreadsheet cell.
- **Dates in the workbook may not be text.** The app writes `YYYY-MM-DD`
  strings, but a user can format the column as a Date in Excel, and calamine
  then hands back a serial number. `cell_to_date_string` in `excel.rs` is the
  only correct way to read the two date columns.
- **Browsers cache the test pages hard.** All three carry a `no-store` meta
  and the harness cache-busts its own sub-resources. Without that, an edit to
  `app.js` or `styles.css` shows up as a phantom bug in the code you just
  changed — this cost real debugging time twice.
- **Don't use `cargo test --verbose` in CI.** It prints a full rustc
  invocation per crate and buries the actual error.
