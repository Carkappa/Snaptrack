// Aggregation behind the summary panel at the top of the Applications tab.
//
// Like calendar.js this is deliberately free of Tauri and DOM dependencies -
// it takes the plain rows `list_applications` returns and gives back plain
// numbers, so it can be exercised on its own (tests/stats.test.html).
(() => {
  "use strict";

  /// The list a fresh install starts from, mirroring `default_status_defs`
  /// in models.rs. The real list comes from the backend and is editable, so
  /// this is only the fallback for a call that doesn't pass one.
  ///
  /// `kind` is what keeps the response rate meaningful once the user can
  /// invent statuses: "waiting" is sent-with-no-answer, "replied" is any
  /// answer including a rejection, "closed" is ended by the user and is
  /// excluded from the rate entirely.
  const DEFAULT_STATUS_DEFS = [
    { name: "Applied", kind: "waiting" },
    { name: "Interviewing", kind: "replied" },
    { name: "Offered", kind: "replied" },
    { name: "Rejected", kind: "replied" },
    { name: "Ghosted", kind: "waiting" },
    { name: "Withdrawn", kind: "closed" },
  ];

  function defsOrDefault(defs) {
    return Array.isArray(defs) && defs.length ? defs : DEFAULT_STATUS_DEFS;
  }

  function namesOf(defs) {
    return defsOrDefault(defs).map((d) => d.name);
  }

  function emptyCounts(order) {
    const counts = Object.create(null);
    for (const status of order) counts[status] = 0;
    return counts;
  }

  /// Counts by status, in display order, with each one's share of the total.
  ///
  /// A status the workbook contains but the app doesn't know about (someone
  /// typed into the cell) is kept rather than dropped, listed after the known
  /// ones, so the numbers on screen always add up to the row count.
  function statusBreakdown(applications, statusDefs) {
    const rows = applications || [];
    const order = namesOf(statusDefs);
    const counts = emptyCounts(order);
    const extras = [];

    for (const app of rows) {
      const status = (app && app.status ? String(app.status) : "").trim() || "Applied";
      if (status in counts) {
        counts[status] += 1;
      } else {
        if (!(status in counts)) extras.push(status);
        counts[status] = (counts[status] || 0) + 1;
      }
    }

    const total = rows.length;
    const listed = order.concat(extras.filter((s, i) => extras.indexOf(s) === i));

    return {
      total,
      segments: listed.map((status) => ({
        status,
        count: counts[status] || 0,
        share: total ? (counts[status] || 0) / total : 0,
        known: order.includes(status),
      })),
    };
  }

  /// How many applications got any reply, out of those that could still get
  /// one. Applications you withdrew are excluded from both sides - you ended
  /// those yourself, and counting them as silence would be misleading.
  function responseRate(applications, statusDefs) {
    const rows = applications || [];
    const defs = defsOrDefault(statusDefs);
    const kindOf = Object.create(null);
    for (const def of defs) kindOf[def.name] = def.kind;

    let considered = 0;
    let responded = 0;
    for (const app of rows) {
      const status = (app && app.status ? String(app.status) : "").trim() || "Applied";
      // A status the list no longer contains still counts as waiting: the
      // row exists, and dropping it would quietly inflate the rate.
      const kind = kindOf[status] || "waiting";
      if (kind === "closed") continue;
      considered += 1;
      if (kind === "replied") responded += 1;
    }
    return {
      responded,
      considered,
      rate: considered ? responded / considered : 0,
    };
  }

  /// Turns a breakdown into donut arc lengths for an SVG circle of the given
  /// radius, as [dashLength, gapLength] plus the offset each segment starts
  /// at. Zero-count statuses are dropped so they can't leave seams.
  function donutSegments(breakdown, radius) {
    const circumference = 2 * Math.PI * radius;
    let consumed = 0;
    return breakdown.segments
      .filter((segment) => segment.count > 0)
      .map((segment) => {
        const length = segment.share * circumference;
        const arc = {
          status: segment.status,
          count: segment.count,
          length,
          gap: circumference - length,
          // SVG dash offsets run backwards around the circle.
          offset: -consumed,
          circumference,
        };
        consumed += length;
        return arc;
      });
  }

  function percent(value) {
    if (!value) return "0%";
    const pct = value * 100;
    // Never round a real, non-zero share down to "0%".
    if (pct > 0 && pct < 1) return "<1%";
    return `${Math.round(pct)}%`;
  }

  window.JobTrackerStats = {
    DEFAULT_STATUS_DEFS,
    namesOf,
    statusBreakdown,
    responseRate,
    donutSegments,
    percent,
  };
})();
