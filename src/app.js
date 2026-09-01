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

    apiKeyStatus: el("api-key-status"),
    settingsApiKey: el("settings-api-key"),
    settingsSaveKey: el("settings-save-key"),
    settingsDeleteKey: el("settings-delete-key"),
    settingsKeyMessage: el("settings-key-message"),
    settingsExcelPath: el("settings-excel-path"),
    settingsChoosePath: el("settings-choose-path"),
    settingsPathMessage: el("settings-path-message"),
    settingsHotkey: el("settings-hotkey"),

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
    if (name === "list") {
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
      const result = await invoke("extract_from_image", { imageBase64: base64, mediaType });
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

  dom.tbody.addEventListener("click", (e) => {
    if (e.target.classList.contains("row-url")) {
      e.preventDefault();
      // Webview navigation to arbitrary external URLs is intentionally
      // not wired up; the link text still lets users copy/see the URL.
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

  // ---------- Settings tab ----------

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
    await checkFirstRunSetup();
    await refreshApiKeyStatus();
    await refreshExcelPath();
  }

  init();
})();
