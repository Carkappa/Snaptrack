// The applications list: what it renders, and what an edit preserves.
//
// The row index is the trap this file exists for. The list sorts and
// filters for display, but data-index stays the position in the workbook,
// because that is what every write command addresses. A test that never
// sorts would pass with the two confused.
(() => {
  "use strict";
  const { suite, eq, ok, contains } = window.JobTrackerFlows;

  suite("applications", (test) => {
    test("every fixture row is listed, with a cell per column", async (app) => {
      const headers = app.all("thead th").length;
      const rows = app.all("tr.app-row");
      eq(rows.length, app.harness.rows.length, "one row per application");
      eq(rows[0].children.length, headers, "cells match the header count");
    });

    test("the resume column links only the rows that have one", async (app) => {
      const withResume = app.harness.rows.filter((r) => r.resume).length;
      eq(app.all(".row-resume").length, withResume, "resume links");
      ok(withResume > 0, "the fixtures need at least one resume to test with");
    });

    test("a resume link opens that row's file, not the one under it", async (app) => {
      // Sort by company so display order stops matching workbook order.
      // With the index taken from the display position instead, this is
      // the assertion that fails.
      await app.click(app.all("thead th")[1]);
      const link = app.$(".row-resume");
      const company = link.closest("tr").children[1].textContent;
      await app.click(link);
      const opened = app.harness.openedResumes;
      eq(opened.length, 1, "one file opened");
      contains(opened[0], company.replace(/\s+/g, "-"), "it opened that row's resume");
    });

    test("editing a row keeps the fields the form does not show", async (app) => {
      // The resume path is on the row but has no form field. An edit
      // rebuilds the whole application object, so anything the form does
      // not carry is silently dropped - this shipped once already.
      const index = app.harness.rows.findIndex((r) => r.resume);
      const before = app.harness.rows[index].resume;
      const row = app.all("tr.app-row").find((r) => r.dataset.index === String(index));
      ok(row, "the row with a resume is in the list");

      await app.click(row);
      ok(!app.byId("editing-banner").hidden, "the form is in edit mode");
      await app.type("f-location", "Somewhere Else, TX");
      await app.click("#form-save");

      const sent = app.lastCall("update_application_at_index").args.application;
      eq(sent.location, "Somewhere Else, TX", "the edit was applied");
      eq(sent.resume, before, "the resume path survived the round trip");
    });

    test("deleting a row asks first and names the row it would delete",
      async (app) => {
        await app.click(app.all("thead th")[1]); // sort, so index != position
        app.answerConfirms(false);
        const row = app.all("tr.app-row")[0];
        const company = row.children[1].textContent;
        await app.click(row.querySelector(".row-delete"));

        eq(app.calls("delete_application_at_index").length, 0, "nothing was deleted");
        contains(app.confirmed[0], company, "the question names the row you clicked");
      });

    test("deleting sends the workbook position, not the display position",
      async (app) => {
        // The delete command re-checks the company and position at that
        // index and refuses if they do not match, so a wrong index here
        // surfaces as a thrown error rather than a lost row.
        await app.click(app.all("thead th")[1]);
        app.answerConfirms(true);
        const row = app.all("tr.app-row")[0];
        const index = Number(row.dataset.index);
        const expected = app.harness.rows[index];
        const call = await app.clickAwaiting(row.querySelector(".row-delete"),
                                             "delete_application_at_index");
        eq(call.args.index, index, "the workbook index was sent");
        eq(call.args.expectedCompany, expected.company, "with the row it expected to find");
      });

    test("searching narrows the list without renumbering the rows", async (app) => {
      const target = app.harness.rows[3];
      await app.type("search-box", target.company);
      const rows = app.all("tr.app-row");
      ok(rows.length < app.harness.rows.length, "the list narrowed");
      eq(rows[0].dataset.index, "3", "the surviving row kept its workbook index");
    });

    test("a search that matches nothing says so rather than emptying silently",
      async (app) => {
        await app.type("search-box", "zzz-no-such-company");
        eq(app.all("tr.app-row").length, 0, "no rows");
        contains(app.text("#list-status"), "No matches", "the status line explains");
      });
  });
})();
