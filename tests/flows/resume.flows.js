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
        ok(app.all("#resume-preview .resume-bullet").length > 0,
           "a tailored resume was rendered into the sheet");
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

    test("an edit in the sheet is what gets saved, not what the model wrote",
      async (app) => {
        await app.tab("resume");
        await app.click("#master-resume-import");
        await app.choose("resume-job-picker", "");
        await app.type("resume-job-text", "Acme Rockets, Propulsion Engineer.");
        await app.click("#resume-tailor");

        const bullet = app.$("#resume-preview .resume-bullet");
        bullet.textContent = "Rewrote this line by hand";
        bullet.dispatchEvent(new app.win.Event("input", { bubbles: true }));
        await app.click("#resume-save-file");

        const sent = app.lastCall("save_tailored_resume").args.resume;
        const bullets = sent.sections.flatMap((s) => s.entries.flatMap((e) => e.bullets));
        ok(bullets.indexOf("Rewrote this line by hand") !== -1,
           "the edited line reached the backend: " + JSON.stringify(bullets));
      });

    test("cutting a line removes it from what is sent", async (app) => {
      await app.tab("resume");
      await app.click("#master-resume-import");
      await app.choose("resume-job-picker", "");
      await app.type("resume-job-text", "Acme Rockets, Propulsion Engineer.");
      await app.click("#resume-tailor");

      const before = app.all("#resume-preview .resume-bullet").length;
      ok(before > 0, "there is a bullet to cut");
      const doomed = app.$("#resume-preview .resume-bullet").textContent;
      await app.click('#resume-preview .resume-cut[data-cut="bullet"]');

      eq(app.all("#resume-preview .resume-bullet").length, before - 1, "the line went");
      await app.click("#resume-save-file");
      const sent = app.lastCall("save_tailored_resume").args.resume;
      const bullets = sent.sections.flatMap((s) => s.entries.flatMap((e) => e.bullets));
      ok(bullets.indexOf(doomed) === -1, "the cut line is not in the saved resume");
    });

    test("undoing edits puts the model's version back", async (app) => {
      await app.tab("resume");
      await app.click("#master-resume-import");
      await app.choose("resume-job-picker", "");
      await app.type("resume-job-text", "Acme Rockets, Propulsion Engineer.");
      await app.click("#resume-tailor");

      const original = app.all("#resume-preview .resume-bullet").length;
      ok(app.byId("resume-revert").hidden, "nothing to undo before an edit");
      await app.click('#resume-preview .resume-cut[data-cut="bullet"]');
      ok(!app.byId("resume-revert").hidden, "undo is offered once you have edited");

      await app.click("#resume-revert");
      eq(app.all("#resume-preview .resume-bullet").length, original, "the line came back");
      ok(app.byId("resume-revert").hidden, "and there is nothing left to undo");
    });

    test("the length readout tracks what is actually on the sheet", async (app) => {
      await app.tab("resume");
      await app.click("#master-resume-import");
      await app.choose("resume-job-picker", "");
      await app.type("resume-job-text", "Acme Rockets, Propulsion Engineer.");
      await app.click("#resume-tailor");

      const readout = () => app.text("#resume-length-label");
      contains(readout(), "lines", "it reports a length");
      const before = readout();
      await app.click('#resume-preview .resume-cut[data-cut="entry"]');
      ok(readout() !== before, "cutting a whole entry changes the count: " + readout());
    });

    test("importing a .tex keeps its style and says which document", async (app) => {
      await app.tab("resume");
      ok(app.byId("resume-template-card").hidden, "no style before importing one");

      app.harness.pickResumeLatex = true;
      await app.click("#master-resume-import");

      ok(!app.byId("resume-template-card").hidden, "the style card is shown");
      const card = app.text("#resume-template-card");
      contains(card, "your own style", "it says the style is being kept");
      contains(card, "article", "and names the document class it read");
      contains(app.text("#master-resume-message"), "LaTeX style was kept", "the message says so");
      app.harness.pickResumeLatex = false;
    });

    test("importing a plain file leaves no style behind", async (app) => {
      await app.tab("resume");
      app.harness.resumeTemplate = null;
      await app.click("#master-resume-import");
      ok(app.byId("resume-template-card").hidden,
         "a PDF has no style to keep, so nothing should claim one");
    });

    test("removing the style asks first and goes back to the built-in one",
      async (app) => {
        await app.tab("resume");
        app.harness.pickResumeLatex = true;
        await app.click("#master-resume-import");
        app.harness.pickResumeLatex = false;

        app.answerConfirms(false);
        await app.click("#resume-template-clear");
        ok(!app.byId("resume-template-card").hidden, "saying no keeps the style");

        app.answerConfirms(true);
        await app.click("#resume-template-clear");
        ok(app.byId("resume-template-card").hidden, "saying yes removes it");
        eq(app.harness.resumeTemplate, null, "and the backend forgot it");
      });

    test("a style that could not be used is reported, not hidden", async (app) => {
      await app.tab("resume");
      app.harness.pickResumeLatex = true;
      await app.click("#master-resume-import");
      app.harness.pickResumeLatex = false;
      app.harness.templateTypesetFails = true;

      await app.choose("resume-job-picker", "");
      await app.type("resume-job-text", "Acme Rockets, Propulsion Engineer.");
      await app.click("#resume-tailor");
      await app.click("#resume-save-file");

      const message = app.text("#resume-save-message");
      contains(message, "Saved", "it still saved something");
      contains(message, "not used", "and said the style could not be used");
      app.harness.templateTypesetFails = false;
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
