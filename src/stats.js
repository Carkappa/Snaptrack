// Aggregation behind the summary panel at the top of the Applications tab.
//
// Like calendar.js this is deliberately free of Tauri and DOM dependencies -
// it takes the plain rows `list_applications` returns and gives back plain
// numbers, so it can be exercised on its own (tests/stats.test.html).
(() => {
  "use strict";

  /// Fixed display order, and the colour each status carries everywhere in
  /// the UI: the summary chips, the meters, and the donut are all keyed off
  /// this one list so they can never disagree.
  const STATUS_ORDER = [
    "Applied",
    "Interviewing",
    "Offered",
    "Rejected",
    "Ghosted",
    "Withdrawn",
  ];

  /// A reply of any kind - an interview, an offer, or a rejection. A
  /// rejection is still an answer; silence is what "Ghosted" and a bare
  /// "Applied" mean.
  const RESPONDED = ["Interviewing", "Offered", "Rejected"];

  function emptyCounts() {
    const counts = Object.create(null);
    for (const status of STATUS_ORDER) counts[status] = 0;
    return counts;
  }

  /// Counts by status, in display order, with each one's share of the total.
  ///
  /// A status the workbook contains but the app doesn't know about (someone
  /// typed into the cell) is kept rather than dropped, listed after the known
  /// ones, so the numbers on screen always add up to the row count.
  function statusBreakdown(applications) {
    const rows = applications || [];
    const counts = emptyCounts();
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
    const order = STATUS_ORDER.concat(
      extras.filter((s, i) => extras.indexOf(s) === i)
    );

    return {
      total,
      segments: order.map((status) => ({
        status,
        count: counts[status] || 0,
        share: total ? (counts[status] || 0) / total : 0,
        known: STATUS_ORDER.includes(status),
      })),
    };
  }

  /// How many applications got any reply, out of those that could still get
  /// one. Applications you withdrew are excluded from both sides - you ended
  /// those yourself, and counting them as silence would be misleading.
  function responseRate(applications) {
    const rows = applications || [];
    let considered = 0;
    let responded = 0;
    for (const app of rows) {
      const status = (app && app.status ? String(app.status) : "").trim() || "Applied";
      if (status === "Withdrawn") continue;
      considered += 1;
      if (RESPONDED.includes(status)) responded += 1;
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
    STATUS_ORDER,
    RESPONDED,
    statusBreakdown,
    responseRate,
    donutSegments,
    percent,
  };
})();
