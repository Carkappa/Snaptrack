# Job Tracker

Screenshot a job posting, press a shortcut, paste. The fields get pulled out
for you to check, and the row is saved into an Excel workbook you own.

A small tray app. Free and offline by default — nothing leaves your machine
unless you pick a cloud model.

## Install

**Windows** (PowerShell) — SmartScreen warns once, click *More info* →
*Run anyway*:

```powershell
irm https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.ps1 | iex
```

**macOS** (Apple Silicon):

```bash
curl -fsSL https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.sh | bash
```

Or grab an installer from the
[latest release](https://github.com/Carkappa/Snaptrack/releases/latest).
New versions install themselves after that.

## Using it

`Ctrl+Shift+J` (`Cmd+Shift+J` on macOS) opens it from anywhere — rebindable in
Settings. It **lives in the tray**: closing the window hides it.

Paste a screenshot with `Ctrl+V`, check the fields, press Enter. Rows go to
`Documents\JobApplications.xlsx`, a normal spreadsheet you can sort and edit —
it's re-read every time. Point Settings at a different folder and it offers to
take the workbook, its backups and its screenshots with it.

Wrong guess? Click a field, then click one of the OCR blocks listed under the
thumbnail to drop that text in. The app remembers where you corrected it and
starts there next time for that job board.

## Reading screenshots

Switchable in Settings. Nothing is ever invented — a field that isn't visible
comes back empty.

| | Cost | Notes |
| --- | --- | --- |
| **Tesseract** (default) | free, offline | Needs [Tesseract](https://github.com/UB-Mannheim/tesseract/wiki) on your `PATH`. Raw text goes to Notes to check against. |
| **[Ollama](https://ollama.com)** | free, offline | A model on your own machine. Pick from a list with sizes and hardware notes; Settings downloads it for you. **Vision models read the screenshot directly**, skipping Tesseract entirely. |
| **Claude, ChatGPT, Gemini** | API key | Most accurate. Each key stored separately in your OS keychain. |

Or use none of them and type entries by hand.

## Also

Calendar of applications per day · overview with response rate · edit, delete
and undo · search · import another `.xlsx` · custom statuses · CSV export ·
the screenshot behind each row, openable later. Every write is backed up
first.

## If something's wrong

- **Extraction is off** — fix the fields before saving. Light-mode
  screenshots at a readable size do best.
- **"It's open in Excel"** — close it and click Retry. The app never writes
  over a locked file.
- **Lost the window** — check the tray, or press the shortcut.

---

Building, tests and releases: [DEVELOPING.md](DEVELOPING.md).
