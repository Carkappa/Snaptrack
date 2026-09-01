# Job Tracker - Windows terminal installer.
#
#   irm https://raw.githubusercontent.com/Carkappa/Snaptrack/main/scripts/install.ps1 | iex
#
# Downloads the latest release installer (.exe) and runs it silently.
# The build is unsigned, so SmartScreen may still warn on first launch -
# click "More info" then "Run anyway".

$ErrorActionPreference = 'Stop'

$Repo = 'Carkappa/Snaptrack'

function Write-Info($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }
function Write-Fail($msg) { Write-Host "Error: $msg" -ForegroundColor Red; exit 1 }

Write-Info "Looking up the latest release of $Repo"
try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" `
        -Headers @{ 'User-Agent' = 'job-tracker-installer' }
} catch {
    Write-Fail "Could not reach the GitHub release API. If the repo is private, download the installer manually from https://github.com/$Repo/releases"
}

$asset = $release.assets | Where-Object { $_.name -like '*-setup.exe' } | Select-Object -First 1
if (-not $asset) {
    $asset = $release.assets | Where-Object { $_.name -like '*.msi' } | Select-Object -First 1
}
if (-not $asset) {
    Write-Fail "No .exe or .msi found in the latest release. Download it manually from https://github.com/$Repo/releases"
}

$tmp = Join-Path $env:TEMP "job-tracker-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
$installerPath = Join-Path $tmp $asset.name

try {
    Write-Info "Downloading $($asset.name)"
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $installerPath `
        -Headers @{ 'User-Agent' = 'job-tracker-installer' }

    Write-Info "Running the installer"
    if ($installerPath -like '*.msi') {
        Start-Process -Wait -FilePath 'msiexec.exe' -ArgumentList '/i', "`"$installerPath`"", '/quiet', '/norestart'
    } else {
        # Tauri's NSIS installer accepts /S for a silent install.
        Start-Process -Wait -FilePath $installerPath -ArgumentList '/S'
    }
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

Write-Host ''
Write-Info 'Done. Job Tracker is installed and will appear in your Start menu.'
Write-Host @'

  Job Tracker runs in your system tray.
    - Press Ctrl+Shift+J from anywhere to open the capture panel
    - Ctrl+V pastes a screenshot of a job posting
    - Esc hides the window; Quit from the tray menu to exit

  Screenshot extraction uses Tesseract by default (free, offline).
  Install it from https://github.com/UB-Mannheim/tesseract/wiki and make
  sure you tick "Add to PATH", or switch to the Claude API in Settings.

'@
