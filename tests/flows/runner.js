// Drives tests/ui-harness.html from a test page.
//
// The harness already stands in a whole Tauri bridge and boots the real
// index.html and app.js; this only adds the parts a machine needs that a
// person clicking around does not - waiting for the app to settle, and
// starting from a clean one per suite.
//
// It exists because the pure-logic pages cover calendar.js, stats.js and
// format.js, and app.js - the largest file in src/ - had nothing. Both
// bugs found by hand in that file were wiring, not logic: a command that
// was never registered, and a form that dropped a field it did not know
// about. Neither is reachable from a unit test.
(() => {
  "use strict";

  const suites = [];

  /// Registers a suite. Each one gets its own harness, so a suite that
  /// saves a row cannot change what the next suite sees.
  function suite(name, fn) {
    suites.push({ name, fn });
  }

  // ---- assertions, same vocabulary as the other test pages ----

  function eq(actual, expected, what) {
    if (actual !== expected) {
      throw new Error((what || "value") + ": expected " + JSON.stringify(expected) +
                      ", got " + JSON.stringify(actual));
    }
  }

  function ok(cond, what) {
    if (!cond) throw new Error(what || "expected truthy");
  }

  function contains(haystack, needle, what) {
    if (String(haystack).indexOf(needle) === -1) {
      throw new Error((what || "text") + ": expected it to contain " +
                      JSON.stringify(needle) + ", got " + JSON.stringify(String(haystack)));
    }
  }

  // ---- waiting ----

  // Every wait is bounded. An unbounded poll under Chrome's
  // --virtual-time-budget burns the whole budget and the page is dumped
  // mid-run, which reads as a mystery failure rather than a timeout.
  // Waits are bounded by the clock, not by a number of turns. Turns are
  // the wrong unit once the yield is a MessageChannel post: 400 of those
  // are over in a few milliseconds, long before a fetch of index.html and
  // app.js can land, so a perfectly healthy boot times out.
  const WAIT_MS = 10000;
  const MAX_TICKS = 200000;

  /// Yields one macrotask without using a timer.
  ///
  /// setTimeout is clamped to a second in a background tab, which turned a
  /// two-second bound into a six-minute one and looked exactly like a hang.
  /// A MessageChannel post is not throttled, so the same loop runs at the
  /// same speed whether the tab is in front, behind, or headless in CI.
  function tick(win) {
    return new Promise((resolve) => {
      if (typeof win.MessageChannel !== "function") {
        win.setTimeout(resolve, 0);
        return;
      }
      const channel = new win.MessageChannel();
      channel.port1.onmessage = () => {
        channel.port1.close();
        resolve();
      };
      channel.port2.postMessage(0);
    });
  }

  /// Waits until check() returns something truthy, and returns it.
  async function until(win, check, what) {
    const deadline = Date.now() + WAIT_MS;
    for (let i = 0; i < MAX_TICKS; i += 1) {
      let value = null;
      try {
        value = check();
      } catch (e) {
        value = null;
      }
      if (value) return value;
      if (Date.now() > deadline) break;
      await tick(win);
    }
    throw new Error("timed out waiting for " + (what || "a condition"));
  }

  /// Lets pending promises and timers run out. The app awaits several
  /// invokes in a row on most actions; one turn of the loop is not enough.
  async function settle(win, turns) {
    for (let i = 0; i < (turns || 6); i += 1) await tick(win);
  }

  // ---- the app under test ----

  /// Boots a fresh harness and returns a handle onto it.
  async function boot() {
    const frame = document.createElement("iframe");
    frame.width = "480";
    frame.height = "700";
    frame.style.cssText = "border:1px solid #ddd;vertical-align:top";
    // Cache-busted for the same reason the harness busts its own
    // sub-resources: a stale app.js reads as a bug in the code you just
    // changed, and has cost real debugging time here twice.
    frame.src = "ui-harness.html?flows=" + Date.now();
    document.getElementById("stage").appendChild(frame);

    await new Promise((resolve, reject) => {
      frame.onload = resolve;
      frame.onerror = () => reject(new Error("the harness page would not load"));
    });

    const win = frame.contentWindow;
    const doc = frame.contentDocument;

    await until(win, () => {
      const status = doc.getElementById("harness-status");
      if (status && /failed/.test(status.textContent)) {
        throw new Error("harness reported: " + status.textContent);
      }
      return status && status.textContent === "ready";
    }, "the harness to finish booting");

    // "ready" means app.js has been evaluated. Its own start-up still has
    // invokes in flight, so wait for the thing it draws last.
    await until(win, () => doc.querySelectorAll("tr.app-row").length > 0,
                "the applications list to render");

    return new App(frame, win, doc);
  }

  /// A handle onto one booted harness. Every helper here is something a
  /// person would do by clicking; nothing reaches into app.js internals.
  class App {
    constructor(frame, win, doc) {
      this.frame = frame;
      this.win = win;
      this.doc = doc;
      this.harness = win.__harness;
    }

    $(selector) {
      const el = this.doc.querySelector(selector);
      if (!el) throw new Error("no element matches " + selector);
      return el;
    }

    all(selector) {
      return Array.from(this.doc.querySelectorAll(selector));
    }

    byId(id) {
      const el = this.doc.getElementById(id);
      if (!el) throw new Error("no element with id " + id);
      return el;
    }

    /// Clicks and waits for whatever it started to finish.
    async click(target) {
      const el = typeof target === "string" ? this.$(target) : target;
      el.click();
      await settle(this.win);
      return el;
    }

    /// Types into a field the way a person does, so any listener on it
    /// fires. Setting .value alone is the classic way to write a passing
    /// test for a broken form.
    async type(target, value) {
      const el = typeof target === "string"
        ? (this.doc.getElementById(target) || this.$(target))
        : target;
      if (!el) throw new Error("no field " + target);
      el.value = value;
      el.dispatchEvent(new this.win.Event("input", { bubbles: true }));
      el.dispatchEvent(new this.win.Event("change", { bubbles: true }));
      await settle(this.win, 2);
      return el;
    }

    async choose(id, value) {
      const el = this.byId(id);
      el.value = value;
      el.dispatchEvent(new this.win.Event("change", { bubbles: true }));
      await settle(this.win);
      return el;
    }

    async tab(name) {
      await this.click('[data-tab="' + name + '"]');
      await settle(this.win);
    }

    /// The commands app.js has sent to the mocked backend, newest last.
    calls(name) {
      return this.harness.calls.filter((c) => c.cmd === name);
    }

    lastCall(name) {
      const matching = this.calls(name);
      if (!matching.length) throw new Error("app.js never called " + name);
      return matching[matching.length - 1];
    }

    /// Clicks, then waits for a *new* call to `name` and returns it.
    ///
    /// Waiting for "at least one" instead is the trap: earlier tests in
    /// the same suite have usually called it already, so the wait returns
    /// at once and the assertion reads a stale call. That produced two
    /// green-looking failures the first time this file ran.
    async clickAwaiting(target, name) {
      const before = this.calls(name).length;
      const el = typeof target === "string" ? this.$(target) : target;
      el.click();
      await until(this.win, () => this.calls(name).length > before,
                  "a new " + name + " call");
      await settle(this.win);
      return this.lastCall(name);
    }

    /// Answers every window.confirm from here on, and records what was
    /// asked.
    ///
    /// The app guards its destructive edits with confirm(), and a headless
    /// browser dismisses dialogs by default - so without this the "yes"
    /// path is unreachable and the button reads as doing nothing at all.
    answerConfirms(answer) {
      this.confirmed = [];
      this.win.confirm = (question) => {
        this.confirmed.push(String(question));
        return answer;
      };
    }

    text(selector) {
      return this.$(selector).textContent.trim();
    }

    async waitFor(check, what) {
      return until(this.win, check, what);
    }

    dispose() {
      this.frame.remove();
    }
  }

  // ---- runner ----

  async function run() {
    const results = [];
    const summary = document.getElementById("summary");

    for (const registered of suites) {
      let app = null;
      try {
        app = await boot();
      } catch (e) {
        results.push({
          name: registered.name + " - harness boot",
          ok: false,
          error: String((e && e.message) || e),
        });
        summary.textContent = "Running - " + results.length + " so far...";
        continue;
      }

      const tests = [];
      const test = (name, fn) => tests.push({ name, fn });
      registered.fn(test);

      for (const t of tests) {
        try {
          await t.fn(app);
          results.push({ name: registered.name + ": " + t.name, ok: true });
        } catch (e) {
          results.push({
            name: registered.name + ": " + t.name,
            ok: false,
            error: String((e && e.message) || e),
          });
        }
      }
      app.dispose();
      summary.textContent = "Running - " + results.length + " so far...";
    }

    const failed = results.filter((r) => !r.ok);
    summary.textContent = failed.length === 0
      ? "All " + results.length + " tests passed."
      : failed.length + " of " + results.length + " tests FAILED.";
    summary.className = failed.length === 0 ? "pass" : "fail";
    document.title = (failed.length === 0 ? "PASS " : "FAIL ") +
                     (results.length - failed.length) + "/" + results.length;
    document.getElementById("results").innerHTML = results
      .map((r) => '<li class="' + (r.ok ? "pass" : "fail") + '">' +
                  (r.ok ? "PASS" : "FAIL") + " - " + r.name +
                  (r.ok ? "" : "<br>&nbsp;&nbsp;&nbsp;&nbsp;" + r.error) + "</li>")
      .join("");
    window.__testResults = { total: results.length, failed: failed.length, results };
  }

  window.JobTrackerFlows = { suite, run, eq, ok, contains, settle, until };
})();
