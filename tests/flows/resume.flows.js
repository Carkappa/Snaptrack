// The resume tab end to end: import a file, tailor it, save it, and have
// the row remember which one was sent.
//
// Every step here crosses the invoke boundary, so this is where a command
// that exists on one side and not the other shows up.
(() => {
  "use strict";
  const { suite, eq, ok, contains } = window.JobTrackerFlows;

  suite("resume", (test) => {
    test("importing a file fills the box and stores what it read", async (app) => {
      await app.tab("resume");
      await app.click("#master-resume-import");

      eq(app.lastCall("pick_resume_file").cmd, "pick_resume_file", "it asked for a file");
      const imported = app.lastCall("import_resume_file");
      ok(imported.args.path, "it passed the chosen path through");
      contains(app.byId("master-resume").value, "Imported from", "the box shows what it read");
      eq(app.byId("master-resume").value, app.harness.masterResume, "and it was saved");
      contains(app.text("#master-resume-message"), "Imported", "it says so");
    });

    test("cancelling the picker changes nothing and says nothing", async (app) => {
      await app.tab("resume");
      await app.click("#master-resume-import");
      const kept = app.byId("master-resume").value;
      const imports = app.calls("import_resume_file").length;

      app.harness.pickResumeCancels = true;
      await app.click("#master-resume-import");

      eq(app.byId("master-resume").value, kept, "the text is untouched");
      eq(app.calls("import_resume_file").length, imports, "nothing was imported");
      eq(app.text("#master-resume-message"), "", "no stale success message is left up");
      app.harness.pickResumeCancels = false;
    });

    test("a file with no readable text reports it and keeps what was there",
      async (app) => {
        await app.tab("resume");
        await app.click("#master-resume-import");
        const kept = app.byId("master-resume").value;

        app.harness.importResumeFails = true;
        await app.click("#master-resume-import");

        contains(app.text("#master-resume-message"), "No readable text", "it explains");
        eq(app.byId("master-resume").value, kept, "the resume was not wiped");
        app.harness.importResumeFails = false;
      });

    test("tailoring for a saved application sends that job to the model",
      async (app) => {
        await app.tab("resume");
        await app.click("#master-resume-import");

        const picker = app.byId("resume-job-picker");
        const option = Array.from(picker.options).find((o) => /Stripe/.test(o.textContent));
        ok(option, "the saved applications are offered");
        await app.choose("resume-job-picker", option.value);
        await app.click("#resume-tailor");

        const call = app.lastCall("tailor_resume");
        eq(call.args.company, "Stripe", "the company came from the picked row");
        ok(app.byId("resume-result").value.length > 0, "a tailored resume came back");
      });

    test("saving against a saved application records it on the row", async (app) => {
      await app.tab("resume");
      await app.click("#master-resume-import");
      const picker = app.byId("resume-job-picker");
      const option = Array.from(picker.options).find((o) => /Stripe/.test(o.textContent));
      await app.choose("resume-job-picker", option.value);
      await app.click("#resume-tailor");
      await app.click("#resume-save-file");

      contains(app.text("#resume-save-message"), "recorded against", "it says it linked");
      const row = app.harness.rows.find((r) => r.company === "Stripe");
      ok(row.resume, "the workbook row carries the path");

      // The list is read again rather than patched, so the link has to be
      // there without anyone pressing refresh.
      await app.tab("list");
      const linked = app.all(".row-resume").map((a) => a.dataset.index);
      const index = app.harness.rows.indexOf(row);
      ok(linked.indexOf(String(index)) !== -1, "the list shows the new link");
    });

    test("a pasted posting saves a PDF without claiming it linked", async (app) => {
      await app.tab("resume");
      await app.click("#master-resume-import");
      await app.choose("resume-job-picker", "");
      await app.type("resume-job-text", "Acme Rockets, Propulsion Engineer, Reno NV.");
      await app.click("#resume-tailor");
      await app.click("#resume-save-file");

      const message = app.text("#resume-save-message");
      contains(message, "Saved", "it saved");
      ok(message.indexOf("recorded against") === -1,
         "it must not claim a row it never matched: " + message);
    });

    test("tailoring with no resume saved refuses instead of inventing one",
      async (app) => {
        // Tests in a suite share one harness, and the ones above this
        // imported a resume. Put it back to empty rather than assuming.
        app.harness.masterResume = "";
        await app.tab("resume");
        app.byId("master-resume").value = "";
        await app.click("#resume-tailor");
        contains(app.text("#resume-tailor-message"), "resume first", "it explains why");
      });
  });
})();
