# Job Tracker

Screenshot a job posting, press a shortcut, paste. The company, role,
location and the rest get pulled out for you to check, and the row is
saved into an Excel workbook you own.

A small desktop app that lives in your system tray. Free and offline by
default — nothing is sent anywhere unless you choose a cloud model.

## Install

**Windows** — in PowerShell:

```powershell
irm https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.ps1 | iex
```

**macOS** (Apple Silicon):

```bash
curl -fsSL https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.sh | bash
```

Or download an installer from the
[latest release](https://github.com/Carkappa/Snaptrack/releases/latest)
and run it.

The build isn't code-signed, so **Windows SmartScreen will warn you once** —
click *More info* → *Run anyway*. On macOS the script clears the Gatekeeper
flag for you.

## First run

The window opens by itself the first time. After that **Job Tracker lives in
your system tray, not the taskbar** — closing the window hides it, it doesn't
quit. Use the tray icon, or the global shortcut, to bring it back.

`Ctrl+Shift+J` (`Cmd+Shift+J` on macOS) opens it from anywhere. You can change
that in Settings.

## Using it

1. Apply for a job, and screenshot the posting.
2. Press the shortcut.
3. Press `Ctrl+V` to paste the screenshot — or drag a file in, choose one, or
   click **Skip screenshot** to type an entry by hand.
4. Check the fields it filled in and fix anything wrong.
5. Press Enter to save.

The row is written to `Documents\JobApplications.xlsx`. It's a normal
spreadsheet — open it, sort it, edit it, back it up. The app reads it fresh
every time, so your changes are respected.

## Reading screenshots

Four ways, switchable in Settings at any time:

**Tesseract** — the default. Free, runs entirely on your machine, nothing
leaves it. Less accurate than a model, so the full recognised text is always
attached to the Notes field for you to check. Needs
[Tesseract installed](https://github.com/UB-Mannheim/tesseract/wiki) — on
Windows, tick **Add to PATH** during setup.

**A local model via [Ollama](https://ollama.com)** — free, offline, no key.
Tesseract reads the screenshot, then a model on your own machine works out
which words are the company and which are the title. That's language rather
than layout, so it copes with job boards no layout rule was written for.
Needs Tesseract and Ollama installed, plus a pulled model:

```bash
ollama pull qwen2.5:3b
```

A 3B model runs on a CPU in a second or two. Settings shows whether Ollama is
reachable and whether the model is pulled.

**Claude, ChatGPT or Gemini** — meaningfully more accurate, because they read
the page the way you would. Pick one in Settings and paste in an API key for
it; each provider's key is stored separately in your OS keychain. There's also
a Model field if a provider retires the default.

Either way, nothing is invented: a field that isn't visible in the screenshot
comes back empty for you to fill in.

You can also use neither and type entries by hand.

### When it guesses wrong

Two things make that cheap rather than annoying:

**Click a block to fill a field.** With Tesseract, the text blocks it found
are listed under the thumbnail. Click a field, click a block, and that text
goes in. Layout guesswork will always lose on some site's design; this works
on any layout, including ones nobody tuned for.

**It learns from your corrections.** When you save, the app notes where on the
page the values you kept actually sat, keyed to the job board it recognised.
The next capture from that board starts from your correction instead of the
guess. Nothing to configure — the corrections you were already making are the
signal.

## What else it does

- **Calendar** — a month grid shaded by how many applications went out each
  day, plus a year view. Click a day to see what you sent.
- **Overview** — where every application stands, your response rate, and how
  many are still waiting. Click a status to filter the list to it.
- **Edit, delete, undo** — click a row to edit it, `×` to delete it, and Undo
  in the toast to put it back. Your workbook is backed up before every write.
- **Open screenshot** — the capture a row came from is archived, and openable
  from the edit form, which helps once the listing is taken down.
- **Search** across company, role, location, job ID and notes.
- **Import** another `.xlsx`, skipping anything you already track.
- **Custom statuses** — rename, add or remove them in Settings.
- **CSV export**, one click.

## Updates

New versions install themselves. The app checks once when it starts, at most
once a day, and waits until you have no unsaved entry open before restarting.
Both the check and the automatic install can be switched off in Settings, and
**Check now** is there when you want it.

## If something's wrong

**"Tesseract not found"** — install it and make sure it's on your `PATH`, or
switch to a cloud model, or just type entries by hand.

**Extraction got it wrong** — that happens some of the time; fix the fields
before saving. Tesseract does noticeably better on a light-mode screenshot at
a readable size. A cloud model does better still.

**"It's open in Excel"** — close the workbook and click Retry. The app never
writes over a file another program has locked.

**Can't find the window** — look in the system tray, or press the shortcut.

---

Building it, running the tests, and cutting a release are in
[DEVELOPING.md](DEVELOPING.md).
