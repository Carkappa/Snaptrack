# Job Tracker

Screenshot a job posting, press a shortcut, paste. The fields get pulled out
for you to check, and the row is saved into an Excel workbook you own. A small
tray app, free and offline by default — nothing leaves your machine unless you
pick a cloud model.

## Install

**Windows** (PowerShell) — SmartScreen warns once, click *More info* →
*Run anyway*:

```powershell
irm https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.ps1 | iex
```

**macOS** — Homebrew, if you have it. Picks the right build for Apple
Silicon or Intel:

```bash
brew tap Carkappa/snaptrack https://github.com/Carkappa/Snaptrack
brew install --cask job-tracker
```

Or without Homebrew:

```bash
curl -fsSL https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.sh | bash
```

Or grab an installer from the
[latest release](https://github.com/Carkappa/Snaptrack/releases/latest). After
that new versions install themselves, waiting until you have no half-finished
entry open. Switch that off in Settings if you'd rather.

## Using it

The window opens itself the first time. After that it **lives in the tray** —
closing it only hides it. `Ctrl+Shift+J` (`Cmd+Shift+J` on macOS) opens it
from anywhere, and is rebindable in Settings.

Paste a screenshot with `Ctrl+V`, check the fields, press Enter. Rows go to
`Documents\JobApplications.xlsx`, a normal spreadsheet you can sort and edit —
it's re-read every time. Point Settings at a different folder and it offers to
take the workbook, its backups and its screenshots with it.

**Wrong guess?** Click a field, then click one of the text blocks listed under the
thumbnail to drop that text in. The app remembers where you corrected it and
starts there next time for that job board.

## Reading screenshots

Switchable in Settings. Nothing is ever invented — a field that isn't visible
comes back empty.

| | Cost | Notes |
| --- | --- | --- |
| **Tesseract** (default) | free, offline | Reads text but doesn't understand it, so which field is which is guesswork. Needs [Tesseract](https://github.com/UB-Mannheim/tesseract/wiki) on your `PATH`. |
| **[Ollama](https://ollama.com)** | free, offline | A model on your machine. Pick one from a list with sizes and hardware notes; Settings downloads it. Vision models read the screenshot directly, skipping Tesseract. Unloaded after each capture, so it holds no RAM idle. |
| **Claude, ChatGPT, Gemini** | API key | Most accurate. Each key stored separately in your OS keychain. |
| **Texas A&M AI Chat** | free with a NetID | GPT, Claude and Gemini through the university. Key from [chat.tamu.ai](https://chat.tamu.ai). |

**Which to pick:** anything that understands the page beats Tesseract, which
only reads it. A Texas A&M key is the best of both, free and accurate. Cloud
keys next, then a local vision model if you'd rather nothing left your
machine. Or use none of them and type entries in by hand.

Set a running order under **If that doesn't work** and the next choice takes
over when one fails — an expired key, a rate limit, no network.

## Also

Calendar of applications per day · overview with response rate · edit, delete
and undo · search · import another `.xlsx` · custom statuses · CSV export ·
the screenshot behind each row, openable later. Every write is backed up
first.

## If something's wrong

- **Extraction is off** — fix the fields before saving, or switch method.
  Light-mode screenshots at a readable size do best with Tesseract.
- **"It's open in Excel"** — close it and click Retry. The app never writes
  over a locked file.
- **Lost the window** — check the tray, or press the shortcut.

---

Building, tests and releases: [DEVELOPING.md](DEVELOPING.md).
