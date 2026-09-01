(() => {
  "use strict";

  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;
  const { getCurrentWindow } = window.__TAURI__.window;

  const isMac = navigator.userAgent.includes("Mac");

  const el = (id) => document.getElementById(id);

  const dom = {
    setupScreen: el("setup-screen"),
    setupApiKey: el("setup-api-key"),
    setupSave: el("setup-save"),
    setupSkip: el("setup-skip"),
    setupError: el("setup-error"),

    tabButtons: Array.from(document.querySelectorAll(".tab-btn")),
    tabPanels: Array.from(document.querySelectorAll(".tab-panel")),

    dropzone: el("capture-dropzone"),
    chooseFileBtn: el("choose-file-btn"),
    skipScreenshotBtn: el("skip-screenshot-btn"),
    thumbnailWrap: el("thumbnail-wrap"),
    thumbnail: el("thumbnail"),
    thumbnailStatus: el("thumbnail-status"),
    parseFailed: el("parse-failed"),
    parseFailedRaw: el("parse-failed-raw"),
    pasteHintKey: el("paste-hint-key"),

    form: el("application-form"),
    editingBanner: el("editing-banner"),
    discardEditBtn: el("discard-edit-btn"),
    formSaveBtn: el("form-save"),
    fCompany: el("f-company"),
    fPosition: el("f-position"),
    fLocation: el("f-location"),
    fWorkType: el("f-work-type"),
    fEmploymentType: el("f-employment-type"),
    fSalaryRange: el("f-salary-range"),
    fStatus: el("f-status"),
    fDateApplied: el("f-date-applied"),
    fJobId: el("f-job-id"),
    fUrl: el("f-url"),
    fNotes: el("f-notes"),
    formCancel: el("form-cancel"),
    saveError: el("save-error"),

    duplicateBanner: el("duplicate-banner"),
    duplicateStatus: el("duplicate-status"),
    duplicateSaveAnyway: el("duplicate-save-anyway"),
    duplicateUpdateStatus: el("duplicate-update-status"),

    searchBox: el("search-box"),
    listStatus: el("list-status"),
    listStats: el("list-stats"),
    exportCsvBtn: el("export-csv-btn"),
    tbody: el("applications-tbody"),

    calPrev: el("cal-prev"),
    calNext: el("cal-next"),
    calToday: el("cal-today"),
    calMonthLabel: el("cal-month-label"),
    calStats: el("cal-stats"),
    calWeekdays: el("cal-weekdays"),
    calDays: el("cal-days"),
    calGrid: el("cal-grid"),
    calYear: el("cal-year"),
    calYearScroll: el("cal-year-scroll"),
    calYearMonths: el("cal-year-months"),
    calYearGrid: el("cal-year-grid"),
    calViewButtons: Array.from(document.querySelectorAll(".cal-view-btn")),
    calDayDetail: el("cal-day-detail"),
    calUndated: el("cal-undated"),

    extractionMethodSelect: el("settings-extraction-method"),
    extractionMethodStatus: el("extraction-method-status"),

    apiKeyStatus: el("api-key-status"),
    settingsApiKey: el("settings-api-key"),
    settingsSaveKey: el("settings-save-key"),
    settingsDeleteKey: el("settings-delete-key"),
    settingsKeyMessage: el("settings-key-message"),
    settingsExcelPath: el("settings-excel-path"),
    settingsChoosePath: el("settings-choose-path"),
    settingsPathMessage: el("settings-path-message"),
    settingsHotkey: el("settings-hotkey"),
    settingsVersion: el("settings-version"),
    settingsUpdateCheck: el("settings-update-check"),
    settingsCheckUpdate: el("settings-check-update"),
    settingsUpdateMessage: el("settings-update-message"),

    settingsAutoInstall: el("settings-auto-install"),

    updateBanner: el("update-banner"),
    updateHeadline: el("update-headline"),
    updateNotes: el("update-notes"),
    updateProgress: el("update-progress"),
    updateProgressBar: el("update-progress-bar"),
    updateBannerActions: el("update-banner-actions"),
    updateInstall: el("update-install"),
    updateLater: el("update-later"),

    retryToast: el("retry-toast"),
    retryMessage: el("retry-message"),
    retryBtn: el("retry-btn"),
    retryDismiss: el("retry-dismiss"),
  };

  let allApplications = [];
  let pendingRetry = null; // { fn: () => Promise, label: string }
  let setupDismissedThisSession = false;
  let editingIndex = null; // set when editing an existing row from the list tab
  let currentImage = null; // { base64, mediaType } of the screenshot behind the current form, if any
  let currentExtractionMethod = "tesseract";

  let autoInstallUpdates = true;
  let updateInstallStarted = false; // guards against a second install kicking off
  let pendingUpdateVersion = null;   // version the banner is currently offering

  const cal = window.JobTrackerCalendar;
  const calTodayParts = cal.todayParts();
  let calCursor = { year: calTodayParts.year, month: calTodayParts.month }; // month on screen
  let calSelectedDay = null; // "YYYY-MM-DD" of the day whose entries are listed
  let calView = "month";     // "month" | "year"
  let calGroups = { byDate: Object.create(null), undated: [] };

  function todayIso() {
    const d = new Date();
    const pad = (n) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  }

  function showError(node, message) {
    node.textContent = message;
    node.hidden = false;
  }

  function hideError(node) {
    node.hidden = true;
    node.textContent = "";
  }

  function showRetryToast(message, retryFn) {
    pendingRetry = retryFn;
    dom.retryMessage.textContent = message;
    dom.retryToast.hidden = false;
  }

  function hideRetryToast() {
    pendingRetry = null;
    dom.retryToast.hidden = true;
  }

  dom.retryBtn.addEventListener("click", () => {
    const fn = pendingRetry;
    hideRetryToast();
    if (fn) fn();
  });
  dom.retryDismiss.addEventListener("click", hideRetryToast);

  // ---------- Keyboard hint labels ----------

  function applyPlatformHints() {
    const mod = isMac ? "Cmd" : "Ctrl";
    dom.pasteHintKey.textContent = mod;
    dom.settingsHotkey.textContent = `${mod}+Shift+J`;
  }

  // ---------- Tabs ----------

  function activateTab(name) {
    dom.tabButtons.forEach((btn) => btn.classList.toggle("active", btn.dataset.tab === name));
    dom.tabPanels.forEach((panel) => panel.classList.toggle("active", panel.id === `tab-${name}`));
    if (name === "list" || name === "calendar") {
      loadApplications();
    }
  }

  dom.tabButtons.forEach((btn) => {
    btn.addEventListener("click", () => activateTab(btn.dataset.tab));
  });

  // ---------- Status dropdown ----------

  async function populateStatusDropdown() {
    let statuses = ["Applied", "Interviewing", "Offered", "Rejected", "Ghosted", "Withdrawn"];
    try {
      statuses = await invoke("get_statuses");
    } catch (_) {
      // Fall back to the hardcoded list above.
    }
    dom.fStatus.innerHTML = "";
    for (const status of statuses) {
      const opt = document.createElement("option");
      opt.value = status;
      opt.textContent = status;
      dom.fStatus.appendChild(opt);
    }
    dom.fStatus.value = "Applied";
  }

  // ---------- First-run setup ----------

  async function checkFirstRunSetup() {
    const hasKey = await invoke("has_api_key");
    if (!hasKey && !setupDismissedThisSession) {
      dom.setupScreen.hidden = false;
    }
  }

  dom.setupSave.addEventListener("click", async () => {
    const key = dom.setupApiKey.value.trim();
    if (!key) {
      showError(dom.setupError, "Enter an API key, or choose Skip.");
      return;
    }
    try {
      await invoke("save_api_key", { key });
      hideError(dom.setupError);
      dom.setupScreen.hidden = true;
      refreshApiKeyStatus();
    } catch (e) {
      showError(dom.setupError, String(e));
    }
  });

  dom.setupSkip.addEventListener("click", () => {
    setupDismissedThisSession = true;
    dom.setupScreen.hidden = true;
  });

  // ---------- Capture: paste / drag-drop / file picker ----------

  function resetCaptureArea() {
    dom.thumbnailWrap.hidden = true;
    dom.thumbnail.src = "";
    dom.parseFailed.hidden = true;
    dom.parseFailedRaw.value = "";
    dom.form.hidden = true;
    dom.duplicateBanner.hidden = true;
    hideError(dom.saveError);
    dom.form.reset();
    dom.fStatus.value = "Applied";
    dom.fDateApplied.value = todayIso();
    currentImage = null;
    exitEditMode();
  }

  function exitEditMode() {
    editingIndex = null;
    dom.editingBanner.hidden = true;
    dom.formSaveBtn.textContent = "Save (Enter)";
  }

  function enterEditModeFor(index) {
    const app = allApplications[index];
    if (!app) return;
    resetCaptureArea();
    editingIndex = index;
    currentImage = null;
    dom.editingBanner.hidden = false;
    dom.formSaveBtn.textContent = "Update (Enter)";

    dom.fCompany.value = app.company || "";
    dom.fPosition.value = app.position || "";
    dom.fLocation.value = app.location || "";
    dom.fWorkType.value = app.work_type || "";
    dom.fEmploymentType.value = app.employment_type || "";
    dom.fSalaryRange.value = app.salary_range || "";
    dom.fStatus.value = app.status || "Applied";
    dom.fDateApplied.value = app.date_applied || todayIso();
    dom.fJobId.value = app.job_id || "";
    dom.fUrl.value = app.url || "";
    dom.fNotes.value = app.notes || "";

    dom.form.hidden = false;
    activateTab("capture");
    dom.fCompany.focus();
  }

  dom.discardEditBtn.addEventListener("click", () => {
    resetCaptureArea();
  });

  function showBlankForm() {
    dom.thumbnailWrap.hidden = true;
    dom.parseFailed.hidden = true;
    dom.form.hidden = false;
    dom.fCompany.focus();
  }

  function base64FromArrayBuffer(buffer) {
    const bytes = new Uint8Array(buffer);
    let binary = "";
    const chunkSize = 0x8000;
    for (let i = 0; i < bytes.length; i += chunkSize) {
      binary += String.fromCharCode.apply(null, bytes.subarray(i, i + chunkSize));
    }
    return btoa(binary);
  }

  async function processImage(base64, mediaType) {
    exitEditMode();
    currentImage = { base64, mediaType };
    dom.thumbnailWrap.hidden = false;
    dom.thumbnail.src = `data:${mediaType};base64,${base64}`;
    dom.thumbnailStatus.textContent = "Extracting details…";
    dom.parseFailed.hidden = true;
    dom.form.hidden = true;

    try {
      const result =
        currentExtractionMethod === "tesseract"
          ? await invoke("extract_with_local_ocr", { imageBase64: base64 })
          : await invoke("extract_from_image", { imageBase64: base64, mediaType });
      dom.thumbnailStatus.textContent = "Extracted - review and save below.";
      if (result.kind === "Parsed") {
        applyExtractedFields(result.fields);
      } else {
        dom.parseFailed.hidden = false;
        dom.parseFailedRaw.value = result.raw_text;
        applyExtractedFields({});
      }
      dom.form.hidden = false;
    } catch (e) {
      dom.thumbnailStatus.textContent = `Couldn't extract details: ${e}`;
      applyExtractedFields({});
      dom.form.hidden = false;
    }
  }

  function applyExtractedFields(fields) {
    dom.fCompany.value = fields.company || "";
    dom.fPosition.value = fields.position || "";
    dom.fLocation.value = fields.location || "";
    dom.fWorkType.value = fields.work_type || "";
    dom.fEmploymentType.value = fields.employment_type || "";
    dom.fSalaryRange.value = fields.salary_range || "";
    dom.fJobId.value = fields.job_id || "";
    dom.fUrl.value = fields.url || "";
    dom.fNotes.value = fields.notes || "";
    dom.fStatus.value = "Applied";
    dom.fDateApplied.value = todayIso();
  }

  dom.skipScreenshotBtn.addEventListener("click", () => {
    resetCaptureArea();
    showBlankForm();
  });

  dom.chooseFileBtn.addEventListener("click", async () => {
    try {
      const path = await invoke("pick_image_file");
      if (!path) return;
      const payload = await invoke("read_image_file", { path });
      await processImage(payload.base64, payload.media_type);
    } catch (e) {
      dom.thumbnailWrap.hidden = false;
      dom.thumbnailStatus.textContent = `Couldn't read that file: ${e}`;
    }
  });

  dom.dropzone.addEventListener("dragover", (e) => {
    e.preventDefault();
    dom.dropzone.classList.add("drag-over");
  });
  dom.dropzone.addEventListener("dragleave", () => {
    dom.dropzone.classList.remove("drag-over");
  });
  dom.dropzone.addEventListener("drop", async (e) => {
    e.preventDefault();
    dom.dropzone.classList.remove("drag-over");
    const file = e.dataTransfer.files && e.dataTransfer.files[0];
    if (!file || !file.type.startsWith("image/")) return;
    const buffer = await file.arrayBuffer();
    const base64 = base64FromArrayBuffer(buffer);
    await processImage(base64, file.type);
  });

  // Cmd/Ctrl+V pastes the OS clipboard image via the Rust clipboard
  // plugin, unless the user is typing into a text field (where normal
  // text paste should keep working).
  document.addEventListener("keydown", async (e) => {
    const mod = isMac ? e.metaKey : e.ctrlKey;
    if (!mod || e.key.toLowerCase() !== "v") return;

    const tag = document.activeElement && document.activeElement.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA") return;

    const captureTabActive = document.getElementById("tab-capture").classList.contains("active");
    if (!captureTabActive) return;

    e.preventDefault();
    try {
      const base64 = await invoke("read_clipboard_image");
      if (base64) {
        await processImage(base64, "image/png");
      } else {
        dom.thumbnailWrap.hidden = false;
        dom.thumbnailStatus.textContent = "No image found on the clipboard.";
      }
    } catch (err) {
      dom.thumbnailWrap.hidden = false;
      dom.thumbnailStatus.textContent = `Couldn't read the clipboard: ${err}`;
    }
  });

  // ---------- Save flow ----------

  function collectFormApplication() {
    const opt = (v) => (v && v.trim() ? v.trim() : null);
    return {
      date_applied: dom.fDateApplied.value || todayIso(),
      company: dom.fCompany.value.trim(),
      position: dom.fPosition.value.trim(),
      location: opt(dom.fLocation.value),
      work_type: opt(dom.fWorkType.value),
      employment_type: opt(dom.fEmploymentType.value),
      salary_range: opt(dom.fSalaryRange.value),
      status: dom.fStatus.value,
      last_updated: todayIso(),
      job_id: opt(dom.fJobId.value),
      url: opt(dom.fUrl.value),
      notes: opt(dom.fNotes.value),
    };
  }

  async function saveScreenshotIfPresent(application) {
    if (!currentImage) return;
    try {
      await invoke("save_screenshot", {
        company: application.company,
        position: application.position,
        dateApplied: application.date_applied,
        imageBase64: currentImage.base64,
        mediaType: currentImage.mediaType,
      });
    } catch (_) {
      // Best-effort convenience copy - never block a successful save on this.
    }
  }

  async function attemptUpdate() {
    hideError(dom.saveError);
    const application = collectFormApplication();
    try {
      await invoke("update_application_at_index", { index: editingIndex, application });
      allApplications[editingIndex] = application;
      resetCaptureArea();
      activateTab("capture");
      installDeferredUpdate();
    } catch (e) {
      showError(dom.saveError, String(e));
      showRetryToast("Update failed - your entry is still here.", () => attemptUpdate());
    }
  }

  async function attemptSave(force) {
    if (editingIndex !== null) {
      return attemptUpdate();
    }
    hideError(dom.saveError);
    const application = collectFormApplication();
    try {
      const result = await invoke("save_application", { application, force });
      if (result.outcome === "Saved") {
        dom.duplicateBanner.hidden = true;
        await saveScreenshotIfPresent(application);
        resetCaptureArea();
        activateTab("capture");
        installDeferredUpdate();
      } else if (result.outcome === "Duplicate") {
        dom.duplicateStatus.textContent = result.existing_status;
        dom.duplicateBanner.hidden = false;
      }
    } catch (e) {
      showError(dom.saveError, String(e));
      showRetryToast("Save failed - your entry is still here.", () => attemptSave(force));
    }
  }

  dom.form.addEventListener("submit", (e) => {
    e.preventDefault();
    attemptSave(false);
  });

  dom.duplicateSaveAnyway.addEventListener("click", () => attemptSave(true));

  dom.duplicateUpdateStatus.addEventListener("click", async () => {
    hideError(dom.saveError);
    const company = dom.fCompany.value.trim();
    const position = dom.fPosition.value.trim();
    const status = dom.fStatus.value;
    try {
      await invoke("update_existing_status", { company, position, status });
      dom.duplicateBanner.hidden = true;
      resetCaptureArea();
      activateTab("capture");
      installDeferredUpdate();
    } catch (e) {
      showError(dom.saveError, String(e));
      showRetryToast("Update failed - your entry is still here.", () =>
        dom.duplicateUpdateStatus.click()
      );
    }
  });

  dom.formCancel.addEventListener("click", () => {
    getCurrentWindow().hide();
  });

  // ---------- List tab ----------

  function escapeHtml(s) {
    const div = document.createElement("div");
    div.textContent = s == null ? "" : s;
    return div.innerHTML;
  }

  function renderStats() {
    if (allApplications.length === 0) {
      dom.listStats.textContent = "";
      return;
    }
    const counts = {};
    for (const app of allApplications) {
      counts[app.status] = (counts[app.status] || 0) + 1;
    }
    dom.listStats.textContent = Object.entries(counts)
      .map(([status, count]) => `${count} ${status}`)
      .join(" · ");
  }

  function renderApplicationsTable() {
    renderStats();
    const query = dom.searchBox.value.trim().toLowerCase();
    const rows = allApplications
      .map((app, index) => ({ app, index }))
      .filter(({ app }) => {
        if (!query) return true;
        return (
          (app.company || "").toLowerCase().includes(query) ||
          (app.position || "").toLowerCase().includes(query)
        );
      });

    if (rows.length === 0) {
      dom.tbody.innerHTML = "";
      dom.listStatus.textContent = allApplications.length === 0
        ? "No applications saved yet."
        : "No matches.";
      return;
    }
    dom.listStatus.textContent = `${rows.length} of ${allApplications.length} application${allApplications.length === 1 ? "" : "s"}`;

    dom.tbody.innerHTML = rows
      .map(({ app, index }) => {
        const url = app.url
          ? `<a href="#" data-url="${escapeHtml(app.url)}" class="row-url">link</a>`
          : "";
        return `<tr data-index="${index}" class="app-row">
          <td>${escapeHtml(app.date_applied)}</td>
          <td>${escapeHtml(app.company)}</td>
          <td>${escapeHtml(app.position)}</td>
          <td>${statusSelect(index, app.status)}</td>
          <td>${escapeHtml(app.location)}</td>
          <td>${escapeHtml(app.work_type)}</td>
          <td>${escapeHtml(app.salary_range)}</td>
          <td>${escapeHtml(app.last_updated)}</td>
          <td>${escapeHtml(app.job_id)}</td>
          <td>${url}</td>
          <td>${escapeHtml(app.notes)}</td>
        </tr>`;
      })
      .join("");
  }

  let statusOptionsCache = ["Applied", "Interviewing", "Offered", "Rejected", "Ghosted", "Withdrawn"];

  function statusSelect(index, current) {
    const options = statusOptionsCache
      .map((s) => `<option value="${s}" ${s === current ? "selected" : ""}>${s}</option>`)
      .join("");
    return `<select data-index="${index}" class="row-status">${options}</select>`;
  }

  dom.tbody.addEventListener("change", async (e) => {
    const target = e.target;
    if (!target.classList.contains("row-status")) return;
    const index = Number(target.dataset.index);
    const newStatus = target.value;
    const previous = allApplications[index].status;
    try {
      await invoke("update_status_at_index", { index, status: newStatus });
      allApplications[index].status = newStatus;
      allApplications[index].last_updated = todayIso();
      renderApplicationsTable();
    } catch (err) {
      target.value = previous;
      showRetryToast(`Couldn't update status: ${err}`, () => {
        target.value = newStatus;
        target.dispatchEvent(new Event("change"));
      });
    }
  });

  dom.tbody.addEventListener("click", async (e) => {
    if (e.target.classList.contains("row-url")) {
      e.preventDefault();
      // Handed to the OS browser rather than navigated to in the webview.
      // The Rust side allows only http/https - a row's URL came off a
      // screenshot or a spreadsheet cell, not from this app.
      try {
        await invoke("open_url", { url: e.target.dataset.url });
      } catch (err) {
        dom.listStatus.textContent = String(err);
      }
      return;
    }
    if (e.target.classList.contains("row-status")) return;
    const row = e.target.closest(".app-row");
    if (!row) return;
    enterEditModeFor(Number(row.dataset.index));
  });

  async function loadApplications() {
    dom.listStatus.textContent = "Loading…";
    try {
      statusOptionsCache = await invoke("get_statuses");
    } catch (_) {
      /* keep cached fallback */
    }
    try {
      allApplications = await invoke("list_applications");
      renderApplicationsTable();
      renderCalendar();
    } catch (e) {
      dom.listStatus.textContent = `Couldn't load applications: ${e}`;
    }
  }

  dom.searchBox.addEventListener("input", renderApplicationsTable);

  dom.exportCsvBtn.addEventListener("click", async () => {
    try {
      const path = await invoke("export_csv");
      dom.listStatus.textContent = `Exported to ${path}`;
    } catch (e) {
      dom.listStatus.textContent = `Couldn't export CSV: ${e}`;
    }
  });

  // ---------- Calendar tab ----------

  function renderWeekdayHeader() {
    if (dom.calWeekdays.childElementCount) return;
    dom.calWeekdays.innerHTML = cal.WEEKDAY_NAMES.map((d) => `<span>${d}</span>`).join("");
  }

  function renderCalendar() {
    calGroups = cal.groupByDate(allApplications);

    dom.calViewButtons.forEach((btn) =>
      btn.classList.toggle("active", btn.dataset.view === calView)
    );
    dom.calGrid.hidden = calView !== "month";
    dom.calYear.hidden = calView !== "year";

    if (calView === "year") {
      renderYearView();
    } else {
      renderMonthView();
    }

    renderUndatedNote();
    renderDayDetail();
  }

  function renderUndatedNote() {
    const { undated } = calGroups;
    if (undated.length > 0) {
      dom.calUndated.hidden = false;
      dom.calUndated.textContent = `${undated.length} application${undated.length === 1 ? " has an unreadable Date Applied and isn't" : "s have an unreadable Date Applied and aren't"} shown on the calendar.`;
    } else {
      dom.calUndated.hidden = true;
      dom.calUndated.textContent = "";
    }
  }

  function renderMonthView() {
    renderWeekdayHeader();
    const { byDate } = calGroups;
    const { year, month } = calCursor;

    dom.calMonthLabel.textContent = cal.monthLabel(year, month);

    const { weeks } = cal.monthGrid(year, month, byDate, calTodayParts);
    dom.calDays.innerHTML = weeks
      .flat()
      .map((cell) => {
        const classes = ["cal-day", `level-${cell.level}`];
        if (!cell.inMonth) classes.push("outside");
        if (cell.isToday) classes.push("today");
        if (cell.count > 0) classes.push("has-apps");
        if (cell.iso === calSelectedDay) classes.push("selected");
        const label = `${cal.dayLabel(cell.iso)}: ${cell.count} application${cell.count === 1 ? "" : "s"}`;
        return `<button type="button" class="${classes.join(" ")}" data-iso="${cell.iso}" title="${escapeHtml(label)}" aria-label="${escapeHtml(label)}">
          <span class="cal-day-number">${cell.day}</span>
          ${cell.count ? `<span class="cal-day-count">${cell.count}</span>` : ""}
        </button>`;
      })
      .join("");

    const stats = cal.monthStats(year, month, byDate);
    const streak = cal.currentStreak(byDate, calTodayParts);
    const parts = [
      `${stats.total} application${stats.total === 1 ? "" : "s"} this month`,
      `${stats.activeDays} active day${stats.activeDays === 1 ? "" : "s"}`,
    ];
    if (stats.busiest) {
      parts.push(`busiest ${cal.dayLabel(stats.busiest.iso)} (${stats.busiest.count})`);
    }
    if (streak > 0) {
      parts.push(`${streak}-day streak`);
    }
    dom.calStats.textContent = parts.join(" · ");
  }

  function renderYearView() {
    const { byDate } = calGroups;
    const { year } = calCursor;

    dom.calMonthLabel.textContent = String(year);

    const { columns, monthStarts } = cal.yearGrid(year, byDate, calTodayParts);

    // Month labels sit on the same column track as the grid below them.
    dom.calYearMonths.innerHTML = monthStarts
      .map(
        (m) =>
          `<span style="grid-column: ${m.column + 1} / span 4">${cal.MONTH_NAMES[m.month - 1].slice(0, 3)}</span>`
      )
      .join("");

    dom.calYearGrid.innerHTML = columns
      .flat()
      .map((cell) => {
        const classes = ["cal-year-day", `level-${cell.level}`];
        if (!cell.inYear) classes.push("outside");
        if (cell.isToday) classes.push("today");
        if (cell.count > 0) classes.push("has-apps");
        if (cell.iso === calSelectedDay) classes.push("selected");
        const label = `${cal.dayLabel(cell.iso)}: ${cell.count} application${cell.count === 1 ? "" : "s"}`;
        return `<button type="button" class="${classes.join(" ")}" data-iso="${cell.iso}" title="${escapeHtml(label)}" aria-label="${escapeHtml(label)}"></button>`;
      })
      .join("");

    scrollYearIntoView();

    const stats = cal.yearStats(year, byDate);
    const streak = cal.currentStreak(byDate, calTodayParts);
    const parts = [
      `${stats.total} application${stats.total === 1 ? "" : "s"} in ${year}`,
      `${stats.activeDays} active day${stats.activeDays === 1 ? "" : "s"}`,
    ];
    if (stats.busiest) {
      parts.push(`busiest ${cal.dayLabel(stats.busiest.iso)} (${stats.busiest.count})`);
    }
    if (streak > 0) {
      parts.push(`${streak}-day streak`);
    }
    dom.calStats.textContent = parts.join(" · ");
  }

  function renderDayDetail() {
    if (!calSelectedDay) {
      dom.calDayDetail.innerHTML =
        '<p class="hint">Click a day to see the applications you sent that day.</p>';
      return;
    }
    const entries = calGroups.byDate[calSelectedDay] || [];
    if (entries.length === 0) {
      dom.calDayDetail.innerHTML = `<h3>${escapeHtml(cal.dayLabel(calSelectedDay))}</h3>
        <p class="hint">No applications on this day.</p>`;
      return;
    }
    dom.calDayDetail.innerHTML =
      `<h3>${escapeHtml(cal.dayLabel(calSelectedDay))} - ${entries.length} application${entries.length === 1 ? "" : "s"}</h3>` +
      entries
        .map(
          ({ app, index }) => `<button type="button" class="cal-entry" data-index="${index}">
            <span class="cal-entry-company">${escapeHtml(app.company)}</span>
            <span class="cal-entry-position">${escapeHtml(app.position)}</span>
            <span class="cal-entry-status">${escapeHtml(app.status)}</span>
          </button>`
        )
        .join("");
  }

  dom.calDays.addEventListener("click", (e) => {
    const cell = e.target.closest(".cal-day");
    if (!cell) return;
    const iso = cell.dataset.iso;
    // Clicking a neighbouring month's day pages there rather than
    // selecting a day that isn't on screen.
    const parts = cal.parseDate(iso);
    if (parts && (parts.year !== calCursor.year || parts.month !== calCursor.month)) {
      calCursor = { year: parts.year, month: parts.month };
    }
    calSelectedDay = calSelectedDay === iso ? null : iso;
    renderCalendar();
  });

  /// 53 weeks don't fit the window, and the left edge (January) is usually
  /// the least interesting part. Centre on today, the selected day, or
  /// failing both the first day with anything on it.
  function scrollYearIntoView() {
    const scroller = dom.calYearScroll;
    if (!scroller.clientWidth) return; // laid out only once the tab is visible
    const target =
      dom.calYearGrid.querySelector(".cal-year-day.selected") ||
      dom.calYearGrid.querySelector(".cal-year-day.today") ||
      dom.calYearGrid.querySelector(".cal-year-day.has-apps");
    if (!target) {
      scroller.scrollLeft = 0;
      return;
    }
    const cell = target.getBoundingClientRect();
    const box = scroller.getBoundingClientRect();
    scroller.scrollLeft += cell.left - box.left - scroller.clientWidth / 2 + cell.width / 2;
  }

  dom.calYearGrid.addEventListener("click", (e) => {
    const cell = e.target.closest(".cal-year-day");
    if (!cell || cell.classList.contains("outside")) return;
    const parts = cal.parseDate(cell.dataset.iso);
    if (!parts) return;
    // A day in the year view is a way in to that month, not a destination.
    calCursor = { year: parts.year, month: parts.month };
    calSelectedDay = cell.dataset.iso;
    calView = "month";
    renderCalendar();
  });

  dom.calViewButtons.forEach((btn) => {
    btn.addEventListener("click", () => {
      if (calView === btn.dataset.view) return;
      calView = btn.dataset.view;
      renderCalendar();
    });
  });

  dom.calDayDetail.addEventListener("click", (e) => {
    const entry = e.target.closest(".cal-entry");
    if (!entry) return;
    enterEditModeFor(Number(entry.dataset.index));
  });

  /// The arrows step a month at a time in the month view and a year at a
  /// time in the year view, so one toolbar serves both.
  function stepCalendar(direction) {
    calCursor =
      calView === "year"
        ? { year: calCursor.year + direction, month: calCursor.month }
        : cal.shiftMonth(calCursor.year, calCursor.month, direction);
    calSelectedDay = null;
    renderCalendar();
  }

  dom.calPrev.addEventListener("click", () => stepCalendar(-1));
  dom.calNext.addEventListener("click", () => stepCalendar(1));

  dom.calToday.addEventListener("click", () => {
    calCursor = { year: calTodayParts.year, month: calTodayParts.month };
    calSelectedDay = cal.iso(calTodayParts.year, calTodayParts.month, calTodayParts.day);
    renderCalendar();
  });

  // ---------- Settings tab ----------

  async function refreshExtractionMethodStatus() {
    if (currentExtractionMethod === "tesseract") {
      const available = await invoke("local_ocr_available");
      dom.extractionMethodStatus.textContent = available
        ? "Tesseract detected - screenshots are read locally, for free."
        : "Tesseract not found. Install it with `brew install tesseract` (macOS), your package manager (Linux), or from github.com/tesseract-ocr/tesseract (Windows), or switch to Claude below.";
    } else {
      dom.extractionMethodStatus.textContent =
        "Uses the Anthropic API for higher-accuracy extraction - see the API key section below.";
    }
  }

  async function loadExtractionMethod() {
    try {
      currentExtractionMethod = await invoke("get_extraction_method");
    } catch (_) {
      currentExtractionMethod = "tesseract";
    }
    dom.extractionMethodSelect.value = currentExtractionMethod;
    await refreshExtractionMethodStatus();
  }

  dom.extractionMethodSelect.addEventListener("change", async () => {
    currentExtractionMethod = dom.extractionMethodSelect.value;
    try {
      await invoke("set_extraction_method", { method: currentExtractionMethod });
    } catch (_) {
      /* best effort - the in-memory value still takes effect this session */
    }
    await refreshExtractionMethodStatus();
  });

  async function refreshApiKeyStatus() {
    const hasKey = await invoke("has_api_key");
    dom.apiKeyStatus.textContent = hasKey
      ? "An API key is stored in your OS keychain."
      : "No API key stored - screenshot extraction is disabled until you add one.";
  }

  dom.settingsSaveKey.addEventListener("click", async () => {
    const key = dom.settingsApiKey.value.trim();
    if (!key) return;
    try {
      await invoke("save_api_key", { key });
      dom.settingsApiKey.value = "";
      dom.settingsKeyMessage.hidden = false;
      dom.settingsKeyMessage.textContent = "Saved.";
      refreshApiKeyStatus();
    } catch (e) {
      dom.settingsKeyMessage.hidden = false;
      dom.settingsKeyMessage.textContent = String(e);
    }
  });

  dom.settingsDeleteKey.addEventListener("click", async () => {
    try {
      await invoke("delete_api_key");
      dom.settingsKeyMessage.hidden = false;
      dom.settingsKeyMessage.textContent = "Removed.";
      refreshApiKeyStatus();
    } catch (e) {
      dom.settingsKeyMessage.hidden = false;
      dom.settingsKeyMessage.textContent = String(e);
    }
  });

  async function refreshExcelPath() {
    try {
      dom.settingsExcelPath.value = await invoke("get_excel_path");
    } catch (e) {
      dom.settingsExcelPath.value = "";
      dom.settingsPathMessage.hidden = false;
      dom.settingsPathMessage.textContent = String(e);
    }
  }

  dom.settingsChoosePath.addEventListener("click", async () => {
    try {
      const path = await invoke("pick_excel_path");
      if (!path) return;
      await invoke("set_excel_path", { path });
      dom.settingsExcelPath.value = path;
      dom.settingsPathMessage.hidden = false;
      dom.settingsPathMessage.textContent = "Saved.";
    } catch (e) {
      dom.settingsPathMessage.hidden = false;
      dom.settingsPathMessage.textContent = String(e);
    }
  });

  // ---------- Updates ----------

  function showSettingsUpdateMessage(message) {
    dom.settingsUpdateMessage.hidden = false;
    dom.settingsUpdateMessage.textContent = message;
  }

  /// True while the capture form holds something the user hasn't saved.
  /// Automatic installs hold off in that case rather than restarting out
  /// from under a half-filled entry.
  function hasUnsavedWork() {
    if (dom.form.hidden) return false;
    if (editingIndex !== null) return true;
    return Boolean(
      dom.fCompany.value.trim() ||
        dom.fPosition.value.trim() ||
        dom.fNotes.value.trim() ||
        dom.fUrl.value.trim()
    );
  }

  function showUpdateBanner(result, note) {
    pendingUpdateVersion = result.version;
    dom.updateHeadline.textContent = `Version ${result.version} is available.`;
    dom.updateNotes.textContent = note || (result.notes || "").trim();
    dom.updateNotes.hidden = !dom.updateNotes.textContent;
    dom.updateProgress.hidden = true;
    dom.updateBannerActions.hidden = false;
    dom.updateBanner.hidden = false;
  }

  function showInstallingBanner(version) {
    pendingUpdateVersion = version;
    dom.updateHeadline.textContent = `Installing version ${version}…`;
    dom.updateNotes.textContent = "The app will restart when it's done.";
    dom.updateNotes.hidden = false;
    dom.updateProgressBar.style.width = "0%";
    dom.updateProgressBar.classList.add("indeterminate");
    dom.updateProgress.hidden = false;
    dom.updateBannerActions.hidden = true;
    dom.updateBanner.hidden = false;
  }

  listen("update-download-progress", (event) => {
    const { downloaded, total } = event.payload || {};
    if (!total) return; // length unknown - leave the indeterminate sweep running
    const percent = Math.min(100, Math.round((downloaded / total) * 100));
    dom.updateProgressBar.classList.remove("indeterminate");
    dom.updateProgressBar.style.width = `${percent}%`;
    dom.updateNotes.textContent = `Downloaded ${percent}%.`;
  });

  listen("update-installing", () => {
    dom.updateProgressBar.classList.remove("indeterminate");
    dom.updateProgressBar.style.width = "100%";
    dom.updateNotes.textContent = "Installing - the app will restart in a moment.";
  });

  /// Downloads, installs, and restarts. Succeeds by never returning.
  async function runInstall(version) {
    if (updateInstallStarted) return;
    updateInstallStarted = true;
    showInstallingBanner(version);
    try {
      await invoke("install_update");
    } catch (e) {
      updateInstallStarted = false;
      dom.updateProgress.hidden = true;
      dom.updateProgressBar.classList.remove("indeterminate");
      dom.updateBannerActions.hidden = false;
      dom.updateHeadline.textContent = `Version ${version} couldn't be installed.`;
      dom.updateNotes.textContent = String(e);
      dom.updateNotes.hidden = false;
      showRetryToast(`Update failed: ${e}`, () => runInstall(version));
    }
  }

  /// Called after a successful save: an update that held off because the
  /// capture form had unsaved work in it can now go ahead.
  function installDeferredUpdate() {
    if (!autoInstallUpdates || updateInstallStarted) return;
    if (!pendingUpdateVersion || dom.updateBanner.hidden) return;
    if (hasUnsavedWork()) return;
    runInstall(pendingUpdateVersion);
  }

  /// `force` is what the Settings tab's "Check now" passes: it bypasses both
  /// the once-a-day throttle and the automatic-checks preference. The startup
  /// call leaves it false, so a user who turned checks off is never contacted.
  async function checkForUpdate(force) {
    const result = await invoke("check_for_update", { force });
    if (result.outcome !== "Available") return result;

    if (autoInstallUpdates && !hasUnsavedWork()) {
      runInstall(result.version);
    } else if (autoInstallUpdates) {
      showUpdateBanner(
        result,
        "Waiting to install until your open entry is saved - or install it now."
      );
    } else {
      showUpdateBanner(result);
    }
    return result;
  }

  dom.updateLater.addEventListener("click", () => {
    dom.updateBanner.hidden = true;
  });

  dom.updateInstall.addEventListener("click", () => {
    if (pendingUpdateVersion) runInstall(pendingUpdateVersion);
  });

  dom.settingsCheckUpdate.addEventListener("click", async () => {
    dom.settingsCheckUpdate.disabled = true;
    showSettingsUpdateMessage("Checking…");
    try {
      const result = await checkForUpdate(true);
      if (result.outcome === "Available") {
        showSettingsUpdateMessage(
          autoInstallUpdates && !hasUnsavedWork()
            ? `Version ${result.version} is installing now.`
            : `Version ${result.version} is available - see the banner at the top.`
        );
      } else if (result.outcome === "UpToDate") {
        showSettingsUpdateMessage("You're on the latest version.");
      } else {
        showSettingsUpdateMessage(result.reason);
      }
    } catch (e) {
      showSettingsUpdateMessage(String(e));
    } finally {
      dom.settingsCheckUpdate.disabled = false;
    }
  });

  dom.settingsUpdateCheck.addEventListener("change", async () => {
    try {
      await invoke("set_update_check_enabled", { enabled: dom.settingsUpdateCheck.checked });
    } catch (e) {
      showSettingsUpdateMessage(String(e));
    }
  });

  dom.settingsAutoInstall.addEventListener("change", async () => {
    autoInstallUpdates = dom.settingsAutoInstall.checked;
    try {
      await invoke("set_auto_install_updates", { enabled: autoInstallUpdates });
    } catch (e) {
      showSettingsUpdateMessage(String(e));
    }
  });

  async function loadUpdateSettings() {
    try {
      dom.settingsVersion.textContent = await invoke("get_app_version");
    } catch (_) {
      dom.settingsVersion.textContent = "unknown";
    }
    try {
      dom.settingsUpdateCheck.checked = await invoke("get_update_check_enabled");
    } catch (_) {
      dom.settingsUpdateCheck.checked = true;
    }
    try {
      autoInstallUpdates = await invoke("get_auto_install_updates");
    } catch (_) {
      autoInstallUpdates = true;
    }
    dom.settingsAutoInstall.checked = autoInstallUpdates;
  }

  // ---------- Global hotkey / Esc ----------

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      getCurrentWindow().hide();
    }
  });

  listen("capture-shortcut-triggered", () => {
    activateTab("capture");
    if (!dom.form.hidden) return;
    dom.dropzone.focus();
  });

  // ---------- Init ----------

  async function init() {
    applyPlatformHints();
    resetCaptureArea();
    await populateStatusDropdown();
    await loadExtractionMethod();
    await checkFirstRunSetup();
    await refreshApiKeyStatus();
    await refreshExcelPath();
    await loadUpdateSettings();
    // One check on startup, throttled and opt-out-able in Rust. A failure
    // here (offline, endpoint down) must never block using the app.
    checkForUpdate(false).catch(() => {});
  }

  init();
})();
