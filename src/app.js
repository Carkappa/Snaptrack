(() => {
  "use strict";

  const { invoke } = window.__TAURI__.core;
  const { listen } = window.__TAURI__.event;
  const { getCurrentWindow } = window.__TAURI__.window;

  const isMac = navigator.userAgent.includes("Mac");

  const el = (id) => document.getElementById(id);

  const dom = {
    setupScreen: el("setup-screen"),
    setupStart: el("setup-start"),
    setupSettings: el("setup-settings"),
    setupHotkeyKeys: el("setup-hotkey-keys"),

    tabButtons: Array.from(document.querySelectorAll(".tab-btn")),
    tabPanels: Array.from(document.querySelectorAll(".tab-panel")),

    dropzone: el("capture-dropzone"),
    chooseFileBtn: el("choose-file-btn"),
    skipScreenshotBtn: el("skip-screenshot-btn"),
    thumbnailWrap: el("thumbnail-wrap"),
    thumbnail: el("thumbnail"),
    thumbnailStatus: el("thumbnail-status"),
    ocrBlocksPanel: el("ocr-blocks-panel"),
    ocrBlocks: el("ocr-blocks"),
    ocrTargetLabel: el("ocr-target-label"),
    parseFailed: el("parse-failed"),
    parseFailedRaw: el("parse-failed-raw"),
    pasteHintKey: el("paste-hint-key"),

    form: el("application-form"),
    editingBanner: el("editing-banner"),
    screenshotRow: el("screenshot-row"),
    openScreenshotBtn: el("open-screenshot-btn"),
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
    exportCsvBtn: el("export-csv-btn"),
    tbody: el("applications-tbody"),
    sortHeaders: Array.from(document.querySelectorAll("#applications-table th.sortable")),
    summaryPanel: el("summary-panel"),
    summaryToggle: el("summary-toggle"),
    summaryDonutArcs: el("summary-donut-arcs"),
    summaryTotal: el("summary-total"),
    summaryResponseRate: el("summary-response-rate"),
    summaryResponded: el("summary-responded"),
    summaryWaiting: el("summary-waiting"),
    summaryRows: el("summary-rows"),
    filterChip: el("filter-chip"),
    filterChipDot: el("filter-chip-dot"),
    filterChipLabel: el("filter-chip-label"),
    filterChipClear: el("filter-chip-clear"),

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

    fallbackList: el("fallback-list"),
    fallbackAdd: el("fallback-add"),
    fallbackAddBtn: el("fallback-add-btn"),
    ollamaGroup: el("ollama-group"),
    ollamaStatus: el("ollama-status"),
    ollamaModelRow: el("ollama-model-row"),
    masterResume: el("master-resume"),
    masterResumeImport: el("master-resume-import"),
    masterResumeSave: el("master-resume-save"),
    masterResumeMessage: el("master-resume-message"),
    resumeJobPicker: el("resume-job-picker"),
    resumeJobText: el("resume-job-text"),
    resumeTailor: el("resume-tailor"),
    resumeTailorMessage: el("resume-tailor-message"),
    resumeResultGroup: el("resume-result-group"),
    resumeResult: el("resume-result"),
    resumeSaveFile: el("resume-save-file"),
    resumeOpenFile: el("resume-open-file"),
    resumeSaveMessage: el("resume-save-message"),
    providerCards: el("provider-cards"),
    settingsOllamaModel: el("settings-ollama-model"),
    ollamaPullRow: el("ollama-pull-row"),
    ollamaPull: el("ollama-pull"),
    ollamaPullProgress: el("ollama-pull-progress"),
    ollamaPullBar: el("ollama-pull-bar"),
    ollamaPullStatus: el("ollama-pull-status"),
    ollamaModelDetail: el("ollama-model-detail"),
    settingsOllamaUnload: el("settings-ollama-unload"),
    settingsOllamaHost: el("settings-ollama-host"),
    settingsOllamaSave: el("settings-ollama-save"),
    settingsOllamaReset: el("settings-ollama-reset"),
    settingsExcelPath: el("settings-excel-path"),
    settingsChoosePath: el("settings-choose-path"),
    settingsPathMessage: el("settings-path-message"),
    settingsHotkey: el("settings-hotkey"),
    settingsHotkeyRecord: el("settings-hotkey-record"),
    settingsHotkeyReset: el("settings-hotkey-reset"),
    settingsHotkeyMessage: el("settings-hotkey-message"),
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

    undoToast: el("undo-toast"),
    undoMessage: el("undo-message"),
    undoBtn: el("undo-btn"),
    undoDismiss: el("undo-dismiss"),

    importBtn: el("settings-import"),
    importMessage: el("settings-import-message"),
    statusList: el("settings-status-list"),
    statusAdd: el("settings-status-add"),
    statusMessage: el("settings-status-message"),

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
  // The OCR text blocks behind the form, what the click-to-fill list is
  // built from and what a correction is matched against when saving.
  let currentOcrBlocks = [];
  let currentOcrSite = null;
  let lastFocusedField = null;

  let autoInstallUpdates = true;
  let updateInstallStarted = false; // guards against a second install kicking off
  let pendingUpdateVersion = null;   // version the banner is currently offering

  const statsLib = window.JobTrackerStats;
  const cal = window.JobTrackerCalendar;
  const calTodayParts = cal.todayParts();
  let calCursor = { year: calTodayParts.year, month: calTodayParts.month }; // month on screen
  let calSelectedDay = null; // "YYYY-MM-DD" of the day whose entries are listed
  let calView = "month";     // "month" | "year"

  // Applications list ordering. Newest first by default: the workbook is
  // append-ordered, which puts the row you just saved at the bottom.
  let listSort = { key: "date_applied", direction: "desc" };
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
    dom.pasteHintKey.textContent = isMac ? "Cmd" : "Ctrl";
  }

  // ---------- Tabs ----------

  function activateTab(name) {
    dom.tabButtons.forEach((btn) => {
      const selected = btn.dataset.tab === name;
      btn.classList.toggle("active", selected);
      btn.setAttribute("aria-selected", String(selected));
    });
    dom.tabPanels.forEach((panel) => panel.classList.toggle("active", panel.id === `tab-${name}`));
    if (name === "list" || name === "calendar") {
      loadApplications();
    }
    if (name === "resume") {
      loadResumeTab();
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

  // ---------- First-run welcome ----------

  /// Shown once, on the first launch, because the window opens itself then -
  /// a tray-only app that starts invisible looks like it failed to launch,
  /// and the shortcut is not guessable.
  async function checkFirstRunSetup() {
    let seen = true;
    try {
      seen = await invoke("get_seen_welcome");
    } catch (_) {
      seen = true; // never trap someone behind a broken store read
    }
    if (!seen && !setupDismissedThisSession) {
      dom.setupScreen.hidden = false;
    }
  }

  async function dismissWelcome() {
    setupDismissedThisSession = true;
    dom.setupScreen.hidden = true;
    try {
      await invoke("set_seen_welcome");
    } catch (_) {
      /* it will simply show again next launch */
    }
  }

  dom.setupStart.addEventListener("click", dismissWelcome);

  dom.setupSettings.addEventListener("click", async () => {
    await dismissWelcome();
    activateTab("settings");
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
    currentOcrBlocks = [];
    currentOcrSite = null;
    renderOcrBlocks();
    exitEditMode();
  }

  function exitEditMode() {
    editingIndex = null;
    dom.editingBanner.hidden = true;
    dom.screenshotRow.hidden = true;
    dom.formSaveBtn.textContent = "Save (Enter)";
  }

  /// The capture a row came from is archived next to the workbook but was
  /// never reachable from the app. Offered only when one actually exists -
  /// rows typed by hand or imported have none, and that is not a fault.
  async function showScreenshotIfArchived(app) {
    dom.screenshotRow.hidden = true;
    try {
      const path = await invoke("screenshot_for_application", {
        company: app.company,
        position: app.position,
        dateApplied: app.date_applied,
      });
      if (path) dom.screenshotRow.hidden = false;
    } catch (_) {
      /* no screenshot, or the workbook path is unset - stay hidden */
    }
  }

  dom.openScreenshotBtn.addEventListener("click", async () => {
    const app = allApplications[editingIndex];
    if (!app) return;
    try {
      await invoke("open_screenshot", {
        company: app.company,
        position: app.position,
        dateApplied: app.date_applied,
      });
    } catch (e) {
      showRetryToast(String(e), () => dom.openScreenshotBtn.click());
    }
  });

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
    showScreenshotIfArchived(app);
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
    dom.thumbnailStatus.textContent = "Extracting details\u2026";
    dom.thumbnailStatus.classList.remove("multiline");
    dom.thumbnailStatus.classList.remove("multiline");
    dom.parseFailed.hidden = true;
    dom.form.hidden = true;

    try {
      // One call: Rust picks the method, and falls through to the next
      // choice if it fails. Which one answered comes back with the result.
      const result = await invoke("extract_with_chain", {
        imageBase64: base64,
        mediaType,
      });
      dom.thumbnailStatus.textContent = describeExtraction(result);
      // Only the local OCR path knows where the text sat on the page.
      currentOcrBlocks = Array.isArray(result.blocks) ? result.blocks : [];
      currentOcrSite = result.site || null;
      renderOcrBlocks();
      if (result.kind === "Parsed") {
        applyExtractedFields(result.fields);
      } else {
        dom.parseFailed.hidden = false;
        dom.parseFailedRaw.value = result.raw_text;
        applyExtractedFields({});
      }
      dom.form.hidden = false;
    } catch (e) {
      // Rust returns one line per method that failed, each with its own
      // reason - stacked on one line they are unreadable.
      dom.thumbnailStatus.textContent = `Couldn't extract details.
${e}`;
      dom.thumbnailStatus.classList.add("multiline");
      applyExtractedFields({});
      dom.form.hidden = false;
    }
  }

  /// Says what read the page, and admits when the first choice did not.
  function describeExtraction(result) {
    const label = (id) => {
      const p = providers.find((x) => x.id === id);
      return p ? p.label : id;
    };
    const fellBack = (result.fell_back_from || []).length > 0;
    if (!fellBack) return "Extracted - review and save below.";
    return `${label(result.fell_back_from[0])} failed, so ${label(result.used)} read it instead - review and save below.`;
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
    if (recordingHotkey) return;
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

  // ---------- Click a block to fill a field ----------

  /// Form fields a block can be dropped into. Status and date are excluded:
  /// neither is ever a lump of text off the page.
  const FILLABLE = {
    "f-company": "Company",
    "f-position": "Position",
    "f-location": "Location",
    "f-work-type": "Work type",
    "f-employment-type": "Employment type",
    "f-salary-range": "Salary range",
    "f-job-id": "Job ID",
    "f-url": "URL",
    "f-notes": "Notes",
  };

  // Tracked on focusin rather than read at click time: clicking a block
  // has already moved focus off the field by then.
  dom.form.addEventListener("focusin", (e) => {
    if (FILLABLE[e.target.id]) {
      lastFocusedField = e.target.id;
      dom.ocrTargetLabel.textContent = FILLABLE[e.target.id];
    }
  });

  function renderOcrBlocks() {
    if (currentOcrBlocks.length === 0) {
      dom.ocrBlocksPanel.hidden = true;
      dom.ocrBlocks.innerHTML = "";
      return;
    }
    dom.ocrBlocksPanel.hidden = false;
    const target = resolveTargetField();
    dom.ocrTargetLabel.textContent = target ? FILLABLE[target.id] : "the focused field";
    dom.ocrBlocks.innerHTML = currentOcrBlocks
      .map(
        (text, i) =>
          `<button type="button" class="ocr-block" data-block="${i}">${escapeHtml(text)}</button>`
      )
      .join("");
  }

  /// Which field a block should land in.
  ///
  /// `focusin` is the main signal, but it is not the only way a field ends
  /// up focused, and before the first click there is no signal at all -
  /// where a block click would otherwise do nothing. Falls back to the
  /// first empty field, which on a fresh extraction is the one that needs
  /// filling.
  function resolveTargetField() {
    const active = document.activeElement;
    if (active && FILLABLE[active.id]) return active;
    if (lastFocusedField) return el(lastFocusedField);
    const firstEmpty = Object.keys(FILLABLE).find((id) => {
      const node = el(id);
      return node && !node.value.trim();
    });
    return firstEmpty ? el(firstEmpty) : null;
  }

  dom.ocrBlocks.addEventListener("click", (e) => {
    const btn = e.target.closest(".ocr-block");
    if (!btn) return;
    const text = currentOcrBlocks[Number(btn.dataset.block)];
    if (text == null) return;

    const target = resolveTargetField();
    if (!target) {
      dom.ocrTargetLabel.textContent = "a field - click one first";
      return;
    }
    lastFocusedField = target.id;
    dom.ocrTargetLabel.textContent = FILLABLE[target.id];
    target.value = text;
    target.dispatchEvent(new Event("input", { bubbles: true }));
    btn.classList.add("just-used");
    setTimeout(() => btn.classList.remove("just-used"), 600);
    target.focus();
  });

  /// Tells the backend where the kept values actually sat, so the next
  /// capture from this board starts from the corrected layout. Best-effort:
  /// a save must never fail because learning did.
  async function learnFromSave(application) {
    if (!currentOcrSite || currentOcrBlocks.length === 0) return;
    const saved = ["company", "position", "location"]
      .map((f) => [f, application[f]])
      .filter(([, v]) => v && String(v).trim());
    if (saved.length === 0) return;
    try {
      await invoke("learn_ocr_hints", {
        site: currentOcrSite,
        blocks: currentOcrBlocks,
        saved,
      });
    } catch (_) {
      /* never block a save on this */
    }
  }

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
      // Not a form field. An edit rebuilds the whole row, so without this
      // the tailored resume a row points at is dropped the first time
      // anyone corrects a typo in it.
      resume: editingIndex !== null ? allApplications[editingIndex].resume || null : null,
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
        await learnFromSave(application);
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

  const { escapeHtml } = window.JobTrackerFormat;

  let statusFilter = null;   // status name the list is narrowed to, or null
  let statusDefs = statsLib.DEFAULT_STATUS_DEFS; // replaced from the backend on init
  let lastDeleted = null;    // { application, index } for one level of undo

  /// Colours for statuses beyond the built-in six, assigned by position so
  /// two custom statuses never share one. `st-other` is the last resort.
  const EXTRA_STATUS_CLASSES = ["st-x1", "st-x2", "st-x3", "st-x4", "st-x5"];

  /// Shows which status the list is narrowed to, with a way out. Sits next
  /// to the count so it is impossible to forget a filter is on.
  function renderFilterChip() {
    if (!statusFilter) {
      dom.filterChip.hidden = true;
      return;
    }
    dom.filterChip.hidden = false;
    dom.filterChipLabel.textContent = statusFilter;
    dom.filterChipDot.className = `detail-chip ${statusClass(statusFilter)}`;
  }

  function setStatusFilter(status) {
    statusFilter = statusFilter === status ? null : status;
    renderApplicationsTable();
  }

  /// Class carrying a status's colour. Unknown statuses - someone typed into
  /// the Status cell - share one fallback colour rather than going invisible.
  function statusClass(status) {
    const builtIn = statsLib.namesOf(statsLib.DEFAULT_STATUS_DEFS);
    if (builtIn.includes(status)) return `st-${status}`;
    // Position among the *custom* statuses, so two of them can't land on the
    // same colour while the built-in slots sit unused.
    const customs = statusOptionsCache.filter((name) => !builtIn.includes(name));
    const index = customs.indexOf(status);
    if (index === -1) return "st-other";
    return EXTRA_STATUS_CLASSES[index % EXTRA_STATUS_CLASSES.length];
  }

  function renderStats() {
    const breakdown = statsLib.statusBreakdown(allApplications, statusDefs);
    const response = statsLib.responseRate(allApplications, statusDefs);

    dom.summaryTotal.textContent = String(breakdown.total);
    dom.summaryResponseRate.textContent = statsLib.percent(response.rate);
    dom.summaryResponded.textContent = String(response.responded);
    dom.summaryWaiting.textContent = String(response.considered - response.responded);

    const radius = 34;
    dom.summaryDonutArcs.innerHTML = statsLib
      .donutSegments(breakdown, radius)
      .map(
        (arc) =>
          `<circle class="donut-arc ${statusClass(arc.status)}" cx="42" cy="42" r="${radius}"
             stroke-dasharray="${arc.length} ${arc.gap}" stroke-dashoffset="${arc.offset}" />`
      )
      .join("");

    // Meters are scaled against the biggest status, not the total, so a
    // spread-out set of statuses doesn't render as six near-empty bars.
    const busiest = breakdown.segments.reduce((n, seg) => Math.max(n, seg.count), 0);
    dom.summaryRows.innerHTML = breakdown.segments
      .map((seg) => {
        const width = busiest ? (seg.count / busiest) * 100 : 0;
        const classes = ["detail-row"];
        if (seg.count === 0) classes.push("muted");
        if (seg.status === statusFilter) classes.push("active");
        return `<button type="button" class="${classes.join(" ")}" data-status="${escapeHtml(seg.status)}"
          aria-pressed="${seg.status === statusFilter}"
          title="Show only ${escapeHtml(seg.status)} applications">
          <i class="detail-chip ${statusClass(seg.status)}"></i>
          <span class="detail-label">${escapeHtml(seg.status)}</span>
          <span class="meter"><span class="meter-fill ${statusClass(seg.status)}" style="width: ${width}%"></span></span>
          <span class="detail-value">${seg.count}</span>
          <span class="detail-share">${statsLib.percent(seg.share)}</span>
        </button>`;
      })
      .join("");
  }

  /// Sorts display order only - `index` stays the row's position in the
  /// workbook, which is what every write command addresses rows by.
  function sortRows(rows) {
    const { key, direction } = listSort;
    const sign = direction === "asc" ? 1 : -1;
    return rows.slice().sort((a, b) => {
      const left = (a.app[key] || "").toString().toLowerCase();
      const right = (b.app[key] || "").toString().toLowerCase();
      if (left === right) return a.index - b.index; // stable, by workbook order
      // Dates are ISO, so a plain string compare is already chronological.
      return left < right ? -sign : sign;
    });
  }

  function renderSortIndicators() {
    for (const th of dom.sortHeaders) {
      const active = th.dataset.sort === listSort.key;
      th.classList.toggle("sorted", active);
      th.classList.toggle("desc", active && listSort.direction === "desc");
      th.setAttribute(
        "aria-sort",
        active ? (listSort.direction === "asc" ? "ascending" : "descending") : "none"
      );
    }
  }

  /// Fields the search box looks through. Notes is included deliberately -
  /// with Tesseract it holds the whole raw OCR text of the posting, which is
  /// often the only place a team name or a requirement was captured.
  const SEARCH_FIELDS = ["company", "position", "location", "job_id", "notes"];

  function matchesQuery(app, query) {
    if (!query) return true;
    return SEARCH_FIELDS.some((field) =>
      (app[field] || "").toString().toLowerCase().includes(query)
    );
  }

  function renderApplicationsTable() {
    renderStats();
    renderSortIndicators();
    const query = dom.searchBox.value.trim().toLowerCase();
    const rows = sortRows(
      allApplications
        .map((app, index) => ({ app, index }))
        .filter(({ app }) => matchesQuery(app, query))
        .filter(({ app }) => !statusFilter || (app.status || "Applied") === statusFilter)
    );

    renderFilterChip();

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
          <td>${
            app.resume
              ? `<a href="#" class="row-resume" data-index="${index}">PDF</a>`
              : ""
          }</td>
          <td><button type="button" class="row-delete" data-index="${index}" title="Delete this application" aria-label="Delete ${escapeHtml(app.company)} ${escapeHtml(app.position)}">&times;</button></td>
        </tr>`;
      })
      .join("");
  }

  let statusOptionsCache = statsLib.namesOf(statsLib.DEFAULT_STATUS_DEFS);

  function statusSelect(index, current) {
    const options = statusOptionsCache
      .map((s) => `<option value="${s}" ${s === current ? "selected" : ""}>${s}</option>`)
      .join("");
    return `<select data-index="${index}" class="row-status">${options}</select>`;
  }

  const SUMMARY_COLLAPSED_KEY = "jobTracker.summaryCollapsed";

  function applySummaryCollapsed(collapsed) {
    dom.summaryPanel.classList.toggle("collapsed", collapsed);
    dom.summaryToggle.setAttribute("aria-expanded", String(!collapsed));
  }

  dom.summaryToggle.addEventListener("click", () => {
    const collapsed = !dom.summaryPanel.classList.contains("collapsed");
    applySummaryCollapsed(collapsed);
    try {
      window.localStorage.setItem(SUMMARY_COLLAPSED_KEY, collapsed ? "1" : "0");
    } catch (_) {
      // A blocked or full store just means the choice lasts this session.
    }
  });

  function restoreSummaryCollapsed() {
    let collapsed = false;
    try {
      collapsed = window.localStorage.getItem(SUMMARY_COLLAPSED_KEY) === "1";
    } catch (_) {
      /* default to open */
    }
    applySummaryCollapsed(collapsed);
  }

  dom.summaryRows.addEventListener("click", (e) => {
    const row = e.target.closest(".detail-row");
    if (!row) return;
    setStatusFilter(row.dataset.status);
  });

  dom.filterChipClear.addEventListener("click", () => setStatusFilter(statusFilter));

  dom.sortHeaders.forEach((th) => {
    th.addEventListener("click", () => {
      const key = th.dataset.sort;
      if (listSort.key === key) {
        listSort.direction = listSort.direction === "asc" ? "desc" : "asc";
      } else {
        // Dates open newest-first; names open A-Z.
        listSort = { key, direction: key === "date_applied" ? "desc" : "asc" };
      }
      renderApplicationsTable();
    });
  });

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
    if (e.target.classList.contains("row-delete")) {
      e.preventDefault();
      const index = Number(e.target.dataset.index);
      const app = allApplications[index];
      if (!app) return;
      // Deleting is the one write with no in-app undo, so it asks first.
      // The workbook is still backed up to backups/ before being rewritten.
      const ok = window.confirm(
        `Delete the ${app.company} - ${app.position} application?

` +
          "The workbook is backed up first, so this can be recovered from the " +
          "backups folder next to it."
      );
      if (!ok) return;
      await deleteApplication(index, app);
      return;
    }
    if (e.target.classList.contains("row-resume")) {
      e.preventDefault();
      const app = allApplications[Number(e.target.dataset.index)];
      if (!app || !app.resume) return;
      try {
        await invoke("open_saved_resume", { path: app.resume });
      } catch (err) {
        dom.listStatus.textContent = String(err);
      }
      return;
    }
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

  async function deleteApplication(index, app) {
    try {
      await invoke("delete_application_at_index", {
        index,
        expectedCompany: app.company,
        expectedPosition: app.position,
      });
      // Keep the whole row, not just its index: undo has to put back every
      // field, and the workbook no longer holds any of them.
      lastDeleted = { application: { ...app }, index };
      // Re-read rather than splicing locally: every other row's index just
      // shifted, and the calendar is keyed off the same array.
      await loadApplications();
      showUndoToast(`Deleted ${app.company} - ${app.position}.`);
    } catch (e) {
      dom.listStatus.textContent = String(e);
      showRetryToast(`Couldn't delete that row: ${e}`, () => deleteApplication(index, app));
    }
  }

  function showUndoToast(message) {
    dom.undoMessage.textContent = message;
    dom.undoToast.hidden = false;
  }

  function hideUndoToast() {
    dom.undoToast.hidden = true;
    lastDeleted = null;
  }

  dom.undoDismiss.addEventListener("click", hideUndoToast);

  dom.undoBtn.addEventListener("click", async () => {
    if (!lastDeleted) return;
    const { application, index } = lastDeleted;
    hideUndoToast();
    try {
      await invoke("insert_application_at_index", { index, application });
      await loadApplications();
      dom.listStatus.textContent = `Restored ${application.company} - ${application.position}.`;
    } catch (e) {
      dom.listStatus.textContent = String(e);
      showRetryToast(`Couldn't restore that row: ${e}`, () => {
        lastDeleted = { application, index };
        dom.undoBtn.click();
      });
    }
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

    dom.calViewButtons.forEach((btn) => {
      const selected = btn.dataset.view === calView;
      btn.classList.toggle("active", selected);
      btn.setAttribute("aria-selected", String(selected));
    });
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

    renderCalendarFigures(
      cal.monthStats(year, month, byDate),
      cal.currentStreak(byDate, calTodayParts),
      "This month"
    );
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

    renderCalendarFigures(
      cal.yearStats(year, byDate),
      cal.currentStreak(byDate, calTodayParts),
      String(year)
    );
  }

  /// The month and year views show the same four figures, as labelled
  /// values rather than a run-on sentence.
  function renderCalendarFigures(figures, streak, periodLabel) {
    const cells = [
      [periodLabel, figures.total],
      ["Active days", figures.activeDays],
      ["Busiest day", figures.busiest ? figures.busiest.count : 0],
      ["Streak", streak],
    ];
    dom.calStats.innerHTML = cells
      .map(
        ([label, value]) =>
          `<div><dt>${escapeHtml(label)}</dt><dd>${value}</dd></div>`
      )
      .join("");
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

  // ---------- Resume tab ----------

  let savedResumePath = null;
  // The structured resume behind the preview. Saving renders the PDF and
  // the .tex from this, not from the text on screen.
  let tailoredResume = null;

  /// A readable rendering of the structure, for checking before sending.
  function resumePreview(r) {
    const lines = [r.name, r.contact, ""];
    if (r.summary) lines.push(r.summary, "");
    for (const section of r.sections || []) {
      lines.push(section.heading.toUpperCase());
      if (section.items && section.items.length) lines.push(section.items.join(" · "));
      for (const entry of section.entries || []) {
        const right = [entry.location, entry.dates].filter(Boolean).join(", ");
        lines.push(`${entry.title}${right ? "  -  " + right : ""}`);
        if (entry.organisation) lines.push(`  ${entry.organisation}`);
        for (const bullet of entry.bullets || []) lines.push(`  • ${bullet}`);
      }
      lines.push("");
    }
    return lines.join("\n");
  }

  async function loadResumeTab() {
    try {
      dom.masterResume.value = await invoke("get_master_resume");
    } catch (e) {
      dom.masterResumeMessage.textContent = String(e);
    }
    // The picker is filled from the workbook, so tailoring can start from
    // a row already saved rather than retyping the company and role.
    if (allApplications.length === 0) {
      try {
        allApplications = await invoke("list_applications");
      } catch (_) {
        /* the picker just stays empty */
      }
    }
    const current = dom.resumeJobPicker.value;
    dom.resumeJobPicker.innerHTML =
      `<option value="">Not one of my saved applications</option>` +
      allApplications
        .map(
          (app, i) =>
            `<option value="${i}">${escapeHtml(app.company)} - ${escapeHtml(app.position)}</option>`
        )
        .join("");
    if (current) dom.resumeJobPicker.value = current;
  }

  dom.masterResumeImport.addEventListener("click", async () => {
    try {
      const path = await invoke("pick_resume_file");
      // Clear first: leaving the last run's "Imported and saved" up after
      // someone cancels the picker reads as though it imported again.
      dom.masterResumeMessage.textContent = "";
      if (!path) return;
      dom.masterResumeMessage.textContent = "Reading…";
      dom.masterResume.value = await invoke("import_resume_file", { path });
      dom.masterResumeMessage.textContent = "Imported and saved - check it read cleanly.";
    } catch (e) {
      dom.masterResumeMessage.textContent = String(e);
    }
  });

  dom.masterResumeSave.addEventListener("click", async () => {
    try {
      const path = await invoke("set_master_resume", { text: dom.masterResume.value });
      dom.masterResumeMessage.textContent = `Saved to ${path}`;
    } catch (e) {
      dom.masterResumeMessage.textContent = String(e);
    }
  });

  function selectedResumeJob() {
    const index = dom.resumeJobPicker.value;
    return index === "" ? null : allApplications[Number(index)];
  }

  dom.resumeTailor.addEventListener("click", async () => {
    const job = selectedResumeJob();
    const pasted = dom.resumeJobText.value.trim();
    if (!job && !pasted) {
      dom.resumeTailorMessage.textContent =
        "Pick an application, or paste the posting.";
      return;
    }

    dom.resumeTailor.disabled = true;
    dom.resumeTailorMessage.textContent = "Writing… this takes a few seconds.";
    try {
      tailoredResume = await invoke("tailor_resume", {
        company: job ? job.company : "",
        position: job ? job.position : "",
        location: job && job.location ? job.location : "",
        notes: job && job.notes ? job.notes : "",
        pasted,
      });
      dom.resumeResult.value = resumePreview(tailoredResume);
      dom.resumeResultGroup.hidden = false;
      dom.resumeTailorMessage.textContent = "Done - read it before you send it.";
      savedResumePath = null;
      dom.resumeOpenFile.hidden = true;
      dom.resumeSaveMessage.textContent = "";
    } catch (e) {
      dom.resumeTailorMessage.textContent = String(e);
    } finally {
      dom.resumeTailor.disabled = false;
    }
  });

  dom.resumeSaveFile.addEventListener("click", async () => {
    if (!tailoredResume) return;
    const job = selectedResumeJob();
    try {
      const saved = await invoke("save_tailored_resume", {
        company: job ? job.company : "",
        position: job ? job.position : "",
        resume: tailoredResume,
      });
      savedResumePath = saved.pdf;
      dom.resumeSaveMessage.textContent = saved.linked
        ? `Saved ${saved.pdf}, and recorded against that application.`
        : `Saved ${saved.pdf} and the .tex beside it.`;
      // The row now carries the resume, so the list has to be re-read
      // for the link to appear against it.
      if (saved.linked) await loadApplications();
      dom.resumeOpenFile.hidden = false;
    } catch (e) {
      dom.resumeSaveMessage.textContent = String(e);
    }
  });

  dom.resumeOpenFile.addEventListener("click", async () => {
    if (!savedResumePath) return;
    try {
      await invoke("open_saved_resume", { path: savedResumePath });
    } catch (e) {
      dom.resumeSaveMessage.textContent = String(e);
    }
  });

  // ---------- Settings tab ----------

  let providers = [];

  function currentProvider() {
    return (
      providers.find((p) => p.id === currentExtractionMethod) ||
      providers.find((p) => p.id === "tesseract") ||
      { id: "tesseract", label: "Tesseract", needs_key: false, key_label: "", key_placeholder: "", key_help: "" }
    );
  }

  async function refreshExtractionMethodStatus() {
    const provider = currentProvider();
    if (provider.id === "system") {
      const available = await invoke("system_ocr_available");
      dom.extractionMethodStatus.textContent = available
        ? "Uses the OCR engine already built into this machine. Nothing to install, nothing leaves your computer."
        : "This machine has no built-in OCR engine - it needs Windows 10 or later. Pick another method below.";
    } else if (provider.id === "tesseract") {
      const available = await invoke("local_ocr_available");
      dom.extractionMethodStatus.textContent = available
        ? "Tesseract detected - screenshots are read on this machine, for free, and nothing is sent anywhere."
        : "Tesseract not found. Install it with `brew install tesseract` (macOS), your package manager (Linux), or from github.com/UB-Mannheim/tesseract/wiki (Windows) - or pick one of the cloud methods below.";
    } else if (provider.id === "ollama") {
      dom.extractionMethodStatus.textContent =
        "Tesseract reads the screenshot, then a model on this machine pulls the fields out of the text. No key, nothing leaves your machine.";
    } else {
      dom.extractionMethodStatus.textContent = `Sends the screenshot to ${provider.label} for higher-accuracy extraction. Needs an API key, below.`;
    }
  }

  let fallbackChain = [];

  async function renderFallbacks() {
    try {
      fallbackChain = await invoke("get_fallback_chain");
    } catch (_) {
      fallbackChain = [];
    }
    const label = (id) => {
      const p = providers.find((x) => x.id === id);
      return p ? p.label : id;
    };

    dom.fallbackList.innerHTML = fallbackChain.length
      ? fallbackChain
          .map(
            (id, i) => `<li class="fallback-row">
              <span class="fallback-rank">${i + 1}.</span>
              <span class="fallback-name">${escapeHtml(label(id))}</span>
              <button type="button" data-move="up" data-index="${i}" ${i === 0 ? "disabled" : ""} title="Move up" aria-label="Move ${escapeHtml(label(id))} up">&uarr;</button>
              <button type="button" data-move="down" data-index="${i}" ${i === fallbackChain.length - 1 ? "disabled" : ""} title="Move down" aria-label="Move ${escapeHtml(label(id))} down">&darr;</button>
              <button type="button" data-remove="${i}" title="Remove" aria-label="Remove ${escapeHtml(label(id))}">&times;</button>
            </li>`
          )
          .join("")
      : `<li class="fallback-empty">Nothing yet - a failure means typing the entry in by hand.</li>`;

    // Only methods not already in play: the primary, and anything listed.
    const taken = new Set([currentExtractionMethod, ...fallbackChain]);
    const available = providers.filter((p) => !taken.has(p.id));
    dom.fallbackAdd.innerHTML = available
      .map((p) => `<option value="${escapeHtml(p.id)}">${escapeHtml(p.label)}</option>`)
      .join("");
    dom.fallbackAdd.disabled = available.length === 0;
    dom.fallbackAddBtn.disabled = available.length === 0;
  }

  async function saveFallbacks(chain) {
    try {
      fallbackChain = await invoke("set_fallback_chain", { chain });
    } catch (_) {
      /* keep what is on screen; the next render re-reads it */
    }
    await renderFallbacks();
    // A fallback that needs a key needs somewhere to put it.
    await renderProviderCards();
  }

  dom.fallbackAddBtn.addEventListener("click", () => {
    if (!dom.fallbackAdd.value) return;
    saveFallbacks([...fallbackChain, dom.fallbackAdd.value]);
  });

  dom.fallbackList.addEventListener("click", (e) => {
    const btn = e.target.closest("button");
    if (!btn) return;
    const next = [...fallbackChain];
    if (btn.dataset.remove !== undefined) {
      next.splice(Number(btn.dataset.remove), 1);
    } else {
      const i = Number(btn.dataset.index);
      const j = btn.dataset.move === "up" ? i - 1 : i + 1;
      if (j < 0 || j >= next.length) return;
      [next[i], next[j]] = [next[j], next[i]];
    }
    saveFallbacks(next);
  });

  /// A card per method actually in use - the chosen one, then each
  /// fallback. Rendered rather than a single fixed card because a fallback
  /// needs its key too, and there was previously no way to enter one
  /// without temporarily making it the primary.
  ///
  /// An offline setup renders nothing here, so it never sees a cloud
  /// provider's name or key wording.
  async function renderProviderCards() {
    const inUse = [currentExtractionMethod, ...fallbackChain]
      .map((id) => providers.find((p) => p.id === id))
      .filter((p) => p && (p.needs_key || (p.default_model && p.id !== "ollama")));

    if (inUse.length === 0) {
      dom.providerCards.innerHTML = "";
      return;
    }

    const cards = await Promise.all(
      inUse.map(async (p, i) => {
        const stored = p.needs_key ? await invoke("has_api_key", { provider: p.id }) : false;
        let model = p.default_model;
        if (p.default_model) {
          try {
            model = await invoke("get_model", { provider: p.id });
          } catch (_) {
            /* fall back to the shipped default */
          }
        }
        // Only asked for when a key is stored. The shipped default was a
        // guess made before anyone had a key, and a gateway serves an
        // entirely different set - so if it isn't on offer, take the best
        // that is rather than failing at capture time with a 404.
        let available = [];
        let autoPicked = false;
        if (stored && p.default_model) {
          try {
            const choice = await invoke("auto_select_model", { provider: p.id });
            available = choice.available;
            autoPicked = choice.changed;
            if (choice.changed) model = choice.model;
          } catch (_) {
            /* leave the configured model alone */
          }
        }
        const role = i === 0 ? "your chosen method" : `fallback ${i}`;
        return `<div class="settings-group provider-card" data-provider="${escapeHtml(p.id)}">
          <h2>${escapeHtml(p.needs_key ? p.key_label : p.label)}</h2>
          <p class="hint">For ${escapeHtml(p.label)} - ${escapeHtml(role)}.</p>
          ${p.needs_key ? keyBlock(p, stored) : ""}
          ${p.default_model ? modelBlock(p, model, available) : ""}
          <p class="card-message hint"${autoPicked ? "" : " hidden"}>${
            autoPicked
              ? `Your key doesn't offer the default, so ${escapeHtml(model)} was picked - the best of the ${available.length} it does offer.`
              : ""
          }</p>
        </div>`;
      })
    );
    dom.providerCards.innerHTML = cards.join("");
  }

  /// Two states: a key is stored, or it is not. When one is stored the
  /// field is put away - leaving an empty password box next to "a key is
  /// stored" reads as though it did not save.
  function keyBlock(p, stored) {
    if (stored) {
      return `<div class="key-stored">
        <span class="key-badge">Added</span>
        <span class="hint">Held in your OS keychain, never written to a file.</span>
        <button type="button" class="btn btn-link btn-small" data-act="test">Test</button>
        <button type="button" class="btn btn-link btn-small" data-act="replace">Replace</button>
        <button type="button" class="btn btn-link btn-small" data-act="remove">Remove</button>
      </div>
      <div class="key-entry" hidden>
        <input type="password" class="key-input" placeholder="${escapeHtml(p.key_placeholder)}" autocomplete="off" />
        <div class="settings-actions">
          <button type="button" class="btn btn-secondary btn-small" data-act="save">Save</button>
          <button type="button" class="btn btn-link btn-small" data-act="cancel">Cancel</button>
        </div>
      </div>`;
    }
    return `<div class="key-entry">
      <input type="password" class="key-input" placeholder="${escapeHtml(p.key_placeholder)}" autocomplete="off" />
      <p class="hint">${escapeHtml(p.key_help)}</p>
      <div class="settings-actions">
        <button type="button" class="btn btn-secondary btn-small" data-act="save">Save key</button>
      </div>
    </div>`;
  }

  /// A datalist rather than a dropdown: the list is what the key can
  /// actually reach, and typing still works for anything not in it.
  /// Guessing at a name like "protected.Claude Sonnet 4.6" is otherwise a
  /// coin flip, and getting it wrong reads as a broken key.
  function modelBlock(p, model, available) {
    const listId = `models-${p.id}`;
    const options = (available || [])
      .map((m) => `<option value="${escapeHtml(m)}"></option>`)
      .join("");
    return `<label class="settings-sublabel">Model${available && available.length ? ` - ${available.length} available` : ""}</label>
      <div class="path-row">
        <input type="text" class="model-input" list="${listId}" value="${escapeHtml(model)}" placeholder="${escapeHtml(p.default_model)}" autocomplete="off" spellcheck="false" />
        <datalist id="${listId}">${options}</datalist>
        <button type="button" class="btn btn-secondary btn-small" data-act="save-model">Save</button>
        <button type="button" class="btn btn-link btn-small" data-act="reset-model">Default</button>
      </div>`;
  }

  function cardMessage(card, text) {
    const node = card.querySelector(".card-message");
    node.hidden = false;
    node.textContent = text;
  }

  dom.providerCards.addEventListener("click", async (e) => {
    const btn = e.target.closest("button[data-act]");
    if (!btn) return;
    const card = btn.closest(".provider-card");
    const id = card.dataset.provider;
    const act = btn.dataset.act;

    if (act === "replace" || act === "cancel") {
      card.querySelector(".key-stored").hidden = act === "replace";
      card.querySelector(".key-entry").hidden = act === "cancel";
      return;
    }
    if (act === "save") {
      const key = card.querySelector(".key-input").value.trim();
      if (!key) return cardMessage(card, "Paste a key first.");
      try {
        await invoke("save_api_key", { provider: id, key });
        await renderProviderCards();
      } catch (err) {
        cardMessage(card, String(err));
      }
      return;
    }
    if (act === "test") {
      btn.disabled = true;
      cardMessage(card, "Checking…");
      try {
        cardMessage(card, await invoke("test_api_key", { provider: id }));
      } catch (err) {
        cardMessage(card, String(err));
      } finally {
        btn.disabled = false;
      }
      return;
    }
    if (act === "remove") {
      try {
        await invoke("delete_api_key", { provider: id });
        await renderProviderCards();
      } catch (err) {
        cardMessage(card, String(err));
      }
      return;
    }
    if (act === "save-model" || act === "reset-model") {
      const input = card.querySelector(".model-input");
      const value = act === "reset-model" ? "" : input.value;
      try {
        const inForce = await invoke("set_model", { provider: id, model: value });
        input.value = inForce;
        cardMessage(
          card,
          value.trim() === "" ? `Back to the default, ${inForce}.` : `Using ${inForce}.`
        );
      } catch (err) {
        cardMessage(card, String(err));
      }
    }
  });

  async function renderOllamaSection() {
    const provider = currentProvider();
    dom.ollamaGroup.hidden = provider.id !== "ollama";
    if (provider.id !== "ollama") return;
    try {
      dom.settingsOllamaHost.value = await invoke("get_ollama_host");
      dom.settingsOllamaUnload.checked = await invoke("get_ollama_unload");
    } catch (_) {
      dom.settingsOllamaHost.value = "";
    }
    await refreshOllamaStatus();
  }

  /// Reports the three states that need different actions, and picks a
  /// model out of what is already pulled so nothing has to be typed.
  async function refreshOllamaStatus() {
    dom.ollamaStatus.textContent = "Checking…";
    let status;
    try {
      status = await invoke("ollama_status");
    } catch (e) {
      dom.ollamaStatus.textContent = `Couldn't check Ollama: ${e}`;
      return;
    }

    dom.ollamaModelRow.hidden = !status.model_ready;
    dom.ollamaPullRow.hidden = status.model_ready || !status.running;

    if (!status.running) {
      dom.ollamaStatus.textContent =
        "Not reachable. Install Ollama from ollama.com - it starts itself once installed.";
      return;
    }
    if (!status.model_ready) {
      dom.ollamaStatus.textContent = `Running, but no model is downloaded yet. ${status.recommended} is about 2 GB and runs on a CPU.`;
      dom.ollamaPull.textContent = `Download ${status.recommended}`;
      return;
    }

    await renderOllamaCatalogue(status.model);
    const chosen = ollamaCatalogue.find((m) => m.id === status.model);
    dom.ollamaStatus.textContent = chosen
      ? `Ready, using ${chosen.label}. ${chosen.vision ? "It reads the screenshot itself." : "Tesseract reads the screenshot, it reads the text."} Nothing leaves this machine.`
      : `Ready, using ${status.model}. Nothing leaves this machine.`;
  }

  let ollamaCatalogue = [];

  /// Lists what the app offers rather than only what is downloaded, so a
  /// better model is discoverable instead of something you had to know to
  /// go looking for. Split by whether it reads the image or the OCR text,
  /// because that is the difference that actually changes results.
  async function renderOllamaCatalogue(selected) {
    try {
      ollamaCatalogue = await invoke("ollama_models");
    } catch (_) {
      ollamaCatalogue = [];
      return;
    }
    const option = (m) =>
      `<option value="${escapeHtml(m.id)}"${m.id === selected ? " selected" : ""}>` +
      `${escapeHtml(m.label)} - ${escapeHtml(m.accuracy)}, ${escapeHtml(m.size)}` +
      `${m.downloaded ? "" : " (not downloaded)"}</option>`;
    const vision = ollamaCatalogue.filter((m) => m.vision);
    const text = ollamaCatalogue.filter((m) => !m.vision);
    dom.settingsOllamaModel.innerHTML =
      `<optgroup label="Reads the screenshot itself - no Tesseract needed">${vision.map(option).join("")}</optgroup>` +
      `<optgroup label="Reads the text Tesseract found">${text.map(option).join("")}</optgroup>`;
    renderOllamaModelDetail(selected);
  }

  function renderOllamaModelDetail(id) {
    const m = ollamaCatalogue.find((x) => x.id === id);
    if (!m) {
      dom.ollamaModelDetail.hidden = true;
      return;
    }
    dom.ollamaModelDetail.hidden = false;
    dom.ollamaModelDetail.innerHTML =
      `${escapeHtml(m.description)}<br><strong>${escapeHtml(m.accuracy)}</strong> · ` +
      `${escapeHtml(m.size)} · ${escapeHtml(m.hardware)}` +
      (m.downloaded ? "" : ` · <em>not downloaded yet</em>`);
    dom.ollamaPullRow.hidden = m.downloaded;
    if (!m.downloaded) dom.ollamaPull.textContent = `Download ${m.id}`;
  }

  dom.settingsOllamaModel.addEventListener("change", async () => {
    try {
      await invoke("set_model", { provider: "ollama", model: dom.settingsOllamaModel.value });
      renderOllamaModelDetail(dom.settingsOllamaModel.value);
      await refreshOllamaStatus();
    } catch (e) {
      dom.ollamaStatus.textContent = String(e);
    }
  });

  listen("ollama-pull-progress", (event) => {
    const { status, completed, total } = event.payload || {};
    dom.ollamaPullStatus.hidden = false;
    if (total > 0) {
      const pct = Math.min(100, Math.round((completed / total) * 100));
      dom.ollamaPullBar.classList.remove("indeterminate");
      dom.ollamaPullBar.style.width = `${pct}%`;
      dom.ollamaPullStatus.textContent = `${status} - ${pct}%`;
    } else {
      dom.ollamaPullBar.classList.add("indeterminate");
      dom.ollamaPullStatus.textContent = status || "Downloading…";
    }
  });

  dom.ollamaPull.addEventListener("click", async () => {
    const model = dom.ollamaPull.textContent.replace("Download ", "").trim();
    dom.ollamaPull.disabled = true;
    dom.ollamaPullProgress.hidden = false;
    dom.ollamaPullStatus.hidden = false;
    dom.ollamaPullStatus.textContent = "Starting…";
    try {
      await invoke("pull_ollama_model", { model });
      dom.ollamaPullStatus.textContent = `${model} downloaded.`;
      await refreshOllamaStatus();
    } catch (e) {
      dom.ollamaPullStatus.textContent = String(e);
    } finally {
      dom.ollamaPull.disabled = false;
      dom.ollamaPullProgress.hidden = true;
      dom.ollamaPullBar.style.width = "0%";
    }
  });

  dom.settingsOllamaUnload.addEventListener("change", async () => {
    try {
      await invoke("set_ollama_unload", { enabled: dom.settingsOllamaUnload.checked });
    } catch (e) {
      dom.ollamaStatus.textContent = String(e);
    }
  });

  dom.settingsOllamaSave.addEventListener("click", async () => {
    try {
      dom.settingsOllamaHost.value = await invoke("set_ollama_host", {
        host: dom.settingsOllamaHost.value,
      });
      await refreshOllamaStatus();
    } catch (e) {
      dom.ollamaStatus.textContent = String(e);
    }
  });

  dom.settingsOllamaReset.addEventListener("click", async () => {
    try {
      dom.settingsOllamaHost.value = await invoke("set_ollama_host", { host: "" });
      await refreshOllamaStatus();
    } catch (e) {
      dom.ollamaStatus.textContent = String(e);
    }
  });


  async function loadExtractionMethod() {
    try {
      providers = await invoke("get_extraction_providers");
    } catch (_) {
      providers = [];
    }
    try {
      currentExtractionMethod = await invoke("get_extraction_method");
    } catch (_) {
      currentExtractionMethod = "tesseract";
    }

    // Grouped so the choice reads as "what kind of thing" first and "which
    // one" second: offline and cloud are different decisions, not five
    // equivalent options.
    const groups = [];
    for (const p of providers) {
      const name = p.group || "";
      const existing = groups.find((g) => g.name === name);
      if (existing) existing.items.push(p);
      else groups.push({ name, items: [p] });
    }
    const option = (p) =>
      `<option value="${escapeHtml(p.id)}"${p.id === currentExtractionMethod ? " selected" : ""}>${escapeHtml(p.label)}</option>`;
    dom.extractionMethodSelect.innerHTML = groups
      .map((g) =>
        g.name
          ? `<optgroup label="${escapeHtml(g.name)}">${g.items.map(option).join("")}</optgroup>`
          : g.items.map(option).join("")
      )
      .join("");

    await refreshExtractionMethodStatus();
    await renderProviderCards();
    await renderOllamaSection();
    await renderFallbacks();
  }

  dom.extractionMethodSelect.addEventListener("change", async () => {
    currentExtractionMethod = dom.extractionMethodSelect.value;
    try {
      await invoke("set_extraction_method", { method: currentExtractionMethod });
    } catch (_) {
      /* best effort - the in-memory value still applies this session */
    }
    await refreshExtractionMethodStatus();
    await renderProviderCards();
    await renderOllamaSection();
    // Changing the primary can invalidate the fallback list: the backend
    // drops the new primary from it, and the "add" options shift.
    await renderFallbacks();
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

      // Pointing somewhere else would otherwise start an empty workbook and
      // strand the old one. Asked rather than assumed, since moving deletes
      // the original.
      let moveExisting = false;
      if (await invoke("workbook_exists")) {
        moveExisting = window.confirm(
          "Move your existing workbook to the new location?\n\n" +
            "Its backups and archived screenshots come with it, and the old " +
            "copy is removed. Choose Cancel to leave it where it is and " +
            "start fresh at the new path."
        );
      }

      const result = await invoke("set_excel_path", { path, moveExisting });
      dom.settingsExcelPath.value = path;
      dom.settingsPathMessage.hidden = false;
      dom.settingsPathMessage.textContent = describePathChange(result);
    } catch (e) {
      dom.settingsPathMessage.hidden = false;
      dom.settingsPathMessage.textContent = String(e);
    }
  });

  function describePathChange(result) {
    if (!result || result.outcome === "Switched") return "Saved.";
    if (result.outcome === "DestinationExists") {
      return "Saved, but nothing was moved - a workbook is already there, and overwriting it would have destroyed one of them.";
    }
    const bits = ["Workbook moved"];
    if (result.screenshots) bits.push(`${result.screenshots} screenshot${result.screenshots === 1 ? "" : "s"}`);
    if (result.backups) bits.push(`${result.backups} backup${result.backups === 1 ? "" : "s"}`);
    return bits.length > 1 ? `${bits.join(", ")} came too.` : "Workbook moved.";
  }

  // ---------- Settings: import and statuses ----------

  dom.importBtn.addEventListener("click", async () => {
    dom.importBtn.disabled = true;
    dom.importMessage.hidden = false;
    dom.importMessage.textContent = "Choosing a file…";
    try {
      const path = await invoke("pick_import_file");
      if (!path) {
        dom.importMessage.hidden = true;
        return;
      }
      dom.importMessage.textContent = "Importing…";
      const summary = await invoke("import_applications", { path });
      const parts = [`Imported ${summary.imported} application${summary.imported === 1 ? "" : "s"}`];
      if (summary.skipped_duplicates) {
        parts.push(`${summary.skipped_duplicates} already here`);
      }
      if (summary.skipped_blank) {
        parts.push(`${summary.skipped_blank} blank row${summary.skipped_blank === 1 ? "" : "s"} skipped`);
      }
      dom.importMessage.textContent = `${parts.join(", ")}.`;
      if (summary.imported > 0) await loadApplications();
    } catch (e) {
      dom.importMessage.textContent = String(e);
    } finally {
      dom.importBtn.disabled = false;
    }
  });

  const STATUS_KINDS = [
    ["waiting", "No reply yet"],
    ["replied", "They replied"],
    ["closed", "Closed by me"],
  ];

  /// Rows are rebuilt from `statusDefs` on every change rather than edited
  /// in place, so what is on screen is always exactly what would be saved.
  function renderStatusSettings() {
    dom.statusList.innerHTML = statusDefs
      .map(
        (def, i) => `<div class="status-edit-row">
          <input type="text" class="status-name" data-index="${i}" value="${escapeHtml(def.name)}" maxlength="40" aria-label="Status name" />
          <select class="status-kind" data-index="${i}" aria-label="What this status means">
            ${STATUS_KINDS.map(
              ([value, label]) =>
                `<option value="${value}"${def.kind === value ? " selected" : ""}>${label}</option>`
            ).join("")}
          </select>
          <button type="button" class="status-remove btn-link btn-small" data-index="${i}"
            title="Remove this status" aria-label="Remove ${escapeHtml(def.name)}">&times;</button>
        </div>`
      )
      .join("");
  }

  async function saveStatusDefs(defs) {
    try {
      statusDefs = await invoke("set_status_defs", { defs });
      statusOptionsCache = statsLib.namesOf(statusDefs);
      dom.statusMessage.hidden = true;
      renderStatusSettings();
      await populateStatusDropdown();
      renderApplicationsTable();
    } catch (e) {
      dom.statusMessage.hidden = false;
      dom.statusMessage.textContent = String(e);
      renderStatusSettings();
    }
  }

  function rejectStatusEdit(message) {
    dom.statusMessage.hidden = false;
    dom.statusMessage.textContent = message;
    renderStatusSettings(); // puts the field back to the stored value
  }

  dom.statusList.addEventListener("change", (e) => {
    const index = Number(e.target.dataset.index);
    if (Number.isNaN(index)) return;
    const next = statusDefs.map((d) => ({ ...d }));

    if (e.target.classList.contains("status-name")) {
      const name = e.target.value.trim();
      // The backend drops blanks and duplicates, which is right for a bulk
      // set and wrong for a field being retyped - clearing it to edit it
      // would delete the status. Refuse here instead.
      if (!name) {
        rejectStatusEdit("A status needs a name. Use × to remove one.");
        return;
      }
      const clash = statusDefs.some(
        (d, i) => i !== index && d.name.toLowerCase() === name.toLowerCase()
      );
      if (clash) {
        rejectStatusEdit(`There is already a "${name}" status.`);
        return;
      }
      next[index].name = name;
    } else if (e.target.classList.contains("status-kind")) {
      next[index].kind = e.target.value;
    } else {
      return;
    }
    saveStatusDefs(next);
  });

  dom.statusList.addEventListener("click", (e) => {
    if (!e.target.classList.contains("status-remove")) return;
    const index = Number(e.target.dataset.index);
    const removed = statusDefs[index];
    if (!removed) return;
    const inUse = allApplications.filter((a) => (a.status || "Applied") === removed.name).length;
    const warning = inUse
      ? `

${inUse} saved application${inUse === 1 ? "" : "s"} still use it. They keep it - it just stops being offered for new entries.`
      : "";
    if (!window.confirm(`Remove the "${removed.name}" status?${warning}`)) return;
    saveStatusDefs(statusDefs.filter((_, i) => i !== index));
  });

  dom.statusAdd.addEventListener("click", () => {
    // A fixed name would be dropped as a duplicate on the second click.
    const taken = new Set(statusDefs.map((d) => d.name.toLowerCase()));
    let name = "New status";
    for (let n = 2; taken.has(name.toLowerCase()); n++) name = `New status ${n}`;
    saveStatusDefs(statusDefs.concat([{ name, kind: "waiting" }]));
  });

  async function loadStatusDefs() {
    try {
      statusDefs = await invoke("get_status_defs");
    } catch (_) {
      statusDefs = statsLib.DEFAULT_STATUS_DEFS;
    }
    statusOptionsCache = statsLib.namesOf(statusDefs);
    renderStatusSettings();
  }

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

  // ---------- Hotkey ----------

  let recordingHotkey = false;

  /// Turns a keydown into a Tauri accelerator string ("Ctrl+Shift+J").
  /// Returns null while only modifiers are held, so the user can press and
  /// release Ctrl without it being taken as their choice.
  function acceleratorFrom(e) {
    const parts = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.metaKey) parts.push(isMac ? "Cmd" : "Super");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");

    const key = e.key;
    if (["Control", "Meta", "Alt", "Shift", "OS"].includes(key)) return null;
    if (parts.length === 0) return null; // a bare letter would fire while typing

    let name;
    if (/^[a-z]$/i.test(key)) name = key.toUpperCase();
    else if (/^[0-9]$/.test(key)) name = key;
    else if (/^F\d{1,2}$/.test(key)) name = key;
    else {
      const named = {
        " ": "Space", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left",
        ArrowRight: "Right", Enter: "Enter", Tab: "Tab", Backspace: "Backspace",
        Escape: "Escape", Delete: "Delete", Insert: "Insert", Home: "Home",
        End: "End", PageUp: "PageUp", PageDown: "PageDown",
        ",": "Comma", ".": "Period", "/": "Slash",
        ";": "Semicolon", "'": "Quote", "[": "BracketLeft", "]": "BracketRight",
        "\\": "Backslash", "-": "Minus", "=": "Equal", "`": "Backquote",
      };
      name = named[key];
      if (!name) return null;
    }
    parts.push(name);
    return parts.join("+");
  }

  function stopRecording() {
    recordingHotkey = false;
    dom.settingsHotkeyRecord.textContent = "Change…";
    dom.settingsHotkey.classList.remove("recording");
  }

  dom.settingsHotkeyRecord.addEventListener("click", () => {
    if (recordingHotkey) {
      stopRecording();
      loadHotkey();
      return;
    }
    recordingHotkey = true;
    dom.settingsHotkeyRecord.textContent = "Cancel";
    dom.settingsHotkey.classList.add("recording");
    dom.settingsHotkey.value = "Press a combination…";
    dom.settingsHotkeyMessage.textContent = "Listening - press the keys you want, or Cancel.";
    dom.settingsHotkey.focus();
  });

  dom.settingsHotkeyReset.addEventListener("click", async () => {
    stopRecording();
    await applyHotkey(null);
  });

  async function applyHotkey(shortcut) {
    try {
      const inForce = await invoke("set_hotkey", {
        shortcut: shortcut || (isMac ? "Cmd+Shift+J" : "Ctrl+Shift+J"),
      });
      dom.settingsHotkey.value = inForce;
      dom.setupHotkeyKeys.textContent = inForce;
      dom.settingsHotkeyMessage.textContent = `${inForce} now opens Job Tracker from anywhere.`;
    } catch (e) {
      // Rust put the previous shortcut back, so there is always a way in -
      // reload it so the field shows what is actually in force rather than
      // the "press a combination" placeholder.
      await loadHotkey();
      dom.settingsHotkeyMessage.textContent = `${e} Keeping ${dom.settingsHotkey.value}.`;
    }
  }

  async function loadHotkey() {
    try {
      const current = await invoke("get_hotkey");
      dom.settingsHotkey.value = current;
      dom.setupHotkeyKeys.textContent = current;
    } catch (_) {
      dom.settingsHotkey.value = isMac ? "Cmd+Shift+J" : "Ctrl+Shift+J";
    }
  }

  // ---------- Global hotkey / Esc ----------

  document.addEventListener(
    "keydown",
    (e) => {
      if (!recordingHotkey) return;
      // While recording, every combination belongs to the recorder - including
      // Escape, which would otherwise hide the window mid-edit.
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        stopRecording();
        loadHotkey();
        return;
      }
      const accelerator = acceleratorFrom(e);
      if (!accelerator) return;
      stopRecording();
      applyHotkey(accelerator);
    },
    true
  );

  document.addEventListener("keydown", (e) => {
    if (recordingHotkey) return;
    if (e.key === "Escape") {
      getCurrentWindow().hide();
    }
  });

  listen("capture-shortcut-triggered", () => {
    // The window is being shown - from the tray or the hotkey. This is a
    // tray app that can sit running for days, and the only other check
    // happens when the webview first loads, so a release published while
    // it was open would otherwise never surface. Rust still throttles to
    // once a day and honours the "check automatically" preference, so this
    // costs nothing when there is nothing to find.
    checkForUpdate(false).catch(() => {
      /* an update check must never interrupt opening the window */
    });

    activateTab("capture");
    if (!dom.form.hidden) return;
    dom.dropzone.focus();
  });

  // ---------- Init ----------

  async function init() {
    applyPlatformHints();
    restoreSummaryCollapsed();
    await loadStatusDefs();
    resetCaptureArea();
    await populateStatusDropdown();
    await loadExtractionMethod();
    await loadHotkey();
    await checkFirstRunSetup();
    await refreshExcelPath();
    await loadApplications();
    await loadUpdateSettings();
    // One check on startup, throttled and opt-out-able in Rust. A failure
    // here (offline, endpoint down) must never block using the app.
    checkForUpdate(false).catch(() => {});
  }

  init();
})();
