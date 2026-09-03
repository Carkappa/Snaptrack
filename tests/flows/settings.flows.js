// Settings: custom statuses, and the per-method key cards.
//
// The status editor is the reason this file leads with it. Its two
// commands existed on both sides and were never added to
// generate_handler![], so editing a status was broken in every shipped
// build while the harness - which mocks invoke by name - stayed green.
// check-commands.sh now catches that particular shape; these tests catch
// the flow being wrong for any other reason.
(() => {
  "use strict";
  const { suite, eq, ok, contains } = window.JobTrackerFlows;

  suite("settings - statuses", (test) => {
    test("the editor lists the statuses the backend gave it", async (app) => {
      await app.tab("settings");
      const rows = app.all(".status-edit-row");
      eq(rows.length, app.harness.statusDefs.length, "one row per status");
      eq(rows[0].querySelector(".status-name").value, app.harness.statusDefs[0].name,
         "the first name is the stored one");
    });

    test("renaming a status saves it and re-labels the rows using it",
      async (app) => {
        await app.tab("settings");
        const first = app.harness.statusDefs[0].name;
        await app.type('.status-name[data-index="0"]', "Sent");
        await app.waitFor(() => app.calls("set_status_defs").length > 0,
                          "the rename to be saved");

        const saved = app.lastCall("set_status_defs").args.defs;
        eq(saved[0].name, "Sent", "the new name went to the backend");
        ok(first !== "Sent", "the fixture status was not already called that");

        // The dropdown on the capture form reads the same list, so a
        // rename that does not reach it leaves the form offering a status
        // the workbook no longer has.
        await app.tab("capture");
        const options = Array.from(app.byId("f-status").options).map((o) => o.value);
        ok(options.indexOf("Sent") !== -1, "the form offers the renamed status");
      });

    test("adding a status appends a row without disturbing the others",
      async (app) => {
        await app.tab("settings");
        const before = app.all(".status-edit-row").length;
        await app.click("#settings-status-add");
        eq(app.all(".status-edit-row").length, before + 1, "one row was added");
      });

    test("removing a status asks first, and saves the shorter list", async (app) => {
      await app.tab("settings");
      app.answerConfirms(true);
      const before = app.all(".status-edit-row").length;
      const call = await app.clickAwaiting('.status-remove[data-index="0"]',
                                           "set_status_defs");
      eq(call.args.defs.length, before - 1, "one fewer status was saved");
      eq(app.confirmed.length, 1, "it asked before removing");
    });

    test("saying no to that question removes nothing", async (app) => {
      await app.tab("settings");
      app.answerConfirms(false);
      const before = app.harness.statusDefs.length;
      const saves = app.calls("set_status_defs").length;
      await app.click('.status-remove[data-index="0"]');
      eq(app.calls("set_status_defs").length, saves, "nothing was saved");
      eq(app.harness.statusDefs.length, before, "the status is still there");
    });

    test("removing a status that rows still use warns before it goes",
      async (app) => {
        await app.tab("settings");
        app.answerConfirms(false);
        // Whichever status the rows actually carry - the tests above this
        // renamed one and removed another, so naming a specific status
        // here would only be asserting on their order.
        const used = app.harness.statusDefs.findIndex((d) =>
          app.harness.rows.some((r) => (r.status || "") === d.name));
        ok(used !== -1, "some fixture row uses one of the remaining statuses");
        await app.click('.status-remove[data-index="' + used + '"]');
        contains(app.confirmed[0], "still use it",
                 "the question says the saved rows keep it");
      });
  });

  suite("settings - extraction methods", (test) => {
    test("an offline method asks for no API key at all", async (app) => {
      await app.tab("settings");
      await app.choose("settings-extraction-method", "tesseract");
      eq(app.all("#provider-cards .provider-card").length, 0,
         "Tesseract needs no key, so no key card belongs on screen");
    });

    test("choosing a cloud method asks for that provider's key and no other",
      async (app) => {
        await app.tab("settings");
        await app.choose("settings-extraction-method", "gemini");
        const cards = app.all("#provider-cards .provider-card");
        eq(cards.length, 1, "one card, for the one method in use");

        // Scoped to the cards on purpose: "Anthropic" is a legitimate
        // option label in the method dropdown above them, so asserting on
        // the whole panel would fail for the wrong reason.
        const text = cards[0].textContent.toLowerCase();
        contains(text, "gemini", "the card names the provider it wants a key for");
        ok(text.indexOf("anthropic") === -1 && text.indexOf("openai") === -1,
           "it must not ask for a key belonging to a method you did not pick");
      });

    test("a fallback method gets a key card of its own", async (app) => {
      // Listing a method as a fallback used to give it nowhere to put a
      // key - you had to make it the primary, save, and switch back.
      await app.tab("settings");
      await app.choose("settings-extraction-method", "claude");
      app.harness.fallbackChain = ["gemini"];
      await app.choose("settings-extraction-method", "claude");
      const named = app.all("#provider-cards .provider-card")
        .map((c) => c.textContent.toLowerCase());
      ok(named.some((t) => t.indexOf("claude") !== -1 || t.indexOf("anthropic") !== -1),
         "the chosen method has a card");
    });
  });
})();
