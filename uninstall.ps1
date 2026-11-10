# Uninstalls Kuchu from this PC when it was installed with install.ps1 or unzipped into %LOCALAPPDATA%\Programs\Kuchu:
# `kuchu uninstall` (it no longer starts when you sign in, and its relay, settings and data go), then the program.
#
#   irm https://raw.githubusercontent.com/abhee235/kuchu/main/uninstall.ps1 | iex
#
# Installed from the Microsoft Store or with the Windows installer, uninstall Kuchu from Windows' Settings, Apps.
#
# Kuchu changes Claude Code's settings only on your click. If it is still connected, its Settings open: click
# Disconnect there, then run this again. The copies Kuchu kept of Claude Code's settings.json stay, for you.

# All in one function, called on the last lines: a script cut short on its way here runs nothing.
function Uninstall-Kuchu {
    $ErrorActionPreference = 'Stop'
    $dest = Join-Path $env:LOCALAPPDATA 'Programs\Kuchu'
    $kuchu = Join-Path $dest 'kuchu.exe'
    if (-not (Test-Path $kuchu)) {
        throw ("there is no kuchu.exe in $dest. Installed from the Microsoft Store or with the Windows installer, " +
            "uninstall Kuchu from Windows' Settings, Apps.")
    }
    # The Windows installer leaves its own uninstaller beside Kuchu: that copy goes through Windows' Settings, Apps.
    if (Test-Path (Join-Path $dest 'uninstall.exe')) {
        throw "this Kuchu came with the Windows installer: uninstall it from Windows' Settings, Apps."
    }

    # Kuchu's commands write only into a pipe.
    & $kuchu uninstall | Out-String | Write-Host
    if ($LASTEXITCODE -ne 0) {
        Write-Host 'When that is done, run this command again.'
        return
    }

    Remove-Item -Path $dest -Recurse -Force
    Write-Host "Deleted: $dest"
}

# Errors are said, not thrown: with `iex`, a script that exits would close the PowerShell window it runs in.
try { Uninstall-Kuchu }
catch { Write-Host "Kuchu was not uninstalled: $($_.Exception.Message)" -ForegroundColor Red }
