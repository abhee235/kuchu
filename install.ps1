# Installs Kuchu on this PC, for this account only: the latest release, checked against its SHA-256, put in
# %LOCALAPPDATA%\Programs\Kuchu, then `kuchu setup`, which opens Kuchu's Settings, where you connect it to Claude Code.
#
#   irm https://raw.githubusercontent.com/abhee235/kuchu/main/install.ps1 | iex
#
# No administrator rights, and nothing outside your own folders. Run it again to update Kuchu; uninstall.ps1 removes it.
#
# Files fetched this way are not marked as downloaded from the internet, so Windows opens Kuchu without asking. The
# SHA-256 check is what makes sure they are the release's own files.

# All in one function, called on the last lines: a script cut short on its way here runs nothing.
function Install-Kuchu {
    $ErrorActionPreference = 'Stop'
    # Windows PowerShell draws a progress bar for each download, which slows it down a great deal.
    $ProgressPreference = 'SilentlyContinue'
    # GitHub takes TLS 1.2 only, which older Windows PowerShell does not offer by itself.
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = 'https://github.com/abhee235/kuchu'
    # The PC's own CPU, even from a PowerShell running as x64 on an Arm PC.
    $arm = $env:PROCESSOR_ARCHITECTURE -eq 'ARM64' -or $env:PROCESSOR_IDENTIFIER -like 'ARM*'
    $asset = if ($arm) { 'kuchu-windows-arm64.zip' } else { 'kuchu-windows-x64.zip' }

    $work = Join-Path ([IO.Path]::GetTempPath()) ('kuchu-' + [Guid]::NewGuid())
    New-Item -ItemType Directory -Path $work | Out-Null
    try {
        $zip = Join-Path $work $asset
        $sums = Join-Path $work 'SHA256SUMS'
        Write-Host "Downloading $asset, from the latest release..."
        Invoke-WebRequest -UseBasicParsing -Uri "$repo/releases/latest/download/$asset" -OutFile $zip
        Invoke-WebRequest -UseBasicParsing -Uri "$repo/releases/latest/download/SHA256SUMS" -OutFile $sums

        $line = Get-Content $sums | Where-Object { ($_ -split '\s+')[1] -eq $asset } | Select-Object -First 1
        if (-not $line) { throw "the release's SHA256SUMS has no line for $asset." }
        $expected = ($line -split '\s+')[0].ToLowerInvariant()
        $actual = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash.ToLowerInvariant()
        if ($expected -ne $actual) { throw "$asset does not match its SHA-256 in the release, so it was deleted." }
        Write-Host "Checked: its SHA-256 is the release's."

        # With GitHub's CLI signed in, also that the release workflow built this very file.
        if (Get-Command gh -ErrorAction SilentlyContinue) {
            gh auth status *> $null
            if ($LASTEXITCODE -eq 0) {
                gh attestation verify $zip --repo abhee235/kuchu *> $null
                if ($LASTEXITCODE -ne 0) { throw "GitHub could not confirm that the release workflow built $asset." }
                Write-Host "Checked: GitHub confirms the release workflow built it."
            }
        }

        $unpacked = Join-Path $work 'kuchu'
        Expand-Archive -Path $zip -DestinationPath $unpacked -Force
        $new = Join-Path $unpacked 'kuchu.exe'
        if (-not (Test-Path $new)) { throw "$asset has no kuchu.exe in it." }

        # A Kuchu already running quits before it is replaced. (Kuchu's commands write only into a pipe.)
        & $new quit | Out-String | Write-Host
        if ($LASTEXITCODE -ne 0) {
            throw 'the Kuchu that is running did not quit. Quit it from its tray icon, then run this again.'
        }

        $dest = Join-Path $env:LOCALAPPDATA 'Programs\Kuchu'
        New-Item -ItemType Directory -Path $dest -Force | Out-Null
        Copy-Item -Path (Join-Path $unpacked '*') -Destination $dest -Recurse -Force
        Write-Host "Installed: $dest"

        & (Join-Path $dest 'kuchu.exe') setup | Out-String | Write-Host
    }
    finally {
        Remove-Item -Path $work -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Errors are said, not thrown: with `iex`, a script that exits would close the PowerShell window it runs in.
try { Install-Kuchu }
catch { Write-Host "Kuchu was not installed: $($_.Exception.Message)" -ForegroundColor Red }
