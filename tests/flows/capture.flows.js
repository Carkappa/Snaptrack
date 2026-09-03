// Capture: typing an application in by hand, and what the app does when
// the one you are saving is already in the workbook.
//
// The duplicate banner is the interesting part. It is the only place the
// app refuses to do what you asked and offers two other things instead,
// and both of those write to the workbook.
(() => {
  "use strict";
  const { suite, eq, ok, contains } = window.JobTrackerFlows;

  async function fillNewApplication(app, company, position) {
    await app.tab("capture");
    await app.type("f-company", company);
    await app.type("f-position", position);
    await app.type("f-date-applied", "2026-09-02");
  }

  suite("capture", (test) => {
    test("a typed application is saved and appears in the list", async (app) => {
      const before = app.harness.rows.length;
      await fillNewApplication(app, "Hypertext Ltd", "Docs Engineer");
      await app.click("#form-save");

      const saved = app.lastCall("save_application").args.application;
      eq(saved.company, "Hypertext Ltd", "the company was sent");
      eq(saved.position, "Docs Engineer", "the position was sent");

      await app.tab("list");
      eq(app.harness.rows.length, before + 1, "the workbook grew by one");
      const companies = app.all("tr.app-row").map((r) => r.children[1].textContent);
      ok(companies.indexOf("Hypertext Ltd") !== -1, "the new row is listed");
    });

    test("saving without a company is refused rather than writing a blank row",
      async (app) => {
        const before = app.harness.rows.length;
        await app.tab("capture");
        await app.type("f-position", "Nameless Role");
        await app.click("#form-save");
        eq(app.harness.rows.length, before, "nothing was written");
      });

    test("a duplicate is caught and offers the two ways out", async (app) => {
      const existing = app.harness.rows[0];
      await fillNewApplication(app, existing.company, existing.position);
      await app.click("#form-save");

      ok(!app.byId("duplicate-banner").hidden, "the duplicate banner is shown");
      contains(app.text("#duplicate-status"), existing.status,
               "it says what the existing row's status is");
      ok(!app.byId("duplicate-save-anyway").hidden, "save anyway is offered");
      ok(!app.byId("duplicate-update-status").hidden, "updating the status is offered");
    });

    test("save anyway writes the second row", async (app) => {
      const existing = app.harness.rows[0];
      const before = app.harness.rows.length;
      await fillNewApplication(app, existing.company, existing.position);
      await app.click("#form-save");
      await app.click("#duplicate-save-anyway");
      eq(app.harness.rows.length, before + 1, "the duplicate was written on purpose");
    });

    test("updating the status instead touches the existing row, not a new one",
      async (app) => {
        const existing = app.harness.rows[0];
        const before = app.harness.rows.length;
        await fillNewApplication(app, existing.company, existing.position);
        await app.choose("f-status", "Interviewing");
        await app.click("#form-save");
        await app.click("#duplicate-update-status");

        eq(app.harness.rows.length, before, "no row was added");
        eq(app.harness.rows[0].status, "Interviewing", "the existing row moved on");
      });
  });
})();
