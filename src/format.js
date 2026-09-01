// Display helpers shared by the rendering code in app.js.
//
// Escaping lives here rather than inside app.js so it can be tested without
// a mocked Tauri bridge - it is the one function in the frontend where a
// mistake is a security bug rather than a cosmetic one.
(() => {
  "use strict";

  /// Escapes a value for interpolation into either element text or an
  /// attribute value.
  ///
  /// The usual textContent/innerHTML trick escapes `&`, `<` and `>` but
  /// leaves quotes alone. That is fine between tags and wrong inside
  /// `attr="..."`, where a double quote closes the attribute early and the
  /// rest of the value is parsed as markup. Every value rendered by this app
  /// - company, position, URL, notes - came off a screenshot, an OCR pass, or
  /// a spreadsheet cell, so none of it is the app's own text.
  function escapeHtml(value) {
    return (value == null ? "" : String(value))
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  window.JobTrackerFormat = { escapeHtml };
})();
