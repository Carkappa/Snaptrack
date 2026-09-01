// Date math and aggregation for the Calendar tab.
//
// Deliberately free of any Tauri, DOM, or app-state dependency: it takes
// plain application rows (exactly what `list_applications` returns) and
// returns plain data. That keeps it exercisable on its own - see
// tests/calendar.test.html, which loads this file and nothing else.
(() => {
  "use strict";

  const MONTH_NAMES = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
  ];

  const WEEKDAY_NAMES = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

  function pad2(n) {
    return String(n).padStart(2, "0");
  }

  /// `YYYY-MM-DD` for a year/1-based-month/day triple.
  function iso(year, month, day) {
    return `${year}-${pad2(month)}-${pad2(day)}`;
  }

  /// Parses the Date Applied cell into { year, month, day }, or null if it
  /// isn't a date we recognise. The app always writes `YYYY-MM-DD`, but the
  /// workbook is a plain file the user is free to hand-edit, so the two
  /// other formats Excel commonly leaves behind are accepted too.
  function parseDate(raw) {
    if (raw == null) return null;
    const text = String(raw).trim();
    if (!text) return null;

    let year, month, day;
    let m = /^(\d{4})[-/.](\d{1,2})[-/.](\d{1,2})$/.exec(text);
    if (m) {
      year = +m[1]; month = +m[2]; day = +m[3];
    } else {
      // US-style M/D/YYYY, which is what Excel writes on a US locale.
      m = /^(\d{1,2})[-/.](\d{1,2})[-/.](\d{4})$/.exec(text);
      if (!m) return null;
      month = +m[1]; day = +m[2]; year = +m[3];
    }

    if (month < 1 || month > 12 || day < 1 || day > daysInMonth(year, month)) {
      return null;
    }
    return { year, month, day };
  }

  function daysInMonth(year, month) {
    // Day 0 of the next month is the last day of this one.
    return new Date(year, month, 0).getDate();
  }

  /// Weekday index (0 = Sunday) of the 1st of the given month.
  function firstWeekdayOfMonth(year, month) {
    return new Date(year, month - 1, 1).getDay();
  }

  function todayParts() {
    const d = new Date();
    return { year: d.getFullYear(), month: d.getMonth() + 1, day: d.getDate() };
  }

  /// Buckets applications by their Date Applied, keeping each row's index in
  /// the original array so the UI can jump straight to editing it.
  ///
  /// Returns { byDate: { "YYYY-MM-DD": [{ app, index }] }, undated: [...] }.
  function groupByDate(applications) {
    const byDate = Object.create(null);
    const undated = [];
    (applications || []).forEach((app, index) => {
      const parts = parseDate(app && app.date_applied);
      if (!parts) {
        undated.push({ app, index });
        return;
      }
      const key = iso(parts.year, parts.month, parts.day);
      (byDate[key] || (byDate[key] = [])).push({ app, index });
    });
    return { byDate, undated };
  }

  function countOn(byDate, key) {
    const entries = byDate[key];
    return entries ? entries.length : 0;
  }

  /// Buckets a day's count into a 0-4 heat level, scaled against the busiest
  /// day currently on screen so a light month still shows contrast.
  function heatLevel(count, max) {
    if (!count) return 0;
    if (!max || max <= 1) return 1;
    return Math.min(4, Math.max(1, Math.ceil((count / max) * 4)));
  }

  /// The 6x7 cell grid for a month, padded with the neighbouring months'
  /// days so every row is a full week. Always 6 rows, so the grid doesn't
  /// change height as the user pages through months.
  function monthGrid(year, month, byDate, today) {
    const ref = today || todayParts();
    const todayKey = iso(ref.year, ref.month, ref.day);

    const lead = firstWeekdayOfMonth(year, month);
    const prevMonth = month === 1 ? 12 : month - 1;
    const prevYear = month === 1 ? year - 1 : year;
    const prevDays = daysInMonth(prevYear, prevMonth);
    const thisDays = daysInMonth(year, month);
    const nextMonth = month === 12 ? 1 : month + 1;
    const nextYear = month === 12 ? year + 1 : year;

    const cells = [];
    for (let i = lead - 1; i >= 0; i--) {
      cells.push(makeCell(prevYear, prevMonth, prevDays - i, false, byDate, todayKey));
    }
    for (let day = 1; day <= thisDays; day++) {
      cells.push(makeCell(year, month, day, true, byDate, todayKey));
    }
    let nextDay = 1;
    while (cells.length < 42) {
      cells.push(makeCell(nextYear, nextMonth, nextDay++, false, byDate, todayKey));
    }

    const max = cells.reduce((acc, c) => (c.inMonth && c.count > acc ? c.count : acc), 0);
    for (const cell of cells) {
      cell.level = cell.inMonth ? heatLevel(cell.count, max) : 0;
    }

    const weeks = [];
    for (let i = 0; i < cells.length; i += 7) {
      weeks.push(cells.slice(i, i + 7));
    }
    return { weeks, cells, max };
  }

  function makeCell(year, month, day, inMonth, byDate, todayKey) {
    const key = iso(year, month, day);
    return {
      iso: key,
      year,
      month,
      day,
      inMonth,
      isToday: key === todayKey,
      count: countOn(byDate, key),
      level: 0,
    };
  }

  /// The whole year as a GitHub-style grid: one column per week, seven rows
  /// Sunday..Saturday, running from the Sunday on or before 1 January to the
  /// Saturday on or after 31 December.
  ///
  /// Days outside the year are still emitted (so every column is a full
  /// week) but flagged `inYear: false`, and like the month grid's padding
  /// they never take a heat level and never scale the others.
  function yearGrid(year, byDate, today) {
    const ref = today || todayParts();
    const todayKey = iso(ref.year, ref.month, ref.day);

    const start = new Date(year, 0, 1);
    start.setDate(start.getDate() - start.getDay());
    const end = new Date(year, 11, 31);
    end.setDate(end.getDate() + (6 - end.getDay()));

    const columns = [];
    const monthStarts = [];
    let column = [];
    const cursor = new Date(start);

    while (cursor <= end) {
      const cellYear = cursor.getFullYear();
      const cellMonth = cursor.getMonth() + 1;
      const cellDay = cursor.getDate();
      const key = iso(cellYear, cellMonth, cellDay);
      const inYear = cellYear === year;

      if (inYear && cellDay === 1) {
        monthStarts.push({ month: cellMonth, column: columns.length });
      }

      column.push({
        iso: key,
        year: cellYear,
        month: cellMonth,
        day: cellDay,
        inYear,
        isToday: key === todayKey,
        count: countOn(byDate, key),
        level: 0,
      });

      if (column.length === 7) {
        columns.push(column);
        column = [];
      }
      cursor.setDate(cursor.getDate() + 1);
    }
    if (column.length) columns.push(column);

    const cells = columns.flat();
    const max = cells.reduce((acc, c) => (c.inYear && c.count > acc ? c.count : acc), 0);
    for (const cell of cells) {
      cell.level = cell.inYear ? heatLevel(cell.count, max) : 0;
    }

    return { columns, cells, max, monthStarts };
  }

  /// Totals for a whole year, plus the busiest single day in it.
  function yearStats(year, byDate) {
    let total = 0;
    let activeDays = 0;
    let busiest = null;
    for (let month = 1; month <= 12; month++) {
      const days = daysInMonth(year, month);
      for (let day = 1; day <= days; day++) {
        const key = iso(year, month, day);
        const count = countOn(byDate, key);
        if (!count) continue;
        total += count;
        activeDays += 1;
        if (!busiest || count > busiest.count) {
          busiest = { iso: key, count };
        }
      }
    }
    return { total, activeDays, busiest };
  }

  /// The span of years the workbook actually covers, so the year view can
  /// stop the user paging into empty decades.
  function yearRange(byDate) {
    let min = null;
    let max = null;
    for (const key of Object.keys(byDate)) {
      const parts = parseDate(key);
      if (!parts) continue;
      if (min === null || parts.year < min) min = parts.year;
      if (max === null || parts.year > max) max = parts.year;
    }
    return { min, max };
  }

  /// Headline numbers for the month on screen.
  function monthStats(year, month, byDate) {
    const days = daysInMonth(year, month);
    let total = 0;
    let activeDays = 0;
    let busiest = null;
    for (let day = 1; day <= days; day++) {
      const count = countOn(byDate, iso(year, month, day));
      if (!count) continue;
      total += count;
      activeDays += 1;
      if (!busiest || count > busiest.count) {
        busiest = { iso: iso(year, month, day), day, count };
      }
    }
    return { total, activeDays, busiest };
  }

  /// Consecutive days ending today (or yesterday, so a streak isn't declared
  /// broken before the day is over) on which at least one application went out.
  function currentStreak(byDate, today) {
    const ref = today || todayParts();
    let cursor = new Date(ref.year, ref.month - 1, ref.day);
    if (countOn(byDate, iso(ref.year, ref.month, ref.day)) === 0) {
      cursor.setDate(cursor.getDate() - 1);
    }
    let streak = 0;
    while (
      countOn(byDate, iso(cursor.getFullYear(), cursor.getMonth() + 1, cursor.getDate())) > 0
    ) {
      streak += 1;
      cursor.setDate(cursor.getDate() - 1);
    }
    return streak;
  }

  /// Steps a { year, month } cursor by whole months, wrapping the year.
  function shiftMonth(year, month, delta) {
    const zero = year * 12 + (month - 1) + delta;
    return { year: Math.floor(zero / 12), month: (zero % 12) + 1 };
  }

  function monthLabel(year, month) {
    return `${MONTH_NAMES[month - 1]} ${year}`;
  }

  /// "1 September 2026" style label for a day cell.
  function dayLabel(key) {
    const parts = parseDate(key);
    if (!parts) return key;
    return `${parts.day} ${MONTH_NAMES[parts.month - 1]} ${parts.year}`;
  }

  window.JobTrackerCalendar = {
    MONTH_NAMES,
    WEEKDAY_NAMES,
    iso,
    parseDate,
    daysInMonth,
    firstWeekdayOfMonth,
    todayParts,
    groupByDate,
    countOn,
    heatLevel,
    monthGrid,
    monthStats,
    yearGrid,
    yearStats,
    yearRange,
    currentStreak,
    shiftMonth,
    monthLabel,
    dayLabel,
  };
})();
